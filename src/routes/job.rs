use rocket::serde::json::Json;
use rocket::State;
use rocket::form::FromForm;
use rocket_okapi::openapi;
use mongodb::bson::{doc, DateTime};
use mongodb::options::FindOptions;
use crate::db::DbConn;
use crate::models::{Subscription, JobSeekerSubscriptionPlan, SubscriptionType, SubscriptionStatus, JobSeekerProfile, CreateJobSeekerProfileDto, UpdateJobSeekerProfileDto};
use crate::guards::{AuthGuard, KycGuard};
use crate::utils::{ApiResponse, ApiError};
use crate::services::RazorpayService;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use mongodb::bson::oid::ObjectId;

// ============================================================================
// JOB SEEKER SUBSCRIPTION ENDPOINTS
// ============================================================================

#[openapi(tag = "JobSeekerSubscription")]
#[post("/job-seeker/subscription/create/<plan_name>")]
pub async fn create_job_seeker_subscription(
    db: &State<DbConn>,
    auth: AuthGuard,
    plan_name: String,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    // Validate plan and get price
    let (price, plan_type) = match plan_name.to_lowercase().as_str() {
        "basic" => (0.5, JobSeekerSubscriptionPlan::Basic),
        "premium" => (1.5, JobSeekerSubscriptionPlan::Premium),
        _ => return Err(ApiError::bad_request("Invalid plan. Choose 'basic' or 'premium'")),
    };

    let now = DateTime::now();
    let expires_at = DateTime::from_millis(
        chrono::Utc::now().timestamp_millis() + 365 * 24 * 60 * 60 * 1000, // 1 year
    );

    // Check if user already has an active subscription
    let existing = db
        .collection::<Subscription>("subscriptions")
        .find_one(
            doc! {
                "user_id": auth.user_id,
                "subscription_type": "jobseeker",
                "status": "active"
            },
            None,
        )
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;

    if existing.is_some() {
        return Err(ApiError::bad_request("You already have an active job seeker subscription"));
    }

    // Create Razorpay order
    let order = RazorpayService::create_order(price as i64)
        .await
        .map_err(|e| ApiError::internal_error(format!("Failed to create payment order: {}", e)))?;

    // Create subscription with cancelled status (pending payment)
    let subscription = Subscription {
        id: None,
        user_id: auth.user_id,
        subscription_type: SubscriptionType::JobSeeker,
        plan_name: plan_name.clone(),
        price,
        status: SubscriptionStatus::Cancelled,
        starts_at: now,
        expires_at,
        auto_renew: false,
        payment_id: None,
        created_at: now,
        updated_at: now,
    };

    let sub_res = db
        .collection::<Subscription>("subscriptions")
        .insert_one(&subscription, None)
        .await
        .map_err(|e| ApiError::internal_error(format!("Failed to create subscription: {}", e)))?;

    let subscription_id = sub_res
        .inserted_id
        .as_object_id()
        .ok_or_else(|| ApiError::internal_error("Invalid subscription ID"))?
        .to_hex();

    Ok(Json(ApiResponse::success(serde_json::json!({
        "subscription_id": subscription_id,
        "order": order,
        "plan_name": plan_name,
        "price": price
    }))))
}

#[derive(serde::Deserialize, rocket_okapi::okapi::schemars::JsonSchema)]
pub struct VerifyJobSeekerPaymentDto {
    pub subscription_id: String,
    pub razorpay_order_id: String,
    pub razorpay_payment_id: String,
    pub razorpay_signature: String,
}

#[openapi(tag = "JobSeekerSubscription")]
#[post("/job-seeker/subscription/verify", data = "<dto>")]
pub async fn verify_job_seeker_payment(
    db: &State<DbConn>,
    auth: AuthGuard,
    dto: Json<VerifyJobSeekerPaymentDto>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    // Verify Razorpay signature
    let secret = std::env::var("RAZORPAY_KEY_SECRET")
        .map_err(|_| ApiError::internal_error("Missing Razorpay secret"))?;

    let payload = format!("{}|{}", dto.razorpay_order_id, dto.razorpay_payment_id);

    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes())
        .map_err(|_| ApiError::internal_error("Invalid HMAC key"))?;

    mac.update(payload.as_bytes());
    let expected_signature = hex::encode(mac.finalize().into_bytes());

    if expected_signature != dto.razorpay_signature {
        return Err(ApiError::bad_request("Invalid payment signature"));
    }

    // Update subscription status
    let sub_id = ObjectId::parse_str(&dto.subscription_id)
        .map_err(|_| ApiError::bad_request("Invalid subscription ID"))?;

    let result = db
        .collection::<Subscription>("subscriptions")
        .update_one(
            doc! {
                "_id": sub_id,
                "user_id": auth.user_id,
                "subscription_type": "jobseeker"
            },
            doc! {
                "$set": {
                    "status": "active",
                    "payment_id": &dto.razorpay_payment_id,
                    "updated_at": DateTime::now()
                }
            },
            None,
        )
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;

    if result.matched_count == 0 {
        return Err(ApiError::not_found("Subscription not found"));
    }

    // Get the subscription details
    let subscription = db
        .collection::<Subscription>("subscriptions")
        .find_one(doc! { "_id": sub_id }, None)
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?
        .ok_or_else(|| ApiError::not_found("Subscription not found"))?;

    Ok(Json(ApiResponse::success(serde_json::json!({
        "message": "Payment verified successfully",
        "subscription": {
            "id": subscription.id.unwrap().to_hex(),
            "plan_name": subscription.plan_name,
            "status": "active",
            "expires_at": subscription.expires_at
        }
    }))))
}

#[openapi(tag = "JobSeekerSubscription")]
#[get("/job-seeker/subscription/status")]
pub async fn get_job_seeker_subscription_status(
    db: &State<DbConn>,
    auth: AuthGuard,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let subscription = db
        .collection::<Subscription>("subscriptions")
        .find_one(
            doc! {
                "user_id": auth.user_id,
                "subscription_type": "jobseeker",
                "status": "active"
            },
            None,
        )
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;

    if let Some(sub) = subscription {
        Ok(Json(ApiResponse::success(serde_json::json!({
            "has_subscription": true,
            "subscription": {
                "id": sub.id.unwrap().to_hex(),
                "plan_name": sub.plan_name,
                "status": format!("{:?}", sub.status),
                "expires_at": sub.expires_at,
                "auto_renew": sub.auto_renew
            }
        }))))
    } else {
        Ok(Json(ApiResponse::success(serde_json::json!({
            "has_subscription": false,
            "subscription": null
        }))))
    }
}

// ============================================================================
// JOB SEEKER PROFILE ENDPOINTS
// ============================================================================

#[openapi(tag = "JobSeeker")]
#[post("/job-seeker/profile", data = "<dto>")]
pub async fn create_job_seeker_profile(
    db: &State<DbConn>,
    kyc_guard: KycGuard,
    dto: Json<CreateJobSeekerProfileDto>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let auth = kyc_guard.auth;

    // Check if user has active subscription
    let has_subscription = db
        .collection::<Subscription>("subscriptions")
        .find_one(
            doc! {
                "user_id": auth.user_id,
                "subscription_type": "jobseeker",
                "status": "active"
            },
            None,
        )
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;

    if has_subscription.is_none() {
        return Err(ApiError::bad_request(
            "Active job seeker subscription required to create profile",
        ));
    }

    let subscription = has_subscription.unwrap();
    let subscription_plan = match subscription.plan_name.as_str() {
        "basic" => JobSeekerSubscriptionPlan::Basic,
        "premium" => JobSeekerSubscriptionPlan::Premium,
        _ => JobSeekerSubscriptionPlan::None,
    };

    // Check if profile already exists
    let existing = db
        .collection::<JobSeekerProfile>("job_seeker_profiles")
        .find_one(doc! { "user_id": auth.user_id }, None)
        .await
        .map_err(|e| ApiError::internal_error(format!("Database error: {}", e)))?;

    if existing.is_some() {
        return Err(ApiError::bad_request("Job seeker profile already exists"));
    }

    // Validate required fields
    if dto.full_name.trim().is_empty() {
        return Err(ApiError::bad_request("Full name is required"));
    }

    if dto.skills.is_empty() {
        return Err(ApiError::bad_request("At least one skill is required"));
    }

    // Create job seeker profile
    let profile = JobSeekerProfile {
        id: None,
        user_id: auth.user_id,
        full_name: dto.full_name.clone(),
        headline: dto.headline.clone(),
        bio: dto.bio.clone(),
        skills: dto.skills.clone(),
        experience_years: dto.experience_years,
        education: dto.education.clone(),
        work_experience: dto.work_experience.clone(),
        preferred_categories: dto.preferred_categories.clone(),
        preferred_job_types: dto.preferred_job_types.clone(),
        preferred_locations: dto.preferred_locations.clone(),
        expected_salary_min: dto.expected_salary_min,
        expected_salary_max: dto.expected_salary_max,
        willing_to_relocate: dto.willing_to_relocate,
        resume_url: dto.resume_url.clone(),
        portfolio_url: dto.portfolio_url.clone(),
        linkedin_url: dto.linkedin_url.clone(),
        subscription_plan,
        subscription_expires_at: Some(subscription.expires_at),
        is_verified: false,
        is_available: true,
        profile_views: 0,
        applications_count: 0,
        created_at: DateTime::now(),
        updated_at: DateTime::now(),
    };

    let result = db
        .collection::<JobSeekerProfile>("job_seeker_profiles")
        .insert_one(&profile, None)
        .await
        .map_err(|e| ApiError::internal_error(format!("Failed to create profile: {}", e)))?;

    Ok(Json(ApiResponse::success_with_message(
        "Job seeker profile created successfully".to_string(),
        serde_json::json!({
            "profile_id": result.inserted_id.as_object_id().unwrap().to_hex()
        }),
    )))
}

#[openapi(tag = "JobSeeker")]
#[get("/job-seeker/profile")]
pub async fn get_job_seeker_profile(
    db: &State<DbConn>,
    auth: AuthGuard,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let profile = db
        .collection::<JobSeekerProfile>("job_seeker_profiles")
        .find_one(doc! { "user_id": auth.user_id }, None)
        .await
        .map_err(|e| ApiError::internal_error(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Job seeker profile not found"))?;

    Ok(Json(ApiResponse::success(serde_json::json!(profile))))
}

#[openapi(tag = "JobSeeker")]
#[get("/job-seeker/profile/<profile_id>")]
pub async fn get_job_seeker_profile_by_id(
    db: &State<DbConn>,
    profile_id: String,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let object_id = ObjectId::parse_str(&profile_id)
        .map_err(|_| ApiError::bad_request("Invalid profile ID"))?;

    // Increment profile views
    db.collection::<JobSeekerProfile>("job_seeker_profiles")
        .update_one(
            doc! { "_id": object_id },
            doc! { "$inc": { "profile_views": 1 } },
            None,
        )
        .await
        .ok();

    let profile = db
        .collection::<JobSeekerProfile>("job_seeker_profiles")
        .find_one(doc! { "_id": object_id }, None)
        .await
        .map_err(|e| ApiError::internal_error(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Job seeker profile not found"))?;

    Ok(Json(ApiResponse::success(serde_json::json!(profile))))
}

#[openapi(tag = "JobSeeker")]
#[put("/job-seeker/profile", data = "<dto>")]
pub async fn update_job_seeker_profile(
    db: &State<DbConn>,
    auth: AuthGuard,
    dto: Json<UpdateJobSeekerProfileDto>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let mut update_doc = doc! {
        "updated_at": DateTime::now()
    };

    if let Some(ref full_name) = dto.full_name {
        if full_name.trim().is_empty() {
            return Err(ApiError::bad_request("Full name cannot be empty"));
        }
        update_doc.insert("full_name", full_name);
    }
    if let Some(ref headline) = dto.headline {
        update_doc.insert("headline", headline);
    }
    if let Some(ref bio) = dto.bio {
        update_doc.insert("bio", bio);
    }
    if let Some(ref skills) = dto.skills {
        if skills.is_empty() {
            return Err(ApiError::bad_request("At least one skill is required"));
        }
        update_doc.insert("skills", skills);
    }
    if let Some(experience) = dto.experience_years {
        update_doc.insert("experience_years", experience);
    }
    if let Some(ref education) = dto.education {
        let education_bson = mongodb::bson::to_bson(education)
            .map_err(|e| ApiError::internal_error(format!("Failed to serialize education: {}", e)))?;
        update_doc.insert("education", education_bson);
    }
    if let Some(ref work_experience) = dto.work_experience {
        let work_exp_bson = mongodb::bson::to_bson(work_experience)
            .map_err(|e| ApiError::internal_error(format!("Failed to serialize work experience: {}", e)))?;
        update_doc.insert("work_experience", work_exp_bson);
    }
    if let Some(ref categories) = dto.preferred_categories {
        update_doc.insert("preferred_categories", categories);
    }
    if let Some(ref job_types) = dto.preferred_job_types {
        update_doc.insert("preferred_job_types", job_types);
    }
    if let Some(ref locations) = dto.preferred_locations {
        update_doc.insert("preferred_locations", locations);
    }
    if let Some(salary_min) = dto.expected_salary_min {
        update_doc.insert("expected_salary_min", salary_min);
    }
    if let Some(salary_max) = dto.expected_salary_max {
        update_doc.insert("expected_salary_max", salary_max);
    }
    if let Some(relocate) = dto.willing_to_relocate {
        update_doc.insert("willing_to_relocate", relocate);
    }
    if let Some(ref resume) = dto.resume_url {
        update_doc.insert("resume_url", resume);
    }
    if let Some(ref portfolio) = dto.portfolio_url {
        update_doc.insert("portfolio_url", portfolio);
    }
    if let Some(ref linkedin) = dto.linkedin_url {
        update_doc.insert("linkedin_url", linkedin);
    }
    if let Some(available) = dto.is_available {
        update_doc.insert("is_available", available);
    }

    let result = db
        .collection::<JobSeekerProfile>("job_seeker_profiles")
        .update_one(
            doc! { "user_id": auth.user_id },
            doc! { "$set": update_doc },
            None,
        )
        .await
        .map_err(|e| ApiError::internal_error(format!("Failed to update profile: {}", e)))?;

    if result.matched_count == 0 {
        return Err(ApiError::not_found("Job seeker profile not found"));
    }

    Ok(Json(ApiResponse::success(serde_json::json!({
        "message": "Job seeker profile updated successfully"
    }))))
}

#[derive(FromForm, serde::Deserialize, rocket_okapi::okapi::schemars::JsonSchema)]
pub struct SearchJobSeekersQuery {
    pub skills: Option<String>, // Comma-separated skills
    pub category: Option<String>,
    pub min_experience: Option<i32>,
    pub max_experience: Option<i32>,
    pub location: Option<String>,
    pub job_type: Option<String>,
    pub page: Option<i64>,
    pub limit: Option<i64>,
}

#[openapi(tag = "JobSeeker")]
#[get("/job-seeker/search?<query..>")]
pub async fn search_job_seekers(
    db: &State<DbConn>,
    query: SearchJobSeekersQuery,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let page = query.page.unwrap_or(1).max(1);
    let limit = query.limit.unwrap_or(20).min(100);
    let skip = (page - 1) * limit;

    let mut filter = doc! {
        "is_available": true,
        "is_verified": true,
    };

    if let Some(skills_str) = query.skills {
        let skills: Vec<&str> = skills_str.split(',').map(|s| s.trim()).collect();
        filter.insert("skills", doc! { "$in": skills });
    }

    if let Some(category) = query.category {
        filter.insert("preferred_categories", category);
    }

    if let Some(job_type) = query.job_type {
        filter.insert("preferred_job_types", job_type);
    }

    if let Some(location) = query.location {
        filter.insert("preferred_locations", location);
    }

    if let Some(min_exp) = query.min_experience {
        filter.insert("experience_years", doc! { "$gte": min_exp });
    }

    if let Some(max_exp) = query.max_experience {
        filter.insert("experience_years", doc! { "$lte": max_exp });
    }

    let find_options = FindOptions::builder()
        .skip(skip as u64)
        .limit(limit)
        .sort(doc! {
            "subscription_plan": -1,
            "profile_views": -1,
            "created_at": -1
        })
        .build();

    let mut cursor = db
        .collection::<JobSeekerProfile>("job_seeker_profiles")
        .find(filter.clone(), find_options)
        .await
        .map_err(|e| ApiError::internal_error(format!("Database error: {}", e)))?;

    let mut profiles = Vec::new();
    while cursor
        .advance()
        .await
        .map_err(|e| ApiError::internal_error(format!("Cursor error: {}", e)))?
    {
        let profile = cursor
            .deserialize_current()
            .map_err(|e| ApiError::internal_error(format!("Deserialization error: {}", e)))?;
        profiles.push(profile);
    }

    let total = db
        .collection::<JobSeekerProfile>("job_seeker_profiles")
        .count_documents(filter, None)
        .await
        .map_err(|e| ApiError::internal_error(format!("Count error: {}", e)))?;

    Ok(Json(ApiResponse::success(serde_json::json!({
        "profiles": profiles,
        "pagination": {
            "page": page,
            "limit": limit,
            "total": total,
            "pages": (total as f64 / limit as f64).ceil() as i64,
        }
    }))))
}

#[openapi(tag = "JobSeeker")]
#[delete("/job-seeker/profile")]
pub async fn delete_job_seeker_profile(
    db: &State<DbConn>,
    auth: AuthGuard,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let result = db
        .collection::<JobSeekerProfile>("job_seeker_profiles")
        .update_one(
            doc! { "user_id": auth.user_id },
            doc! {
                "$set": {
                    "is_available": false,
                    "updated_at": DateTime::now()
                }
            },
            None,
        )
        .await
        .map_err(|e| ApiError::internal_error(format!("Failed to delete profile: {}", e)))?;

    if result.matched_count == 0 {
        return Err(ApiError::not_found("Job seeker profile not found"));
    }

    Ok(Json(ApiResponse::success(serde_json::json!({
        "message": "Job seeker profile deactivated successfully"
    }))))
}
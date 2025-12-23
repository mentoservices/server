use mongodb::bson::oid::ObjectId;
use rocket::serde::json::Json;
use rocket::{State, Request};
use rocket_okapi::openapi;
use mongodb::bson::{doc, DateTime};
use mongodb::options::FindOptions;
use crate::db::DbConn;
use crate::models::{CreateWorkerProfileDto, Subscription, WorkerSubscriptionPlan, UpdateWorkerProfileDto, WorkerProfile, SubscriptionType, SubscriptionStatus, NearbyWorkerQuery, GeoLocation, UpdateLocationDto};
use crate::guards::{AuthGuard, KycGuard};
use crate::utils::{ApiResponse, ApiError};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use crate::services::RazorpayService;
use rocket::http::Status;

// ============================================================================
// SUBSCRIPTION ENDPOINTS (Fixed)
// ============================================================================

#[derive(serde::Deserialize, rocket_okapi::okapi::schemars::JsonSchema)]
pub struct CreateSubscriptionResponse {
    pub subscription_id: String,
    pub order: serde_json::Value,
}

#[openapi(tag = "Subscription")]
#[post("/subscription/create/<plan_name>")]
pub async fn create_subscription(
    db: &State<DbConn>,
    auth: AuthGuard,
    plan_name: String,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    // Validate plan and get price
    let (price, plan_type) = match plan_name.to_lowercase().as_str() {
        "silver" => (1.0, WorkerSubscriptionPlan::Silver),
        "gold" => (2.0, WorkerSubscriptionPlan::Gold),
        _ => return Err(ApiError::bad_request("Invalid plan. Choose 'silver' or 'gold'")),
    };

    let now = DateTime::now();
    let expires_at = DateTime::from_millis(
        chrono::Utc::now().timestamp_millis() + 365 * 24 * 60 * 60 * 1000,
    );
 
    // Check if user already has an active subscription
    let existing = db
        .collection::<Subscription>("subscriptions")
        .find_one(
            doc! { 
                "user_id": auth.user_id,
                "status": "active"
            },
            None,
        )
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;

    if existing.is_some() {
        return Err(ApiError::bad_request("You already have an active subscription"));
    }

    // Create Razorpay order first
    let order = RazorpayService::create_order(price as i64)
        .await
        .map_err(|e| ApiError::internal_error(format!("Failed to create payment order: {}", e)))?;

    // Create subscription with pending status
    let subscription = Subscription {
        id: None,
        user_id: auth.user_id,
        subscription_type: SubscriptionType::Worker,
        plan_name: plan_name.clone(),
        price,
        status: SubscriptionStatus::Cancelled, // Will be updated after payment
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

    let subscription_id = sub_res.inserted_id.as_object_id()
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
pub struct VerifySubscriptionPaymentDto {
    pub subscription_id: String,
    pub razorpay_order_id: String,
    pub razorpay_payment_id: String,
    pub razorpay_signature: String,
}

#[openapi(tag = "Subscription")]
#[post("/subscription/verify", data = "<dto>")]
pub async fn verify_subscription_payment(
    db: &State<DbConn>,
    auth: AuthGuard,
    dto: Json<VerifySubscriptionPaymentDto>,
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
                "user_id": auth.user_id 
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

#[openapi(tag = "Subscription")]
#[get("/subscription/status")]
pub async fn get_subscription_status(
    db: &State<DbConn>,
    auth: AuthGuard,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    
    let subscription = db
        .collection::<Subscription>("subscriptions")
        .find_one(
            doc! { 
                "user_id": auth.user_id,
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
// WORKER PROFILE ENDPOINTS
// ============================================================================

#[openapi(tag = "Worker")]
#[post("/worker/profile", data = "<dto>")]
pub async fn create_worker_profile(
    db: &State<DbConn>,
    kyc_guard: KycGuard,
    dto: Json<CreateWorkerProfileDto>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let auth = kyc_guard.auth;
    
    // Check if user has active subscription
    let has_subscription = db
        .collection::<Subscription>("subscriptions")
        .find_one(
            doc! { 
                "user_id": auth.user_id,
                "status": "active"
            },
            None,
        )
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;

    if has_subscription.is_none() {
        return Err(ApiError::bad_request("Active subscription required to create worker profile"));
    }

    let subscription = has_subscription.unwrap();
    let subscription_plan = match subscription.plan_name.as_str() {
        "silver" => WorkerSubscriptionPlan::Silver,
        "gold" => WorkerSubscriptionPlan::Gold,
        _ => WorkerSubscriptionPlan::None,
    };
    
    let location = GeoLocation {
        geo_type: String::from("Point"),
        coordinates: [dto.longitude.unwrap_or(72.8311), dto.latitude.unwrap_or(21.1702)]
    };

    // Check if worker profile already exists
    let existing = db.collection::<WorkerProfile>("worker_profiles")
        .find_one(doc! { "user_id": auth.user_id }, None)
        .await
        .map_err(|e| ApiError::internal_error(format!("Database error: {}", e)))?;
    
    if existing.is_some() {
        return Err(ApiError::bad_request("Worker profile already exists"));
    }
    
    // Create worker profile
    let worker = WorkerProfile {
        id: None,
        user_id: auth.user_id,
        categories: dto.categories.clone(),
        subcategories: dto.subcategories.clone(),
        experience_years: dto.experience_years,
        description: dto.description.clone(),
        hourly_rate: dto.hourly_rate,
        license_number: dto.license_number.clone(),
        service_areas: dto.service_areas.clone(),
        subscription_plan,
        subscription_expires_at: Some(subscription.expires_at),
        is_verified: false,
        is_available: true,
        rating: 0.0,
        total_reviews: 0,
        total_jobs_completed: 0,
        created_at: DateTime::now(),
        location,
        updated_at: DateTime::now(),
    };
    
    let result = db.collection::<WorkerProfile>("worker_profiles")
        .insert_one(&worker, None)
        .await
        .map_err(|e| ApiError::internal_error(format!("Failed to create profile: {}", e)))?;
    
    Ok(Json(ApiResponse::success_with_message(
        "Worker profile created successfully".to_string(),
        serde_json::json!({
            "worker_id": result.inserted_id.as_object_id().unwrap().to_hex()
        })
    )))
}

#[openapi(tag = "Worker")]
#[get("/worker/profile/<worker_id>")]
pub async fn get_worker_profile_by_id(
    db: &State<DbConn>,
    worker_id: String,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let object_id = ObjectId::parse_str(&worker_id)
        .map_err(|_| ApiError::bad_request("Invalid worker ID"))?;
    
    let worker = db.collection::<WorkerProfile>("worker_profiles")
        .find_one(doc! { "_id": object_id }, None)
        .await
        .map_err(|e| ApiError::internal_error(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Worker profile not found"))?;
    
    Ok(Json(ApiResponse::success(serde_json::json!(worker))))
}

#[openapi(tag = "Worker")]
#[get("/worker/profile")]
pub async fn get_worker_profile(
    db: &State<DbConn>,
    auth: AuthGuard,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let worker = db.collection::<WorkerProfile>("worker_profiles")
        .find_one(doc! { "user_id": auth.user_id }, None)
        .await
        .map_err(|e| ApiError::internal_error(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Worker profile not found"))?;
    
    Ok(Json(ApiResponse::success(serde_json::json!(worker))))
}

#[openapi(tag = "Worker")]
#[put("/worker/profile", data = "<dto>")]
pub async fn update_worker_profile(
    db: &State<DbConn>,
    auth: AuthGuard,
    dto: Json<UpdateWorkerProfileDto>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let mut update_doc = doc! {
        "updated_at": DateTime::now()
    };
    
    if let Some(ref categories) = dto.categories {
        update_doc.insert("categories", categories);
    }
    if let Some(ref subcategories) = dto.subcategories {
        update_doc.insert("subcategories", subcategories);
    }
    if let Some(experience) = dto.experience_years {
        update_doc.insert("experience_years", experience);
    }
    if let Some(ref description) = dto.description {
        update_doc.insert("description", description);
    }
    if let Some(rate) = dto.hourly_rate {
        update_doc.insert("hourly_rate", rate);
    }
    if let Some(ref areas) = dto.service_areas {
        update_doc.insert("service_areas", areas);
    }
    if let Some(available) = dto.is_available {
        update_doc.insert("is_available", available);
    }
    
    let result = db.collection::<WorkerProfile>("worker_profiles")
        .update_one(
            doc! { "user_id": auth.user_id },
            doc! { "$set": update_doc },
            None
        )
        .await
        .map_err(|e| ApiError::internal_error(format!("Failed to update profile: {}", e)))?;
    
    if result.matched_count == 0 {
        return Err(ApiError::not_found("Worker profile not found"));
    }
    
    Ok(Json(ApiResponse::success(serde_json::json!({
        "message": "Worker profile updated successfully"
    }))))
}

#[openapi(tag = "Worker")]
#[get("/worker/search?<query..>")]
pub async fn search_workers(
    db: &State<DbConn>,
    query: SearchWorkersQuery,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let page = query.page.unwrap_or(1).max(1);
    let limit = query.limit.unwrap_or(20).min(100);
    let skip = (page - 1) * limit;
    
    let mut filter = doc! {
        "is_available": true,
        "is_verified": true,
    };
    
    if let Some(category) = query.category {
        filter.insert("categories", category);
    }
    
    if let Some(subcategory) = query.subcategory {
        filter.insert("subcategories", subcategory);
    }
    
    if let Some(min_rating) = query.min_rating {
        filter.insert("rating", doc! { "$gte": min_rating });
    }
    
    let find_options = FindOptions::builder()
        .skip(skip as u64)
        .limit(limit)
        .sort(doc! { 
            "subscription_plan": -1,
            "rating": -1,
            "total_reviews": -1
        })
        .build();
    
    let mut cursor = db.collection::<WorkerProfile>("worker_profiles")
        .find(filter.clone(), find_options)
        .await
        .map_err(|e| ApiError::internal_error(format!("Database error: {}", e)))?;
    
    let mut workers = Vec::new();
    while cursor.advance().await.map_err(|e| ApiError::internal_error(format!("Cursor error: {}", e)))? {
        let worker = cursor.deserialize_current()
            .map_err(|e| ApiError::internal_error(format!("Deserialization error: {}", e)))?;
        workers.push(worker);
    }
    
    let total = db.collection::<WorkerProfile>("worker_profiles")
        .count_documents(filter, None)
        .await
        .map_err(|e| ApiError::internal_error(format!("Count error: {}", e)))?;
    
    Ok(Json(ApiResponse::success(serde_json::json!({
        "workers": workers,
        "pagination": {
            "page": page,
            "limit": limit,
            "total": total,
            "pages": (total as f64 / limit as f64).ceil() as i64,
        }
    }))))
}

#[derive(FromForm, serde::Deserialize, rocket_okapi::okapi::schemars::JsonSchema)]
pub struct SearchWorkersQuery {
    pub category: Option<String>,
    pub subcategory: Option<String>,
    pub city: Option<String>,
    pub min_rating: Option<f64>,
    pub page: Option<i64>,
    pub limit: Option<i64>,
}

#[openapi(tag = "Worker")]
#[get("/worker/nearby?<query..>")]
pub async fn find_nearby_workers(
    db: &State<DbConn>,
    query: NearbyWorkerQuery,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let page = query.page.unwrap_or(1).max(1);
    let limit = query.limit.unwrap_or(20).min(50);
    let skip = (page - 1) * limit;

    // CRITICAL FIX: Build filter WITHOUT geo query first
    let mut match_filter = doc! {
        "is_verified": true,
        "is_available": true,
    };

    if let Some(category) = &query.category {
        match_filter.insert("categories", category);
    }

    if let Some(subcategory) = &query.subcategory {
        match_filter.insert("subcategories", subcategory);
    }

    // Use aggregation pipeline with $geoNear (works better than find with $nearSphere)
    let pipeline = vec![
        doc! {
            "$geoNear": {
                "near": {
                    "type": "Point",
                    "coordinates": [query.longitude, query.latitude]
                },
                "distanceField": "distance",
                "maxDistance": 10000,
                "spherical": true,
                "key": "location"
            }
        },
        doc! {
            "$match": match_filter.clone()
        },
        doc! {
            "$skip": skip
        },
        doc! {
            "$limit": limit
        },
        doc! {
            "$sort": {
                "distance": 1,
                "subscription_plan": -1,
                "rating": -1
            }
        }
    ];

    let mut cursor = db
        .collection::<WorkerProfile>("worker_profiles")
        .aggregate(pipeline.clone(), None)
        .await
        .map_err(|e| ApiError::internal_error(format!("Aggregation error: {}", e)))?;

    let mut workers = Vec::new();
    while cursor.advance().await.map_err(|e| ApiError::internal_error(e.to_string()))? {
        let doc = cursor.deserialize_current()
            .map_err(|e| ApiError::internal_error(e.to_string()))?;
        workers.push(doc);
    }

    // Count total (without skip/limit)
    let count_pipeline = vec![
        doc! {
            "$geoNear": {
                "near": {
                    "type": "Point",
                    "coordinates": [query.longitude, query.latitude]
                },
                "distanceField": "distance",
                "maxDistance": 10000,
                "spherical": true,
                "key": "location"
            }
        },
        doc! {
            "$match": match_filter
        },
        doc! {
            "$count": "total"
        }
    ];

    let mut count_cursor = db
        .collection::<mongodb::bson::Document>("worker_profiles")
        .aggregate(count_pipeline, None)
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;

    let total = if count_cursor.advance().await.unwrap_or(false) {
        count_cursor
            .deserialize_current()
            .ok()
            .and_then(|doc| doc.get_i64("total").ok())
            .unwrap_or(0)
    } else {
        0
    };

    Ok(Json(ApiResponse::success(serde_json::json!({
        "workers": workers,
        "pagination": {
            "page": page,
            "limit": limit,
            "total": total,
            "pages": (total as f64 / limit as f64).ceil() as i64
        }
    }))))
}

#[openapi(tag = "Worker")]
#[post("/worker/location", data = "<dto>")]
pub async fn update_worker_location(
    db: &State<DbConn>,
    auth: AuthGuard,
    dto: Json<UpdateLocationDto>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    if !(dto.latitude >= -90.0 && dto.latitude <= 90.0) {
        return Err(ApiError::bad_request("Invalid latitude"));
    }

    if !(dto.longitude >= -180.0 && dto.longitude <= 180.0) {
        return Err(ApiError::bad_request("Invalid longitude"));
    }

    let result = db
        .collection::<mongodb::bson::Document>("worker_profiles")
        .update_one(
            doc! { "user_id": auth.user_id },
            doc! {
                "$set": {
                    "location": {
                        "type": "Point",
                        "coordinates": vec![dto.longitude, dto.latitude]
                    },
                    "updated_at": DateTime::now()
                }
            },
            None,
        )
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;

    if result.matched_count == 0 {
        return Err(ApiError::not_found("Worker profile not found"));
    }

    Ok(Json(ApiResponse::success(serde_json::json!({
        "message": "Location updated successfully"
    })))) 
}
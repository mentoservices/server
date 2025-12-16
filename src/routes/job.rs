use rocket::serde::json::Json;
use rocket::State;
use rocket_okapi::openapi;
use mongodb::bson::{doc, DateTime};
use mongodb::options::FindOptions;
use crate::db::DbConn;
use crate::models::{Job, CreateJobDto, JobStatus, JobType};
use crate::guards::{AuthGuard, KycGuard};
use crate::utils::{ApiResponse, ApiError};
use rocket::form::FromForm;

#[openapi(tag = "Job")]
#[post("/job/create", data = "<dto>")]
pub async fn create_job(
    db: &State<DbConn>,
    kyc_guard: KycGuard,
    dto: Json<CreateJobDto>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let auth = kyc_guard.auth;

    // Validate required fields
    if dto.title.trim().is_empty() {
        return Err(ApiError::bad_request("Job title is required"));
    }

    if dto.description.trim().is_empty() {
        return Err(ApiError::bad_request("Job description is required"));
    }

    // Convert JobType enum to lowercase string used by Job
    let job_type_str: Option<String> = Some(match dto.job_type {
        JobType::FullTime => "fulltime".to_string(),
        JobType::PartTime => "parttime".to_string(),
        JobType::Contract => "contract".to_string(),
        JobType::Freelance => "freelance".to_string(),
    });

    // Convert experience_required Option<i32> -> Option<String>
    let experience_required_str: Option<String> = dto.experience_required.map(|v| v.to_string());

    // Create job (wrap DTO fields into Option<> where Job expects Option<String>)
    let job = Job {
        id: None,
        posted_by: auth.user_id,
        title: dto.title.clone(),
        description: dto.description.clone(),
        category: Some(dto.category.clone()),          // DTO has String, Job expects Option<String>
        job_type: job_type_str,                        // converted above
        salary_min: dto.salary_min,
        salary_max: dto.salary_max,
        location: Some(dto.location.clone()),
        city: Some(dto.city.clone()),
        pincode: Some(dto.pincode.clone()),
        required_skills: dto.required_skills.clone(),
        experience_required: experience_required_str,
        status: "open".to_string(),                    // Job.status is String
        applications: Vec::new(),
        views: 0,
        is_active: true,
        expires_at: Some(DateTime::from_millis(
            chrono::Utc::now().timestamp_millis() + (30 * 24 * 60 * 60 * 1000), // 30 days
        )),
        created_at: DateTime::now(),
        updated_at: DateTime::now(),
    };

    let result = db
        .collection::<Job>("jobs")
        .insert_one(&job, None)
        .await
        .map_err(|e| ApiError::internal_error(format!("Failed to create job: {}", e)))?;

    Ok(Json(ApiResponse::success_with_message(
        "Job posted successfully".to_string(),
        serde_json::json!({
            "job_id": result.inserted_id.as_object_id().unwrap().to_hex()
        }),
    )))
}

#[derive(FromForm, serde::Deserialize, rocket_okapi::okapi::schemars::JsonSchema)]
pub struct JobSearchQuery {
    pub category: Option<String>,
    pub job_type: Option<String>,
    pub city: Option<String>,
    pub min_salary: Option<f64>,
    pub max_salary: Option<f64>,
    pub page: Option<i64>,
    pub limit: Option<i64>,
}

#[openapi(tag = "Job")]
#[get("/job/search?<query..>")]
pub async fn get_jobs(
    db: &State<DbConn>,
    query: JobSearchQuery,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let page = query.page.unwrap_or(1).max(1);
    let limit = query.limit.unwrap_or(20).min(100);
    let skip = (page - 1) * limit;
    
    let mut filter = doc! {
        "is_active": true,
        "status": "open",
    };
    
    if let Some(category) = query.category {
        filter.insert("category", category);
    }
    
    if let Some(job_type) = query.job_type {
        filter.insert("job_type", job_type);
    }
    
    if let Some(city) = query.city {
        filter.insert("city", city);
    }
    
    if let Some(min_salary) = query.min_salary {
        filter.insert("salary_min", doc! { "$gte": min_salary });
    }
    
    if let Some(max_salary) = query.max_salary {
        filter.insert("salary_max", doc! { "$lte": max_salary });
    }
    
    let find_options = FindOptions::builder()
        .skip(skip as u64)
        .limit(limit)
        .sort(doc! { "created_at": -1 })
        .build();
    
    let mut cursor = db.collection::<Job>("jobs")
        .find(filter.clone(), find_options)
        .await
        .map_err(|e| ApiError::internal_error(format!("Database error: {}", e)))?;
    
    let mut jobs = Vec::new();
    while cursor.advance().await.map_err(|e| ApiError::internal_error(format!("Cursor error: {}", e)))? {
        let job = cursor.deserialize_current()
            .map_err(|e| ApiError::internal_error(format!("Deserialization error: {}", e)))?;
        jobs.push(job);
    }
    
    let total = db.collection::<Job>("jobs")
        .count_documents(filter, None)
        .await
        .map_err(|e| ApiError::internal_error(format!("Count error: {}", e)))?;
    
    Ok(Json(ApiResponse::success(serde_json::json!({
        "jobs": jobs,
        "pagination": {
            "page": page,
            "limit": limit,
            "total": total,
            "pages": (total as f64 / limit as f64).ceil() as i64,
        }
    }))))
}

#[openapi(tag = "Job")]
#[get("/job/<job_id>")]
pub async fn get_job_by_id(
    db: &State<DbConn>,
    job_id: String,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let object_id = mongodb::bson::oid::ObjectId::parse_str(&job_id)
        .map_err(|_| ApiError::bad_request("Invalid job ID"))?;
    
    // Increment view count
    db.collection::<Job>("jobs")
        .update_one(
            doc! { "_id": object_id },
            doc! { "$inc": { "views": 1 } },
            None
        )
        .await
        .ok();
    
    let job = db.collection::<Job>("jobs")
        .find_one(doc! { "_id": object_id }, None)
        .await
        .map_err(|e| ApiError::internal_error(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Job not found"))?;
    
    Ok(Json(ApiResponse::success(serde_json::json!(job))))
}

#[derive(FromForm, serde::Deserialize, rocket_okapi::okapi::schemars::JsonSchema, Default)]
pub struct MyJobsQuery {
    pub page: Option<i64>,
    pub limit: Option<i64>,
}

#[openapi(tag = "Job")]
#[get("/job/my/posted?<query..>")]
pub async fn get_my_posted_jobs(
    db: &State<DbConn>,
    auth: AuthGuard,
    query: MyJobsQuery,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let page = query.page.unwrap_or(1).max(1);
    let limit = query.limit.unwrap_or(20).min(100);
    let skip = (page - 1) * limit;
    
    let filter = doc! { "posted_by": auth.user_id };
    
    let find_options = FindOptions::builder()
        .skip(skip as u64)
        .limit(limit)
        .sort(doc! { "created_at": -1 })
        .build();
    
    let mut cursor = db.collection::<Job>("jobs")
        .find(filter.clone(), find_options)
        .await
        .map_err(|e| ApiError::internal_error(format!("Database error: {}", e)))?;
    
    let mut jobs = Vec::new();
    while cursor.advance().await.map_err(|e| ApiError::internal_error(format!("Cursor error: {}", e)))? {
        let job = cursor.deserialize_current()
            .map_err(|e| ApiError::internal_error(format!("Deserialization error: {}", e)))?;
        jobs.push(job);
    }
    
    let total = db.collection::<Job>("jobs")
        .count_documents(filter, None)
        .await
        .map_err(|e| ApiError::internal_error(format!("Count error: {}", e)))?;
    
    Ok(Json(ApiResponse::success(serde_json::json!({
        "jobs": jobs,
        "pagination": {
            "page": page,
            "limit": limit,
            "total": total,
            "pages": (total as f64 / limit as f64).ceil() as i64,
        }
    }))))
}

#[openapi(tag = "Job")]
#[post("/job/<job_id>/apply")]
pub async fn apply_to_job(
    db: &State<DbConn>,
    kyc_guard: KycGuard,
    job_id: String,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let auth = kyc_guard.auth;
    
    let object_id = mongodb::bson::oid::ObjectId::parse_str(&job_id)
        .map_err(|_| ApiError::bad_request("Invalid job ID"))?;
    
    // Check if already applied
    let job = db.collection::<Job>("jobs")
        .find_one(doc! { "_id": object_id }, None)
        .await
        .map_err(|e| ApiError::internal_error(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Job not found"))?;
    
    if job.applications.contains(&auth.user_id) {
        return Err(ApiError::bad_request("Already applied to this job"));
    }
    
    if job.posted_by == auth.user_id {
        return Err(ApiError::bad_request("Cannot apply to your own job"));
    }
    
    // Add application
    db.collection::<Job>("jobs")
        .update_one(
            doc! { "_id": object_id },
            doc! { 
                "$push": { "applications": auth.user_id },
                "$set": { "updated_at": DateTime::now() }
            },
            None
        )
        .await
        .map_err(|e| ApiError::internal_error(format!("Failed to apply: {}", e)))?;
    
    Ok(Json(ApiResponse::success(serde_json::json!({
        "message": "Application submitted successfully"
    }))))
}

#[derive(serde::Deserialize, rocket_okapi::okapi::schemars::JsonSchema)]
pub struct UpdateJobStatusDto {
    pub status: String, // "open", "in_progress", "completed", "cancelled"
}

#[openapi(tag = "Job")]
#[put("/job/<job_id>/status", data = "<dto>")]
pub async fn update_job_status(
    db: &State<DbConn>,
    auth: AuthGuard,
    job_id: String,
    dto: Json<UpdateJobStatusDto>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let object_id = mongodb::bson::oid::ObjectId::parse_str(&job_id)
        .map_err(|_| ApiError::bad_request("Invalid job ID"))?;
    
    // Verify ownership
    let job = db.collection::<Job>("jobs")
        .find_one(doc! { "_id": object_id }, None)
        .await
        .map_err(|e| ApiError::internal_error(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Job not found"))?;
    
    if job.posted_by != auth.user_id {
        return Err(ApiError::unauthorized("Not authorized to update this job"));
    }
    
    let status = match dto.status.as_str() {
        "open" => "open",
        "in_progress" => "in_progress",
        "completed" => "completed",
        "cancelled" => "cancelled",
        _ => return Err(ApiError::bad_request("Invalid status")),
    };
    
    db.collection::<Job>("jobs")
        .update_one(
            doc! { "_id": object_id },
            doc! { 
                "$set": { 
                    "status": status,
                    "updated_at": DateTime::now()
                }
            },
            None
        )
        .await
        .map_err(|e| ApiError::internal_error(format!("Failed to update status: {}", e)))?;
    
    Ok(Json(ApiResponse::success(serde_json::json!({
        "message": "Job status updated successfully"
    }))))
}

#[openapi(tag = "Job")]
#[delete("/job/<job_id>")]
pub async fn delete_job(
    db: &State<DbConn>,
    auth: AuthGuard,
    job_id: String,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let object_id = mongodb::bson::oid::ObjectId::parse_str(&job_id)
        .map_err(|_| ApiError::bad_request("Invalid job ID"))?;
    
    // Verify ownership
    let job = db.collection::<Job>("jobs")
        .find_one(doc! { "_id": object_id }, None)
        .await
        .map_err(|e| ApiError::internal_error(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Job not found"))?;
    
    if job.posted_by != auth.user_id {
        return Err(ApiError::unauthorized("Not authorized to delete this job"));
    }
    
    // Soft delete
    db.collection::<Job>("jobs")
        .update_one(
            doc! { "_id": object_id },
            doc! { 
                "$set": { 
                    "is_active": false,
                    "updated_at": DateTime::now()
                }
            },
            None
        )
        .await
        .map_err(|e| ApiError::internal_error(format!("Failed to delete job: {}", e)))?;
    
    Ok(Json(ApiResponse::success(serde_json::json!({
        "message": "Job deleted successfully"
    }))))
}
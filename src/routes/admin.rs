use crate::db::DbConn;
use crate::models::{CategoryResponse, MainCategory, SubCategory, SubCategoryResponse, WorkerProfile, JobSeekerProfile};
use crate::utils::{ApiError, ApiResponse};
use mongodb::bson::{doc, DateTime, oid::ObjectId};
use mongodb::options::FindOptions;
use rocket::State;
use rocket::serde::json::Json;
use rocket_okapi::openapi;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Internal struct to deserialize from MongoDB services collection
#[derive(Debug, Serialize, Deserialize)]
struct Service {
    #[serde(rename = "_id")]
    id: ObjectId,
    #[serde(rename = "serviceId")]
    service_id: String,
    name: String,
    #[serde(rename = "serviceCategory")]
    service_category: String,
    price: String,
    rating: String,
    description: String,
    icon: String,
    color: String,
}

#[openapi(tag = "Category")]
#[get("/category/all")]
pub async fn get_all_categories(
    db: &State<DbConn>,
) -> Result<Json<ApiResponse<Vec<CategoryResponse>>>, ApiError> {
    // Fetch all services from the services collection
    let mut cursor = db
        .collection::<Service>("services")
        .find(None, None)
        .await
        .map_err(|e| ApiError::internal_error(format!("Database error: {}", e)))?;

    let mut services = Vec::new();
    while cursor
        .advance()
        .await
        .map_err(|e| ApiError::internal_error(format!("Cursor error: {}", e)))?
    {
        let service = cursor
            .deserialize_current()
            .map_err(|e| ApiError::internal_error(format!("Deserialization error: {}", e)))?;
        services.push(service);
    }

    // Group services by category
    let mut categories_map: HashMap<String, Vec<Service>> = HashMap::new();
    
    for service in services {
        categories_map
            .entry(service.service_category.clone())
            .or_insert_with(Vec::new)
            .push(service);
    }

    // Convert to response format
    let mut categories: Vec<CategoryResponse> = categories_map
        .into_iter()
        .map(|(category_name, services)| {
            let icon = services.first().map(|s| s.icon.clone());
            
            let subcategories: Vec<SubCategoryResponse> = services
                .into_iter()
                .map(|service| SubCategoryResponse {
                    id: service.id.to_hex(),
                    name: service.name,
                    description: Some(service.description),
                })
                .collect();

            CategoryResponse {
                id: category_name.clone(),
                name: category_name,
                description: None,
                icon,
                subcategories,
            }
        })
        .collect();

    // Sort categories alphabetically
    categories.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(Json(ApiResponse::success(categories)))
}

#[openapi(tag = "Category")]
#[get("/category/<category_name>/subcategories")]
pub async fn get_subcategories(
    db: &State<DbConn>,
    category_name: String,
) -> Result<Json<ApiResponse<Vec<SubCategoryResponse>>>, ApiError> {
    let mut cursor = db
        .collection::<Service>("services")
        .find(
            doc! {
                "serviceCategory": &category_name
            },
            None,
        )
        .await
        .map_err(|e| ApiError::internal_error(format!("Database error: {}", e)))?;

    let mut subcategories = Vec::new();
    while cursor
        .advance()
        .await
        .map_err(|e| ApiError::internal_error(format!("Cursor error: {}", e)))?
    {
        let service = cursor
            .deserialize_current()
            .map_err(|e| ApiError::internal_error(format!("Deserialization error: {}", e)))?;

        subcategories.push(SubCategoryResponse {
            id: service.id.to_hex(),
            name: service.name,
            description: Some(service.description),
        });
    }

    if subcategories.is_empty() {
        return Err(ApiError::not_found("Category not found"));
    }

    Ok(Json(ApiResponse::success(subcategories)))
}

// ==================== WORKER ADMIN ROUTES ====================

#[derive(FromForm, serde::Deserialize, rocket_okapi::okapi::schemars::JsonSchema)]
pub struct WorkerListQuery {
    pub status: Option<String>,
    pub is_verified: Option<bool>,
    pub page: Option<i64>,
    pub limit: Option<i64>,
}

#[openapi(tag = "Admin - Workers")]
#[get("/admin/workers?<query..>")]
pub async fn get_all_workers(
    db: &State<DbConn>,
    query: WorkerListQuery,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let page = query.page.unwrap_or(1).max(1);
    let limit = query.limit.unwrap_or(20).min(100);
    let skip = (page - 1) * limit;

    let mut filter = doc! {};
    if let Some(is_verified) = query.is_verified {
        filter.insert("is_verified", is_verified);
    }

    let find_options = FindOptions::builder()
        .skip(skip as u64)
        .limit(limit)
        .sort(doc! { "created_at": -1 })
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

#[derive(serde::Deserialize, rocket_okapi::okapi::schemars::JsonSchema)]
pub struct UpdateWorkerVerificationDto {
    pub is_verified: bool,
}

#[openapi(tag = "Admin - Workers")]
#[put("/admin/workers/<worker_id>/verify", data = "<dto>")]
pub async fn verify_worker(
    db: &State<DbConn>,
    worker_id: String,
    dto: Json<UpdateWorkerVerificationDto>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let object_id = ObjectId::parse_str(&worker_id)
        .map_err(|_| ApiError::bad_request("Invalid worker ID"))?;

    db.collection::<WorkerProfile>("worker_profiles")
        .update_one(
            doc! { "_id": object_id },
            doc! { "$set": { "is_verified": dto.is_verified, "updated_at": DateTime::now() } },
            None
        )
        .await
        .map_err(|e| ApiError::internal_error(format!("Failed to update worker: {}", e)))?;

    Ok(Json(ApiResponse::success(serde_json::json!({
        "message": if dto.is_verified { "Worker verified successfully" } else { "Worker verification revoked" }
    }))))
}

// ==================== JOB SEEKER ADMIN ROUTES ====================

#[derive(FromForm, serde::Deserialize, rocket_okapi::okapi::schemars::JsonSchema)]
pub struct JobSeekerListQuery {
    pub is_verified: Option<bool>,
    pub page: Option<i64>,
    pub limit: Option<i64>,
}

#[openapi(tag = "Admin - Job Seekers")]
#[get("/admin/job-seekers?<query..>")]
pub async fn get_all_job_seekers(
    db: &State<DbConn>,
    query: JobSeekerListQuery,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let page = query.page.unwrap_or(1).max(1);
    let limit = query.limit.unwrap_or(20).min(100);
    let skip = (page - 1) * limit;

    let mut filter = doc! {};
    if let Some(is_verified) = query.is_verified {
        filter.insert("is_verified", is_verified);
    }

    let find_options = FindOptions::builder()
        .skip(skip as u64)
        .limit(limit)
        .sort(doc! { "created_at": -1 })
        .build();

    let mut cursor = db.collection::<JobSeekerProfile>("job_seeker_profiles")
        .find(filter.clone(), find_options)
        .await
        .map_err(|e| ApiError::internal_error(format!("Database error: {}", e)))?;

    let mut profiles = Vec::new();
    while cursor.advance().await.map_err(|e| ApiError::internal_error(format!("Cursor error: {}", e)))? {
        let profile = cursor.deserialize_current()
            .map_err(|e| ApiError::internal_error(format!("Deserialization error: {}", e)))?;
        profiles.push(profile);
    }

    let total = db.collection::<JobSeekerProfile>("job_seeker_profiles")
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

#[derive(serde::Deserialize, rocket_okapi::okapi::schemars::JsonSchema)]
pub struct UpdateJobSeekerVerificationDto {
    pub is_verified: bool,
    pub rejection_reason: Option<String>,
}

#[openapi(tag = "Admin - Job Seekers")]
#[put("/admin/job-seekers/<profile_id>/verify", data = "<dto>")]
pub async fn verify_job_seeker(
    db: &State<DbConn>,
    profile_id: String,
    dto: Json<UpdateJobSeekerVerificationDto>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let object_id = ObjectId::parse_str(&profile_id)
        .map_err(|_| ApiError::bad_request("Invalid profile ID"))?;

    db.collection::<JobSeekerProfile>("job_seeker_profiles")
        .update_one(
            doc! { "_id": object_id },
            doc! { "$set": { "is_verified": dto.is_verified, "updated_at": DateTime::now() } },
            None
        )
        .await
        .map_err(|e| ApiError::internal_error(format!("Failed to update job seeker: {}", e)))?;

    Ok(Json(ApiResponse::success(serde_json::json!({
        "message": if dto.is_verified { "Job seeker verified successfully" } else { "Job seeker verification revoked" }
    }))))
}

// ==================== CATEGORY ADMIN ROUTES ====================

#[derive(serde::Deserialize, rocket_okapi::okapi::schemars::JsonSchema)]
pub struct CreateCategoryDto {
    pub name: String,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub order: Option<i32>,
    pub is_active: Option<bool>,
}

#[openapi(tag = "Admin - Categories")]
#[post("/admin/categories", data = "<dto>")]
pub async fn create_category(
    db: &State<DbConn>,
    dto: Json<CreateCategoryDto>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let category = MainCategory {
        id: None,
        name: dto.name.clone(),
        description: dto.description.clone(),
        icon: dto.icon.clone(),
        is_active: dto.is_active.unwrap_or(true),
        order: dto.order.unwrap_or(0),
        created_at: DateTime::now(),
        updated_at: DateTime::now(),
    };

    let result = db.collection::<MainCategory>("main_categories")
        .insert_one(&category, None)
        .await
        .map_err(|e| ApiError::internal_error(format!("Failed to create category: {}", e)))?;

    Ok(Json(ApiResponse::success_with_message(
        "Category created successfully".to_string(),
        serde_json::json!({
            "id": result.inserted_id.as_object_id().unwrap().to_hex()
        })
    )))
}

#[openapi(tag = "Admin - Categories")]
#[put("/admin/categories/<category_id>", data = "<dto>")]
pub async fn update_category(
    db: &State<DbConn>,
    category_id: String,
    dto: Json<CreateCategoryDto>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let object_id = ObjectId::parse_str(&category_id)
        .map_err(|_| ApiError::bad_request("Invalid category ID"))?;

    let mut update_doc = doc! {
        "name": &dto.name,
        "updated_at": DateTime::now(),
    };

    if let Some(ref desc) = dto.description {
        update_doc.insert("description", desc);
    }
    if let Some(ref icon) = dto.icon {
        update_doc.insert("icon", icon);
    }
    if let Some(order) = dto.order {
        update_doc.insert("order", order);
    }
    if let Some(is_active) = dto.is_active {
        update_doc.insert("is_active", is_active);
    }

    db.collection::<MainCategory>("main_categories")
        .update_one(
            doc! { "_id": object_id },
            doc! { "$set": update_doc },
            None
        )
        .await
        .map_err(|e| ApiError::internal_error(format!("Failed to update category: {}", e)))?;

    Ok(Json(ApiResponse::success(serde_json::json!({
        "message": "Category updated successfully"
    }))))
}

#[openapi(tag = "Admin - Categories")]
#[delete("/admin/categories/<category_id>")]
pub async fn delete_category(
    db: &State<DbConn>,
    category_id: String,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let object_id = ObjectId::parse_str(&category_id)
        .map_err(|_| ApiError::bad_request("Invalid category ID"))?;

    db.collection::<SubCategory>("sub_categories")
        .delete_many(doc! { "main_category_id": object_id }, None)
        .await
        .ok();

    db.collection::<MainCategory>("main_categories")
        .delete_one(doc! { "_id": object_id }, None)
        .await
        .map_err(|e| ApiError::internal_error(format!("Failed to delete category: {}", e)))?;

    Ok(Json(ApiResponse::success(serde_json::json!({
        "message": "Category deleted successfully"
    }))))
}

// ==================== SUBCATEGORY ADMIN ROUTES ====================

#[derive(serde::Deserialize, rocket_okapi::okapi::schemars::JsonSchema)]
pub struct CreateSubcategoryDto {
    pub main_category_id: String,
    pub name: String,
    pub description: Option<String>,
    pub order: Option<i32>,
    pub is_active: Option<bool>,
}

#[openapi(tag = "Admin - Categories")]
#[post("/admin/subcategories", data = "<dto>")]
pub async fn create_subcategory(
    db: &State<DbConn>,
    dto: Json<CreateSubcategoryDto>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let main_category_id = if dto.main_category_id.len() == 24 {
        ObjectId::parse_str(&dto.main_category_id)
            .map_err(|_| ApiError::bad_request("Invalid main category ID"))?
    } else {
        let category_cursor = db.collection::<MainCategory>("main_categories")
            .find_one(doc! { "name": &dto.main_category_id }, None)
            .await
            .map_err(|e| ApiError::internal_error(format!("Database error: {}", e)))?;

        if let Some(category) = category_cursor {
            category.id.ok_or_else(|| ApiError::bad_request("Category missing ID"))?
        } else {
            let new_category = MainCategory {
                id: None,
                name: dto.main_category_id.clone(),
                description: None,
                icon: None,
                is_active: true,
                order: 0,
                created_at: DateTime::now(),
                updated_at: DateTime::now(),
            };

            let result = db.collection::<MainCategory>("main_categories")
                .insert_one(&new_category, None)
                .await
                .map_err(|e| ApiError::internal_error(format!("Failed to create category: {}", e)))?;

            result.inserted_id.as_object_id()
                .ok_or_else(|| ApiError::internal_error("Failed to get inserted category ID"))?
        }
    };

    let subcategory = SubCategory {
        id: None,
        main_category_id,
        name: dto.name.clone(),
        description: dto.description.clone(),
        is_active: dto.is_active.unwrap_or(true),
        order: dto.order.unwrap_or(0),
        created_at: DateTime::now(),
        updated_at: DateTime::now(),
    };

    let result = db.collection::<SubCategory>("sub_categories")
        .insert_one(&subcategory, None)
        .await
        .map_err(|e| ApiError::internal_error(format!("Failed to create subcategory: {}", e)))?;

    Ok(Json(ApiResponse::success_with_message(
        "Subcategory created successfully".to_string(),
        serde_json::json!({
            "id": result.inserted_id.as_object_id().unwrap().to_hex()
        })
    )))
}

#[openapi(tag = "Admin - Categories")]
#[put("/admin/subcategories/<subcategory_id>", data = "<dto>")]
pub async fn update_subcategory(
    db: &State<DbConn>,
    subcategory_id: String,
    dto: Json<CreateSubcategoryDto>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let object_id = ObjectId::parse_str(&subcategory_id)
        .map_err(|_| ApiError::bad_request("Invalid subcategory ID"))?;

    let mut update_doc = doc! {
        "name": &dto.name,
        "updated_at": DateTime::now(),
    };

    if let Some(ref desc) = dto.description {
        update_doc.insert("description", desc);
    }
    if let Some(order) = dto.order {
        update_doc.insert("order", order);
    }
    if let Some(is_active) = dto.is_active {
        update_doc.insert("is_active", is_active);
    }

    db.collection::<SubCategory>("sub_categories")
        .update_one(
            doc! { "_id": object_id },
            doc! { "$set": update_doc },
            None
        )
        .await
        .map_err(|e| ApiError::internal_error(format!("Failed to update subcategory: {}", e)))?;

    Ok(Json(ApiResponse::success(serde_json::json!({
        "message": "Subcategory updated successfully"
    }))))
}

#[openapi(tag = "Admin - Categories")]
#[delete("/admin/subcategories/<subcategory_id>")]
pub async fn delete_subcategory(
    db: &State<DbConn>,
    subcategory_id: String,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let object_id = ObjectId::parse_str(&subcategory_id)
        .map_err(|_| ApiError::bad_request("Invalid subcategory ID"))?;

    db.collection::<SubCategory>("sub_categories")
        .delete_one(doc! { "_id": object_id }, None)
        .await
        .map_err(|e| ApiError::internal_error(format!("Failed to delete subcategory: {}", e)))?;

    Ok(Json(ApiResponse::success(serde_json::json!({
        "message": "Subcategory deleted successfully"
    }))))
}

// ==================== JOBS ADMIN ROUTES ====================

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Job {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub title: String,
    pub company: Option<String>,
    pub description: Option<String>,
    pub location: Option<String>,
    pub job_type: Option<String>,
    pub category: Option<String>,
    pub salary_min: Option<f64>,
    pub salary_max: Option<f64>,
    pub requirements: Option<Vec<String>>,
    pub status: String,
    pub rejection_reason: Option<String>,
    pub applications_count: i32,
    pub posted_by: Option<ObjectId>,
    pub created_at: DateTime,
    pub updated_at: DateTime,
}

#[derive(FromForm, serde::Deserialize, rocket_okapi::okapi::schemars::JsonSchema)]
pub struct JobListQuery {
    pub status: Option<String>,
    pub page: Option<i64>,
    pub limit: Option<i64>,
}

#[openapi(tag = "Admin - Jobs")]
#[get("/admin/jobs?<query..>")]
pub async fn get_all_jobs(
    db: &State<DbConn>,
    query: JobListQuery,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let page = query.page.unwrap_or(1).max(1);
    let limit = query.limit.unwrap_or(20).min(100);
    let skip = (page - 1) * limit;

    let mut filter = doc! {};
    if let Some(status) = query.status {
        filter.insert("status", status);
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

#[derive(serde::Deserialize, rocket_okapi::okapi::schemars::JsonSchema)]
pub struct UpdateJobStatusDto {
    pub status: String,
    pub rejection_reason: Option<String>,
}

#[openapi(tag = "Admin - Jobs")]
#[put("/admin/jobs/<job_id>/status", data = "<dto>")]
pub async fn update_job_status(
    db: &State<DbConn>,
    job_id: String,
    dto: Json<UpdateJobStatusDto>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let object_id = ObjectId::parse_str(&job_id)
        .map_err(|_| ApiError::bad_request("Invalid job ID"))?;

    let mut update_doc = doc! {
        "status": &dto.status,
        "updated_at": DateTime::now(),
    };

    if let Some(ref reason) = dto.rejection_reason {
        update_doc.insert("rejection_reason", reason);
    }

    db.collection::<Job>("jobs")
        .update_one(
            doc! { "_id": object_id },
            doc! { "$set": update_doc },
            None
        )
        .await
        .map_err(|e| ApiError::internal_error(format!("Failed to update job: {}", e)))?;

    Ok(Json(ApiResponse::success(serde_json::json!({
        "message": format!("Job status updated to {}", dto.status)
    }))))
}

#[openapi(tag = "Admin - Jobs")]
#[delete("/admin/jobs/<job_id>")]
pub async fn delete_job(
    db: &State<DbConn>,
    job_id: String,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let object_id = ObjectId::parse_str(&job_id)
        .map_err(|_| ApiError::bad_request("Invalid job ID"))?;

    db.collection::<Job>("jobs")
        .delete_one(doc! { "_id": object_id }, None)
        .await
        .map_err(|e| ApiError::internal_error(format!("Failed to delete job: {}", e)))?;

    Ok(Json(ApiResponse::success(serde_json::json!({
        "message": "Job deleted successfully"
    }))))
}
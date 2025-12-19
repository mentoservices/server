use mongodb::bson::doc;
use rocket::serde::json::Json;
use rocket::{State, get};
use rocket_okapi::openapi;

use crate::models::Service;
use crate::db::DbConn;
use crate::guards::AuthGuard;
use crate::utils::{ApiResponse, ApiError};

/// Get all services
#[openapi(tag = "Services")]
#[get("/services")]
pub async fn get_all_services(
    db: &State<DbConn>,
    _auth: AuthGuard,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let mut cursor = db
        .collection::<Service>("services")
        .find(None, None)
        .await
        .map_err(|e| ApiError::internal_error(format!("Database error: {}", e)))?;
    
    let mut services = Vec::new();
    while cursor.advance().await.map_err(|e| ApiError::internal_error(format!("Cursor error: {}", e)))? {
        let service = cursor.deserialize_current()
            .map_err(|e| ApiError::internal_error(format!("Deserialization error: {}", e)))?;
        services.push(service);
    }
    
    Ok(Json(ApiResponse::success(serde_json::json!({
        "services": services,
        "total": services.len()
    }))))
}

/// Get services by category
#[openapi(tag = "Services")]
#[get("/services/category/<category>")]
pub async fn get_services_by_category(
    category: String,
    db: &State<DbConn>,
    _auth: AuthGuard,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let filter = doc! { "serviceCategory": &category };
    
    let mut cursor = db
        .collection::<Service>("services")
        .find(filter.clone(), None)
        .await
        .map_err(|e| ApiError::internal_error(format!("Database error: {}", e)))?;
    
    let mut services = Vec::new();
    while cursor.advance().await.map_err(|e| ApiError::internal_error(format!("Cursor error: {}", e)))? {
        let service = cursor.deserialize_current()
            .map_err(|e| ApiError::internal_error(format!("Deserialization error: {}", e)))?;
        services.push(service);
    }
    
    if services.is_empty() {
        return Err(ApiError::not_found(format!("No services found for category: {}", category)));
    }
    
    Ok(Json(ApiResponse::success_with_message(
        format!("Services for '{}' fetched successfully", category),
        serde_json::json!({
            "category": category,
            "services": services,
            "total": services.len()
        })
    )))
}

/// Get all unique service categories
#[openapi(tag = "Services")]
#[get("/services/categories")]
pub async fn get_all_categories(
    db: &State<DbConn>,
    _auth: AuthGuard,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let categories = db
        .collection::<Service>("services")
        .distinct("serviceCategory", None, None)
        .await
        .map_err(|e| ApiError::internal_error(format!("Database error: {}", e)))?;
    
    let category_list: Vec<String> = categories
        .iter()
        .filter_map(|b| b.as_str().map(|s| s.to_string()))
        .collect();
    
    Ok(Json(ApiResponse::success_with_message(
        "Categories fetched successfully".to_string(),
        serde_json::json!({
            "categories": category_list,
            "total": category_list.len()
        })
    )))
}

/// Search services by name or description
#[openapi(tag = "Services")]
#[get("/services/search/<query>")]
pub async fn search_services(
    query: String,
    db: &State<DbConn>,
    _auth: AuthGuard,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let filter = doc! {
        "$or": [
            { "name": { "$regex": &query, "$options": "i" } },
            { "description": { "$regex": &query, "$options": "i" } }
        ]
    };
    
    let mut cursor = db
        .collection::<Service>("services")
        .find(filter, None)
        .await
        .map_err(|e| ApiError::internal_error(format!("Database error: {}", e)))?;
    
    let mut services = Vec::new();
    while cursor.advance().await.map_err(|e| ApiError::internal_error(format!("Cursor error: {}", e)))? {
        let service = cursor.deserialize_current()
            .map_err(|e| ApiError::internal_error(format!("Deserialization error: {}", e)))?;
        services.push(service);
    }
    
    Ok(Json(ApiResponse::success_with_message(
        format!("Found {} services matching '{}'", services.len(), query),
        serde_json::json!({
            "query": query,
            "services": services,
            "total": services.len()
        })
    )))
}

/// Get a single service by ID
#[openapi(tag = "Services")]
#[get("/services/<service_id>")]
pub async fn get_service_by_id(
    service_id: String,
    db: &State<DbConn>,
    _auth: AuthGuard,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let filter = doc! { "serviceId": &service_id };
    
    let service = db
        .collection::<Service>("services")
        .find_one(filter, None)
        .await
        .map_err(|e| ApiError::internal_error(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::not_found(format!("Service with ID '{}' not found", service_id)))?;
    
    Ok(Json(ApiResponse::success_with_message(
        "Service fetched successfully".to_string(),
        serde_json::json!(service)
    )))
}
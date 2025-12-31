use crate::db::DbConn;
use crate::models::{CategoryResponse, SubCategoryResponse};
use crate::utils::{ApiError, ApiResponse};
use mongodb::bson::doc;
use rocket::State;
use rocket::serde::json::Json;
use rocket_okapi::openapi;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Internal struct to deserialize from MongoDB
#[derive(Debug, Serialize, Deserialize)]
struct Service {
    #[serde(rename = "_id")]
    id: mongodb::bson::oid::ObjectId,
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
            // Use the first service's icon for the category (clone before moving `services`)
            let first_icon = services.first().map(|s| s.icon.clone());
            
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
                icon: first_icon,
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
    // Find all services in this category
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
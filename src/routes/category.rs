use rocket::serde::json::Json;
use rocket::State;
use rocket_okapi::openapi;
use mongodb::bson::doc;
use mongodb::options::FindOptions;
use crate::db::DbConn;
use crate::models::{MainCategory, SubCategory, CategoryResponse, SubCategoryResponse};
use crate::utils::{ApiResponse, ApiError};

#[openapi(tag = "Category")]
#[get("/category/all")]
pub async fn get_all_categories(
    db: &State<DbConn>,
) -> Result<Json<ApiResponse<Vec<CategoryResponse>>>, ApiError> {
    let find_options = FindOptions::builder()
        .sort(doc! { "order": 1 })
        .build();
    
    let mut cursor = db.collection::<MainCategory>("main_categories")
        .find(doc! { "is_active": true }, find_options)
        .await
        .map_err(|e| ApiError::internal_error(format!("Database error: {}", e)))?;
    
    let mut categories = Vec::new();
    
    while cursor.advance().await.map_err(|e| ApiError::internal_error(format!("Cursor error: {}", e)))? {
        let main_cat = cursor.deserialize_current()
            .map_err(|e| ApiError::internal_error(format!("Deserialization error: {}", e)))?;
        
        // Get subcategories
        let sub_find_options = FindOptions::builder()
            .sort(doc! { "order": 1 })
            .build();
        
        let mut sub_cursor = db.collection::<SubCategory>("sub_categories")
            .find(
                doc! { 
                    "main_category_id": main_cat.id.as_ref().unwrap(),
                    "is_active": true 
                },
                sub_find_options
            )
            .await
            .map_err(|e| ApiError::internal_error(format!("Database error: {}", e)))?;
        
        let mut subcategories = Vec::new();
        while sub_cursor.advance().await.map_err(|e| ApiError::internal_error(format!("Cursor error: {}", e)))? {
            let sub_cat = sub_cursor.deserialize_current()
                .map_err(|e| ApiError::internal_error(format!("Deserialization error: {}", e)))?;
            
            subcategories.push(SubCategoryResponse {
                id: sub_cat.id.unwrap().to_hex(),
                name: sub_cat.name,
                description: sub_cat.description,
            });
        }
        
        categories.push(CategoryResponse {
            id: main_cat.id.unwrap().to_hex(),
            name: main_cat.name,
            description: main_cat.description,
            icon: main_cat.icon,
            subcategories,
        });
    }
    
    Ok(Json(ApiResponse::success(categories)))
}

#[openapi(tag = "Category")]
#[get("/category/<category_id>/subcategories")]
pub async fn get_subcategories(
    db: &State<DbConn>,
    category_id: String,
) -> Result<Json<ApiResponse<Vec<SubCategoryResponse>>>, ApiError> {
    let object_id = mongodb::bson::oid::ObjectId::parse_str(&category_id)
        .map_err(|_| ApiError::bad_request("Invalid category ID"))?;
    
    let find_options = FindOptions::builder()
        .sort(doc! { "order": 1 })
        .build();
    
    let mut cursor = db.collection::<SubCategory>("sub_categories")
        .find(
            doc! { 
                "main_category_id": object_id,
                "is_active": true 
            },
            find_options
        )
        .await
        .map_err(|e| ApiError::internal_error(format!("Database error: {}", e)))?;
    
    let mut subcategories = Vec::new();
    while cursor.advance().await.map_err(|e| ApiError::internal_error(format!("Cursor error: {}", e)))? {
        let sub_cat = cursor.deserialize_current()
            .map_err(|e| ApiError::internal_error(format!("Deserialization error: {}", e)))?;
        
        subcategories.push(SubCategoryResponse {
            id: sub_cat.id.unwrap().to_hex(),
            name: sub_cat.name,
            description: sub_cat.description,
        });
    }
    
    Ok(Json(ApiResponse::success(subcategories)))
}
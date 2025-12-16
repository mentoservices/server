use rocket::serde::json::Json;
use rocket::State;
use rocket_okapi::openapi;
use mongodb::bson::{doc, DateTime};
use mongodb::options::FindOptions;
use crate::db::DbConn;
use crate::models::{Review, CreateReviewDto, WorkerProfile};
use crate::guards::AuthGuard;
use crate::utils::{ApiResponse, ApiError};
use rocket::futures::TryStreamExt;

#[openapi(tag = "Review")]
#[post("/review/create", data = "<dto>")]
pub async fn create_review(
    db: &State<DbConn>,
    auth: AuthGuard,
    dto: Json<CreateReviewDto>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    // Validate rating
    if dto.rating < 1 || dto.rating > 5 {
        return Err(ApiError::bad_request("Rating must be between 1 and 5"));
    }
    
    let worker_id = mongodb::bson::oid::ObjectId::parse_str(&dto.worker_id)
        .map_err(|_| ApiError::bad_request("Invalid worker ID"))?;
    
    // Check if worker exists
    let worker = db.collection::<WorkerProfile>("worker_profiles")
        .find_one(doc! { "_id": worker_id }, None)
        .await
        .map_err(|e| ApiError::internal_error(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Worker not found"))?;
    
    // Check if user already reviewed this worker
    let existing_review = db.collection::<Review>("reviews")
        .find_one(
            doc! { 
                "worker_id": worker_id,
                "user_id": auth.user_id 
            },
            None
        )
        .await
        .map_err(|e| ApiError::internal_error(format!("Database error: {}", e)))?;
    
    if existing_review.is_some() {
        return Err(ApiError::bad_request("You have already reviewed this worker"));
    }
    
    // Create review
    let review = Review {
        id: None,
        worker_id,
        user_id: auth.user_id,
        rating: dto.rating,
        comment: dto.comment.clone(),
        helpful_count: 0,
        created_at: DateTime::now(),
        updated_at: DateTime::now(),
    };
    
    let result = db.collection::<Review>("reviews")
        .insert_one(&review, None)
        .await
        .map_err(|e| ApiError::internal_error(format!("Failed to create review: {}", e)))?;
    
    // Update worker rating
    let all_reviews: Vec<Review> = db.collection::<Review>("reviews")
        .find(doc! { "worker_id": worker_id }, None)
        .await
        .map_err(|e| ApiError::internal_error(format!("Database error: {}", e)))?
        .try_collect()
        .await
        .map_err(|e| ApiError::internal_error(format!("Collection error: {}", e)))?;
    
    let total_reviews = all_reviews.len() as i32;
    let avg_rating = all_reviews.iter().map(|r| r.rating).sum::<i32>() as f64 / total_reviews as f64;
    
    db.collection::<WorkerProfile>("worker_profiles")
        .update_one(
            doc! { "_id": worker_id },
            doc! { 
                "$set": { 
                    "rating": avg_rating,
                    "total_reviews": total_reviews,
                    "updated_at": DateTime::now()
                }
            },
            None
        )
        .await
        .ok();
    
    Ok(Json(ApiResponse::success_with_message(
        "Review submitted successfully".to_string(),
        serde_json::json!({
            "review_id": result.inserted_id.as_object_id().unwrap().to_hex()
        })
    )))
}

#[derive(FromForm,serde::Deserialize, rocket_okapi::okapi::schemars::JsonSchema)]
pub struct WorkerReviewsQuery {
    pub page: Option<i64>,
    pub limit: Option<i64>,
}

#[openapi(tag = "Review")]
#[get("/review/worker/<worker_id>?<query..>")]
pub async fn get_worker_reviews(
    db: &State<DbConn>,
    worker_id: String,
    query: WorkerReviewsQuery,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let page = query.page.unwrap_or(1).max(1);
    let limit = query.limit.unwrap_or(20).min(100);
    let skip = (page - 1) * limit;
    
    let object_id = mongodb::bson::oid::ObjectId::parse_str(&worker_id)
        .map_err(|_| ApiError::bad_request("Invalid worker ID"))?;
    
    let filter = doc! { "worker_id": object_id };
    
    let find_options = FindOptions::builder()
        .skip(skip as u64)
        .limit(limit)
        .sort(doc! { "created_at": -1 })
        .build();
    
    let mut cursor = db.collection::<Review>("reviews")
        .find(filter.clone(), find_options)
        .await
        .map_err(|e| ApiError::internal_error(format!("Database error: {}", e)))?;
    
    let mut reviews = Vec::new();
    while cursor.advance().await.map_err(|e| ApiError::internal_error(format!("Cursor error: {}", e)))? {
        let review = cursor.deserialize_current()
            .map_err(|e| ApiError::internal_error(format!("Deserialization error: {}", e)))?;
        reviews.push(review);
    }
    
    let total = db.collection::<Review>("reviews")
        .count_documents(filter, None)
        .await
        .map_err(|e| ApiError::internal_error(format!("Count error: {}", e)))?;
    
    Ok(Json(ApiResponse::success(serde_json::json!({
        "reviews": reviews,
        "pagination": {
            "page": page,
            "limit": limit,
            "total": total,
            "pages": (total as f64 / limit as f64).ceil() as i64,
        }
    }))))
}

#[openapi(tag = "Review")]
#[delete("/review/<review_id>")]
pub async fn delete_review(
    db: &State<DbConn>,
    auth: AuthGuard,
    review_id: String,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let object_id = mongodb::bson::oid::ObjectId::parse_str(&review_id)
        .map_err(|_| ApiError::bad_request("Invalid review ID"))?;
    
    // Verify ownership
    let review = db.collection::<Review>("reviews")
        .find_one(doc! { "_id": object_id }, None)
        .await
        .map_err(|e| ApiError::internal_error(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Review not found"))?;
    
    if review.user_id != auth.user_id {
        return Err(ApiError::unauthorized("Not authorized to delete this review"));
    }
    
    db.collection::<Review>("reviews")
        .delete_one(doc! { "_id": object_id }, None)
        .await
        .map_err(|e| ApiError::internal_error(format!("Failed to delete review: {}", e)))?;
    
    // Recalculate worker rating
    let all_reviews: Vec<Review> = db.collection::<Review>("reviews")
        .find(doc! { "worker_id": review.worker_id }, None)
        .await
        .map_err(|e| ApiError::internal_error(format!("Database error: {}", e)))?
        .try_collect()
        .await
        .map_err(|e| ApiError::internal_error(format!("Collection error: {}", e)))?;
    
    let total_reviews = all_reviews.len() as i32;
    let avg_rating = if total_reviews > 0 {
        all_reviews.iter().map(|r| r.rating).sum::<i32>() as f64 / total_reviews as f64
    } else {
        0.0
    };
    
    db.collection::<WorkerProfile>("worker_profiles")
        .update_one(
            doc! { "_id": review.worker_id },
            doc! { 
                "$set": { 
                    "rating": avg_rating,
                    "total_reviews": total_reviews,
                    "updated_at": DateTime::now()
                }
            },
            None
        )
        .await
        .ok();
    
    Ok(Json(ApiResponse::success(serde_json::json!({
        "message": "Review deleted successfully"
    }))))
}
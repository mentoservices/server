use mongodb::bson::{oid::ObjectId, DateTime};
use serde::{Deserialize, Serialize};
use rocket_okapi::okapi::schemars::JsonSchema;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Review {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub worker_id: ObjectId,
    pub user_id: ObjectId,
    pub rating: i32, // 1-5
    pub comment: Option<String>,
    pub helpful_count: i32,
    pub created_at: DateTime,
    pub updated_at: DateTime,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreateReviewDto {
    pub worker_id: String,
    pub rating: i32,
    pub comment: Option<String>,
}
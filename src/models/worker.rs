use mongodb::bson::{oid::ObjectId, DateTime};
use serde::{Deserialize, Serialize};
use rocket_okapi::okapi::schemars::JsonSchema;

#[derive(Debug, FromForm, Deserialize, JsonSchema)]
pub struct NearbyWorkerQuery {
    pub latitude: f64,
    pub longitude: f64,

    pub category: Option<String>,
    pub subcategory: Option<String>,

    pub page: Option<i64>,
    pub limit: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum WorkerSubscriptionPlan {
    None,
    Silver,
    Gold,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct UpdateLocationDto {
    pub latitude: f64,
    pub longitude: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GeoLocation {
    #[serde(rename = "type")]
    pub geo_type: String, // "Point"
    pub coordinates: [f64; 2], // [longitude, latitude]
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WorkerProfile {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub user_id: ObjectId,
    pub categories: Vec<String>,
    pub subcategories: Vec<String>,
    pub experience_years: Option<i32>,
    pub description: Option<String>,
    pub hourly_rate: Option<f64>,
    pub license_number: Option<String>,
    pub service_areas: Vec<String>,
    pub subscription_plan: WorkerSubscriptionPlan,
    pub subscription_expires_at: Option<DateTime>,
    pub is_verified: bool,
    pub is_available: bool,
    pub rating: f64,
    pub total_reviews: i32,
    pub total_jobs_completed: i32,
    pub created_at: DateTime,
    pub updated_at: DateTime,
    pub location: GeoLocation,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreateWorkerProfileDto {
    pub categories: Vec<String>,
    pub subcategories: Vec<String>,
    pub experience_years: Option<i32>,
    pub description: Option<String>,
    pub hourly_rate: Option<f64>,
    pub license_number: Option<String>,
    pub longitude: Option<f64>,
    pub latitude: Option<f64>,
    pub service_areas: Vec<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct UpdateWorkerProfileDto {
    pub categories: Option<Vec<String>>,
    pub subcategories: Option<Vec<String>>,
    pub experience_years: Option<i32>,
    pub description: Option<String>,
    pub hourly_rate: Option<f64>,
    pub service_areas: Option<Vec<String>>,
    pub is_available: Option<bool>,
}
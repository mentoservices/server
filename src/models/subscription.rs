use mongodb::bson::{oid::ObjectId, DateTime};
use serde::{Deserialize, Serialize};
use rocket_okapi::okapi::schemars::JsonSchema;

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum SubscriptionType {
    Worker,
    JobSeeker,
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum SubscriptionStatus {
    Active,
    Expired,
    Cancelled,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Subscription {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub user_id: ObjectId,
    pub subscription_type: SubscriptionType,
    pub plan_name: String, // "silver", "gold", "job_seeker_premium"
    pub price: f64,
    pub status: SubscriptionStatus,
    pub starts_at: DateTime,
    pub expires_at: DateTime,
    pub auto_renew: bool,
    pub payment_id: Option<String>,
    pub created_at: DateTime,
    pub updated_at: DateTime,
}

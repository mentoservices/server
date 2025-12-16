use mongodb::bson::{oid::ObjectId, DateTime};
use serde::{Deserialize, Serialize};
use rocket_okapi::okapi::schemars::JsonSchema;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Otp {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub mobile: String,
    pub email: String,
    pub otp: String,
    pub expires_at: DateTime,
    pub verified: bool,
    pub attempts: i32,
    pub created_at: DateTime,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SendOtpDto {
    pub mobile: String,
    pub email: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct VerifyOtpDto {
    pub mobile: String,
    pub otp: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ResendOtpDto {
    pub mobile: String,
    pub email: String,
}
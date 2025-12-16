use mongodb::bson::{oid::ObjectId, DateTime};
use serde::{Deserialize, Serialize};
use rocket_okapi::okapi::schemars::JsonSchema;

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum DocumentType {
    Aadhaar,
    Pan,
    DrivingLicense,
    VoterId,
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum KycStatusEnum {
    Pending,
    Submitted,
    UnderReview,
    Approved,
    Rejected,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Kyc {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub user_id: ObjectId,
    pub full_name: String,
    pub date_of_birth: DateTime,
    pub address: String,
    pub city: String,
    pub state: String,
    pub pincode: String,
    pub document_type: DocumentType,
    pub document_number: String,
    pub document_front_image: String,
    pub document_back_image: Option<String>,
    pub selfie_image: String,
    pub status: KycStatusEnum,
    pub rejection_reason: Option<String>,
    pub verified_by: Option<ObjectId>,
    pub verified_at: Option<DateTime>,
    pub created_at: DateTime,
    pub updated_at: DateTime,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SubmitKycDto {
    pub full_name: String,
    pub date_of_birth: String,
    pub address: String,
    pub city: String,
    pub state: String,
    pub pincode: String,
    pub document_type: DocumentType,
    pub document_number: String,
    pub document_front_image: String,
    pub document_back_image: Option<String>,
    pub selfie_image: String,
}
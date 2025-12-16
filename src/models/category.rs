use mongodb::bson::{oid::ObjectId, DateTime as BsonDateTime};
use serde::{Deserialize, Serialize};
use rocket_okapi::okapi::schemars::JsonSchema;

/// Dummy provider to satisfy serde's `default` without needing `Default` trait
fn default_bson_datetime() -> BsonDateTime {
    // Use Unix epoch as safe placeholder
    BsonDateTime::from_millis(0)
}

/// Main Category stored in MongoDB
#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
pub struct MainCategory {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    #[schemars(skip)]
    pub id: Option<ObjectId>,

    pub name: String,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub is_active: bool,
    pub order: i32,

    #[serde(default = "default_bson_datetime", skip_deserializing, skip_serializing)]
    #[schemars(skip)]
    pub created_at: BsonDateTime,

    #[serde(default = "default_bson_datetime", skip_deserializing, skip_serializing)]
    #[schemars(skip)]
    pub updated_at: BsonDateTime,
}

/// Sub-Category stored in MongoDB
#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
pub struct SubCategory {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    #[schemars(skip)]
    pub id: Option<ObjectId>,

    #[schemars(skip)]
    pub main_category_id: ObjectId,

    pub name: String,
    pub description: Option<String>,
    pub is_active: bool,
    pub order: i32,

    #[serde(default = "default_bson_datetime", skip_deserializing, skip_serializing)]
    #[schemars(skip)]
    pub created_at: BsonDateTime,

    #[serde(default = "default_bson_datetime", skip_deserializing, skip_serializing)]
    #[schemars(skip)]
    pub updated_at: BsonDateTime,
}

/// Response model returned to clients (used in APIs)
#[derive(Debug, Serialize, JsonSchema)]
pub struct CategoryResponse {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub subcategories: Vec<SubCategoryResponse>,
}

/// Response model for sub-categories
#[derive(Debug, Serialize, JsonSchema)]
pub struct SubCategoryResponse {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
}

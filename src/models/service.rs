use mongodb::bson::oid::ObjectId;
use rocket::serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct Service {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    
    #[serde(rename = "serviceId", alias = "service_id")]
    pub service_id: String,
    
    pub name: String,
    
    #[serde(rename = "serviceCategory", alias = "service_category")]
    pub service_category: String,
    
    pub price: String,
    
    pub rating: String,
    
    pub description: String,
    
    pub icon: String,
    
    pub color: String,
}
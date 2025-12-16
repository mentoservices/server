use rocket::serde::json::Json;
use rocket::State;
use rocket::fs::TempFile;
use rocket_okapi::openapi;
use mongodb::bson::{doc, DateTime};
use crate::db::DbConn;
use crate::models::{User, UpdateProfileDto, UserResponse, Subscription, WorkerProfile};
use crate::guards::AuthGuard;
use crate::utils::{ApiResponse, ApiError, validate_email, validate_pincode};
use std::path::Path;
use tokio::fs;

#[derive(serde::Deserialize, rocket_okapi::okapi::schemars::JsonSchema)]
pub struct UpdateFcmTokenDto {
    pub token: String,
    pub platform: String, // "android" or "ios"
}

#[openapi(tag = "User")]
#[get("/user/profile")]
pub async fn get_profile(
    db: &State<DbConn>,
    auth: AuthGuard,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let user = db.collection::<User>("users")
        .find_one(doc! { "_id": auth.user_id }, None)
        .await
        .map_err(|e| ApiError::internal_error(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::not_found("User not found"))?;
    
    // Check for active subscription
    let subscription = db.collection::<Subscription>("subscriptions")
        .find_one(
            doc! { 
                "user_id": auth.user_id,
                "status": "active"
            },
            None
        )
        .await
        .map_err(|e| ApiError::internal_error(format!("Database error: {}", e)))?;
    
    // Check for worker profile
    let worker_profile = db.collection::<WorkerProfile>("worker_profiles")
        .find_one(doc! { "user_id": auth.user_id }, None)
        .await
        .map_err(|e| ApiError::internal_error(format!("Database error: {}", e)))?;
    
    let user_response: UserResponse = user.into();
    
    // Build response with subscription and worker profile info
    let mut response_data = serde_json::to_value(&user_response)
        .map_err(|e| ApiError::internal_error(format!("Serialization error: {}", e)))?;
    
    if let Some(sub) = subscription {
        response_data["subscription_id"] = serde_json::json!(sub.id.map(|id| id.to_hex()));
        response_data["subscription_plan"] = serde_json::json!(sub.plan_name);
        response_data["subscription_expires_at"] = serde_json::json!(sub.expires_at);
    } else {
        response_data["subscription_id"] = serde_json::Value::Null;
        response_data["subscription_plan"] = serde_json::Value::Null;
        response_data["subscription_expires_at"] = serde_json::Value::Null;
    }
    
    if let Some(worker) = worker_profile {
        response_data["worker_profile_id"] = serde_json::json!(worker.id.map(|id| id.to_hex()));
        response_data["worker_is_verified"] = serde_json::json!(worker.is_verified);
    } else {
        response_data["worker_profile_id"] = serde_json::Value::Null;
        response_data["worker_is_verified"] = serde_json::Value::Null;
    }
    
    Ok(Json(ApiResponse::success(response_data)))
}

#[openapi(tag = "User")]
#[put("/user/profile", data = "<dto>")]
pub async fn update_profile(
    db: &State<DbConn>,
    auth: AuthGuard,
    dto: Json<UpdateProfileDto>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    // Validate inputs
    if let Some(ref email) = dto.email {
        if !validate_email(email) {
            return Err(ApiError::bad_request("Invalid email address"));
        }
    }
    
    if let Some(ref pincode) = dto.pincode {
        if !validate_pincode(pincode) {
            return Err(ApiError::bad_request("Invalid pincode"));
        }
    }
    
    // Build update document
    let mut update_doc = doc! {
        "updated_at": DateTime::now()
    };
    
    if let Some(ref name) = dto.name {
        update_doc.insert("name", name);
    }
    if let Some(ref email) = dto.email {
        update_doc.insert("email", email);
    }
    if let Some(ref city) = dto.city {
        update_doc.insert("city", city);
    }
    if let Some(ref pincode) = dto.pincode {
        update_doc.insert("pincode", pincode);
    }
    if let Some(ref profile_photo) = dto.profile_photo {
        update_doc.insert("profile_photo", profile_photo);
    }
    
    db.collection::<User>("users")
        .update_one(
            doc! { "_id": auth.user_id },
            doc! { "$set": update_doc },
            None
        )
        .await
        .map_err(|e| ApiError::internal_error(format!("Failed to update profile: {}", e)))?;
    
    // Fetch updated user
    let user = db.collection::<User>("users")
        .find_one(doc! { "_id": auth.user_id }, None)
        .await
        .map_err(|e| ApiError::internal_error(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::not_found("User not found"))?;
    
    // Check for active subscription
    let subscription = db.collection::<Subscription>("subscriptions")
        .find_one(
            doc! { 
                "user_id": auth.user_id,
                "status": "active"
            },
            None
        )
        .await
        .map_err(|e| ApiError::internal_error(format!("Database error: {}", e)))?;
    
    let user_response: UserResponse = user.into();
    
    // Build response with subscription info
    let mut response_data = serde_json::to_value(&user_response)
        .map_err(|e| ApiError::internal_error(format!("Serialization error: {}", e)))?;
    
    if let Some(sub) = subscription {
        response_data["subscription_id"] = serde_json::json!(sub.id.map(|id| id.to_hex()));
        response_data["subscription_plan"] = serde_json::json!(sub.plan_name);
        response_data["subscription_expires_at"] = serde_json::json!(sub.expires_at);
    } else {
        response_data["subscription_id"] = serde_json::Value::Null;
        response_data["subscription_plan"] = serde_json::Value::Null;
        response_data["subscription_expires_at"] = serde_json::Value::Null;
    }
    
    Ok(Json(ApiResponse::success_with_message(
        "Profile updated successfully".to_string(),
        response_data
    )))
}

#[openapi(tag = "User")]
#[post("/user/upload-photo", data = "<file>")]
pub async fn upload_profile_photo(
    mut file: TempFile<'_>,
    auth: AuthGuard,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    // Create uploads directory if it doesn't exist
    let upload_dir = "uploads/profiles";
    fs::create_dir_all(upload_dir)
        .await
        .map_err(|e| ApiError::internal_error(format!("Failed to create directory: {}", e)))?;
    
    // Generate unique filename
    let extension = file.content_type()
        .and_then(|ct| ct.extension())
        .map(|e| e.as_str())
        .unwrap_or("jpg");
    
    let filename = format!("{}_{}.{}", auth.user_id.to_hex(), chrono::Utc::now().timestamp(), extension);
    let filepath = format!("{}/{}", upload_dir, filename);
    
    // Save file
    file.persist_to(&filepath)
        .await
        .map_err(|e| ApiError::internal_error(format!("Failed to save file: {}", e)))?;
    
    let file_url = format!("/{}", filepath);
    
    Ok(Json(ApiResponse::success(serde_json::json!({
        "url": file_url,
        "message": "Photo uploaded successfully"
    }))))
}

#[openapi(tag = "User")]
#[put("/user/fcm-token", data = "<dto>")]
pub async fn update_fcm_token(
    db: &State<DbConn>,
    auth: AuthGuard,
    dto: Json<UpdateFcmTokenDto>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let field = if dto.platform == "android" {
        "fcm_token.android"
    } else if dto.platform == "ios" {
        "fcm_token.ios"
    } else {
        return Err(ApiError::bad_request("Invalid platform. Use 'android' or 'ios'"));
    };
    
    db.collection::<User>("users")
        .update_one(
            doc! { "_id": auth.user_id },
            doc! { "$set": { field: &dto.token } },
            None
        )
        .await
        .map_err(|e| ApiError::internal_error(format!("Failed to update token: {}", e)))?;
    
    Ok(Json(ApiResponse::success(serde_json::json!({
        "message": "FCM token updated successfully"
    }))))
}

#[openapi(tag = "User")]
#[delete("/user/account")]
pub async fn delete_account(
    db: &State<DbConn>,
    auth: AuthGuard,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    db.collection::<User>("users")
        .update_one(
            doc! { "_id": auth.user_id },
            doc! { "$set": { "is_active": false } },
            None
        )
        .await
        .map_err(|e| ApiError::internal_error(format!("Failed to deactivate account: {}", e)))?;
    
    Ok(Json(ApiResponse::success(serde_json::json!({
        "message": "Account deactivated successfully"
    }))))
}
use rocket::serde::json::Json;
use rocket::State;
use rocket_okapi::openapi;
use mongodb::bson::{doc, DateTime};
use mongodb::options::FindOptions;
use crate::db::DbConn;
use crate::models::{Kyc, SubmitKycDto, User, KycStatusEnum, KycStatus as UserKycStatus};
use crate::guards::AuthGuard;
use crate::utils::{ApiResponse, ApiError};

#[openapi(tag = "KYC")]
#[post("/kyc/submit", data = "<dto>")]
pub async fn submit_kyc(
    db: &State<DbConn>,
    auth: AuthGuard,
    dto: Json<SubmitKycDto>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    // Check if KYC already exists
    let existing_kyc = db.collection::<Kyc>("kycs")
        .find_one(doc! { "user_id": auth.user_id }, None)
        .await
        .map_err(|e| ApiError::internal_error(format!("Database error: {}", e)))?;
    
    if let Some(kyc) = existing_kyc {
        if matches!(kyc.status, KycStatusEnum::Approved) {
            return Err(ApiError::bad_request("KYC already approved"));
        }
        if matches!(kyc.status, KycStatusEnum::Submitted | KycStatusEnum::UnderReview) {
            return Err(ApiError::bad_request("KYC already submitted and under review"));
        }
        
        // Delete old KYC
        db.collection::<Kyc>("kycs")
            .delete_one(doc! { "_id": kyc.id }, None)
            .await
            .ok();
    }
    
    // Parse date of birth
    let dob = chrono::NaiveDate::parse_from_str(&dto.date_of_birth, "%Y-%m-%d")
        .map_err(|_| ApiError::bad_request("Invalid date format. Use YYYY-MM-DD"))?;
    
    let dob_datetime = DateTime::from_millis(dob.and_hms_opt(0, 0, 0).unwrap().and_utc().timestamp_millis());
    
    // Create new KYC
    let kyc = Kyc {
        id: None,
        user_id: auth.user_id,
        full_name: dto.full_name.clone(),
        date_of_birth: dob_datetime,
        address: dto.address.clone(),
        city: dto.city.clone(),
        state: dto.state.clone(),
        pincode: dto.pincode.clone(),
        document_type: dto.document_type.clone(),
        document_number: dto.document_number.clone(),
        document_front_image: dto.document_front_image.clone(),
        document_back_image: dto.document_back_image.clone(),
        selfie_image: dto.selfie_image.clone(),
        status: KycStatusEnum::Submitted,
        rejection_reason: None,
        verified_by: None,
        verified_at: None,
        created_at: DateTime::now(),
        updated_at: DateTime::now(),
    };
    
    let result = db.collection::<Kyc>("kycs")
        .insert_one(&kyc, None)
        .await
        .map_err(|e| ApiError::internal_error(format!("Failed to submit KYC: {}", e)))?;
    
    // Update user KYC status
    db.collection::<User>("users")
        .update_one(
            doc! { "_id": auth.user_id },
            doc! { "$set": { "kyc_status": "submitted" } },
            None
        )
        .await
        .ok();
    
    Ok(Json(ApiResponse::success_with_message(
        "KYC submitted successfully".to_string(),
        serde_json::json!({
            "kyc_id": result.inserted_id.as_object_id().unwrap().to_hex()
        })
    )))
}

#[openapi(tag = "KYC")]
#[get("/kyc/status")]
pub async fn get_kyc_status(
    db: &State<DbConn>,
    auth: AuthGuard,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let kyc = db.collection::<Kyc>("kycs")
        .find_one(doc! { "user_id": auth.user_id }, None)
        .await
        .map_err(|e| ApiError::internal_error(format!("Database error: {}", e)))?;
    
    match kyc {
        Some(kyc) => Ok(Json(ApiResponse::success(serde_json::json!({
            "kyc_exists": true,
            "status": format!("{:?}", kyc.status).to_lowercase(),
            "rejection_reason": kyc.rejection_reason,
            "submitted_at": kyc.created_at,
        })))),
        None => Ok(Json(ApiResponse::success(serde_json::json!({
            "kyc_exists": false,
            "status": "pending",
        })))),
    }
}

// Admin endpoints
#[derive(FromForm, serde::Deserialize, rocket_okapi::okapi::schemars::JsonSchema)]
pub struct KycListQuery {
    pub status: Option<String>,
    pub page: Option<i64>,
    pub limit: Option<i64>,
}

#[openapi(tag = "KYC")]
#[get("/kyc/admin/submissions?<query..>")]
pub async fn get_all_kyc_submissions(
    db: &State<DbConn>,
    _auth: AuthGuard, // TODO: Add admin guard
    query: KycListQuery,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let page = query.page.unwrap_or(1).max(1);
    let limit = query.limit.unwrap_or(20).min(100);
    let skip = (page - 1) * limit;
    
    let mut filter = doc! {};
    if let Some(status) = query.status {
        filter.insert("status", status);
    }
    
    let find_options = FindOptions::builder()
        .skip(skip as u64)
        .limit(limit)
        .sort(doc! { "created_at": -1 })
        .build();
    
    let mut cursor = db.collection::<Kyc>("kycs")
        .find(filter.clone(), find_options)
        .await
        .map_err(|e| ApiError::internal_error(format!("Database error: {}", e)))?;
    
    let mut submissions = Vec::new();
    while cursor.advance().await.map_err(|e| ApiError::internal_error(format!("Cursor error: {}", e)))? {
        let kyc = cursor.deserialize_current()
            .map_err(|e| ApiError::internal_error(format!("Deserialization error: {}", e)))?;
        submissions.push(kyc);
    }
    
    let total = db.collection::<Kyc>("kycs")
        .count_documents(filter, None)
        .await
        .map_err(|e| ApiError::internal_error(format!("Count error: {}", e)))?;
    
    Ok(Json(ApiResponse::success(serde_json::json!({
        "submissions": submissions,
        "pagination": {
            "page": page,
            "limit": limit,
            "total": total,
            "pages": (total as f64 / limit as f64).ceil() as i64,
        }
    }))))
}

#[openapi(tag = "KYC")]
#[get("/kyc/admin/<kyc_id>")]
pub async fn get_kyc_by_id(
    db: &State<DbConn>,
    _auth: AuthGuard, // TODO: Add admin guard
    kyc_id: String,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let object_id = mongodb::bson::oid::ObjectId::parse_str(&kyc_id)
        .map_err(|_| ApiError::bad_request("Invalid KYC ID"))?;
    
    let kyc = db.collection::<Kyc>("kycs")
        .find_one(doc! { "_id": object_id }, None)
        .await
        .map_err(|e| ApiError::internal_error(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::not_found("KYC not found"))?;
    
    Ok(Json(ApiResponse::success(serde_json::json!(kyc))))
}

#[derive(serde::Deserialize, rocket_okapi::okapi::schemars::JsonSchema)]
pub struct UpdateKycStatusDto {
    pub status: String, // "approved" or "rejected"
    pub rejection_reason: Option<String>,
}

#[openapi(tag = "KYC")]
#[put("/kyc/admin/<kyc_id>/status", data = "<dto>")]
pub async fn update_kyc_status(
    db: &State<DbConn>,
    auth: AuthGuard, // TODO: Add admin guard
    kyc_id: String,
    dto: Json<UpdateKycStatusDto>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let object_id = mongodb::bson::oid::ObjectId::parse_str(&kyc_id)
        .map_err(|_| ApiError::bad_request("Invalid KYC ID"))?;
    
    let kyc = db.collection::<Kyc>("kycs")
        .find_one(doc! { "_id": object_id }, None)
        .await
        .map_err(|e| ApiError::internal_error(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::not_found("KYC not found"))?;
    
    let status = match dto.status.as_str() {
        "approved" => "approved",
        "rejected" => "rejected",
        _ => return Err(ApiError::bad_request("Invalid status")),
    };
    
    let mut update_doc = doc! {
        "status": status,
        "verified_by": auth.user_id,
        "verified_at": DateTime::now(),
        "updated_at": DateTime::now(),
    };
    
    if let Some(ref reason) = dto.rejection_reason {
        update_doc.insert("rejection_reason", reason);
    }
    
    db.collection::<Kyc>("kycs")
        .update_one(
            doc! { "_id": object_id },
            doc! { "$set": update_doc },
            None
        )
        .await
        .map_err(|e| ApiError::internal_error(format!("Failed to update KYC: {}", e)))?;
    
    // Update user KYC status
    db.collection::<User>("users")
        .update_one(
            doc! { "_id": kyc.user_id },
            doc! { "$set": { "kyc_status": status } },
            None
        )
        .await
        .ok();
    
    Ok(Json(ApiResponse::success(serde_json::json!({
        "message": format!("KYC {} successfully", status)
    }))))
}
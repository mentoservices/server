use rocket::serde::json::Json;
use rocket::State;
use mongodb::bson::{doc, DateTime, oid::ObjectId as ObjectId};
use crate::db::DbConn;
use crate::models::{
    SendOtpDto, VerifyOtpDto, ResendOtpDto,
    User, KycStatus, UserResponse,
};
use crate::services::{JwtService, msg91::Msg91Service};
use crate::utils::{validate_mobile, validate_email, ApiResponse, ApiError};

const OTP_WINDOW_MS: i64 = 10 * 60 * 1000;
const OTP_LIMIT: i32 = 3;
const REFRESH_LIMIT: i32 = 10;
const REFRESH_WINDOW_MS: i64 = 60 * 1000;


/// --------------------
/// Rate limiter helper
/// --------------------
async fn rate_limit(
    db: &DbConn,
    key: &str,
    limit: i32,
    window_ms: i64,
) -> Result<(), ApiError> {
    let now = chrono::Utc::now().timestamp_millis();
    let window_expires = DateTime::from_millis(now + window_ms);

    let collection = db.collection::<mongodb::bson::Document>("rate_limits");

    let doc = collection
        .find_one(doc! { "key": key }, None)
        .await
        .map_err(|_| ApiError::internal_error("Rate limiter lookup failed"))?;

    match doc {
        // First request OR expired window
        None => {
            collection
                .insert_one(
                    doc! {
                        "key": key,
                        "count": 1,
                        "expires_at": window_expires
                    },
                    None,
                )
                .await
                .map_err(|_| ApiError::internal_error("Rate limiter insert failed"))?;
            Ok(())
        }

        Some(d) => {
            let count = d.get_i32("count").unwrap_or(0);
            let expires_at = d.get_datetime("expires_at").ok();

            // Window expired → reset
            if expires_at.map(|e| *e < DateTime::now()).unwrap_or(true) {
                collection
                    .update_one(
                        doc! { "key": key },
                        doc! {
                            "$set": {
                                "count": 1,
                                "expires_at": window_expires
                            }
                        },
                        None,
                    )
                    .await
                    .map_err(|_| ApiError::internal_error("Rate limiter reset failed"))?;
                return Ok(());
            }

            // Limit exceeded
            if count >= limit {
                return Err(ApiError::too_many_requests(
                    "Too many requests. Please try later.",
                ));
            }

            // Increment count
            collection
                .update_one(
                    doc! { "key": key },
                    doc! { "$inc": { "count": 1 } },
                    None,
                )
                .await
                .map_err(|_| ApiError::internal_error("Rate limiter increment failed"))?;

            Ok(())
        }
    }
}

/// --------------------
/// Send OTP
/// --------------------
#[post("/auth/send-otp", data = "<dto>")]
pub async fn send_otp(
    db: &State<DbConn>,
    dto: Json<SendOtpDto>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    if !validate_mobile(&dto.mobile) {
        return Err(ApiError::bad_request("Invalid mobile number"));
    }
    if !validate_email(&dto.email) {
        return Err(ApiError::bad_request("Invalid email"));
    }

    rate_limit(
        db,
        &format!("send_otp:{}", dto.mobile),
        OTP_LIMIT,
        OTP_WINDOW_MS,
    ).await?;

    Msg91Service::send_otp(&dto.mobile)
        .await
        .map_err(|_| ApiError::internal_error("Failed to send OTP"))?;

    Ok(Json(ApiResponse::success(serde_json::json!({
        "message": "OTP sent successfully"
    }))))
}

/// --------------------
/// Resend OTP
/// --------------------
#[post("/auth/resend-otp", data = "<dto>")]
pub async fn resend_otp(
    db: &State<DbConn>,
    dto: Json<ResendOtpDto>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    if !validate_mobile(&dto.mobile) {
        return Err(ApiError::bad_request("Invalid mobile number"));
    }

    rate_limit(
        db,
        &format!("resend_otp:{}", dto.mobile),
        OTP_LIMIT,
        OTP_WINDOW_MS,
    ).await?;

    Msg91Service::send_otp(&dto.mobile)
        .await
        .map_err(|_| ApiError::internal_error("Failed to resend OTP"))?;

    Ok(Json(ApiResponse::success(serde_json::json!({
        "message": "OTP resent successfully"
    }))))
}

/// --------------------
/// Verify OTP + Login
/// --------------------
#[post("/auth/verify-otp", data = "<dto>")]
pub async fn verify_otp(
    db: &State<DbConn>,
    dto: Json<VerifyOtpDto>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    Msg91Service::verify_otp(&dto.mobile, &dto.otp)
        .await
        .map_err(|_| ApiError::unauthorized("Invalid OTP"))?;

    let user = db
        .collection::<User>("users")
        .find_one(doc! { "mobile": &dto.mobile }, None)
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;

    let (user, is_new_user) = match user {
        Some(mut u) => {
            u.last_login_at = DateTime::now();
            db.collection::<User>("users")
                .update_one(
                    doc! { "_id": u.id },
                    doc! { "$set": { "last_login_at": DateTime::now() } },
                    None,
                ).await.ok();
            (u, false)
        }
        None => {
            let user = User {
                id: None,
                mobile: dto.mobile.clone(),
                email: None,
                name: None,
                profile_photo: None,
                city: None,
                pincode: None,
                kyc_status: KycStatus::Pending,
                is_active: true,
                fcm_token: None,
                last_login_at: DateTime::now(),
                created_at: DateTime::now(),
                updated_at: DateTime::now(),
            };

            let res = db.collection::<User>("users")
                .insert_one(&user, None)
                .await
                .map_err(|e| ApiError::internal_error(e.to_string()))?;

            let mut u = user;
            u.id = Some(res.inserted_id.as_object_id().unwrap());
            (u, true)
        }
    };

    let access_token = JwtService::generate_access_token(
    user.id.as_ref().unwrap(),
    &user.mobile,
)
.map_err(|e| ApiError::internal_error(e.to_string()))?;

let refresh_token = JwtService::generate_refresh_token(
    user.id.as_ref().unwrap(),
    &user.mobile,
)
.map_err(|e| ApiError::internal_error(e.to_string()))?;

    Ok(Json(ApiResponse::success(serde_json::json!({
        "message": if is_new_user { "Registration successful" } else { "Login successful" },
        "isNewUser": is_new_user,
        "user": UserResponse::from(user),
        "accessToken": access_token,
        "refreshToken": refresh_token
    }))))
}

/// --------------------
/// Silent Refresh Token
/// --------------------
#[derive(serde::Deserialize)]
pub struct RefreshTokenDto {
    pub refresh_token: String,
}

#[post("/auth/refresh", data = "<dto>")]
pub async fn refresh_token(
    db: &State<DbConn>,
    dto: Json<RefreshTokenDto>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    rate_limit(
        db,
        "refresh_token",
        REFRESH_LIMIT,
        REFRESH_WINDOW_MS,
    ).await?;

    let claims = JwtService::verify_token(&dto.refresh_token, true)
        .map_err(|_| ApiError::unauthorized("Invalid refresh token"))?;

    let user_id = ObjectId::parse_str(&claims.sub)
    .map_err(|_| ApiError::unauthorized("Invalid user id in token"))?;

    let access = JwtService::generate_access_token(&user_id, &claims.mobile)
    .map_err(|e| ApiError::internal_error(e.to_string()))?;

    Ok(Json(ApiResponse::success(serde_json::json!({
        "accessToken": access
    }))))
}

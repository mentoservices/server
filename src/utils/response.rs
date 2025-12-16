use rocket_okapi::okapi::Map;
use serde::{Deserialize, Serialize};
use rocket::http::Status;
use rocket::response::{self, Responder, Response};
use rocket::Request;
use std::io::Cursor;
use rocket_okapi::okapi::schemars::JsonSchema;
use rocket_okapi::response::OpenApiResponderInner;
use rocket_okapi::r#gen::OpenApiGenerator;
use rocket_okapi::okapi::openapi3::{MediaType, Response as OpenApiResponse, Responses};

/// -----------------------------
/// Generic API response
/// -----------------------------
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        ApiResponse {
            success: true,
            message: None,
            data: Some(data),
        }
    }

    pub fn success_with_message(message: String, data: T) -> Self {
        ApiResponse {
            success: true,
            message: Some(message),
            data: Some(data),
        }
    }

    pub fn error(message: String) -> ApiResponse<()> {
        ApiResponse {
            success: false,
            message: Some(message),
            data: None,
        }
    }
}

/// -----------------------------
/// API Error
/// -----------------------------
#[derive(Debug, Serialize, JsonSchema)]
pub struct ApiError {
    #[schemars(skip)]
    #[serde(skip_serializing)]
    pub status: Status,
    pub message: String,
}

impl ApiError {
    pub fn bad_request(message: impl Into<String>) -> Self {
        ApiError {
            status: Status::BadRequest,
            message: message.into(),
        }
    }

    pub fn unauthorized(message: impl Into<String>) -> Self {
        ApiError {
            status: Status::Unauthorized,
            message: message.into(),
        }
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        ApiError {
            status: Status::NotFound,
            message: message.into(),
        }
    }

    pub fn too_many_requests(message: impl Into<String>) -> Self {
        ApiError {
            status: Status::TooManyRequests, // ✅ 429
            message: message.into(),
        }
    }

    pub fn internal_error(message: impl Into<String>) -> Self {
        ApiError {
            status: Status::InternalServerError,
            message: message.into(),
        }
    }
}

/// -----------------------------
/// Rocket Responder
/// -----------------------------
impl<'r> Responder<'r, 'static> for ApiError {
    fn respond_to(self, _: &'r Request<'_>) -> response::Result<'static> {
        let body = serde_json::to_string(&ApiResponse::<()>::error(self.message))
            .unwrap_or_else(|_| r#"{"success":false,"message":"Internal error"}"#.to_string());

        Response::build()
            .status(self.status)
            .header(rocket::http::ContentType::JSON)
            .sized_body(body.len(), Cursor::new(body))
            .ok()
    }
}

/// -----------------------------
/// OpenAPI integration
/// -----------------------------
impl OpenApiResponderInner for ApiError {
    fn responses(generator: &mut OpenApiGenerator) -> rocket_okapi::Result<Responses> {
        let schema = generator.json_schema::<ApiResponse<()>>();

        let mut content = Map::new();
        content.insert(
            "application/json".to_owned(),
            MediaType {
                schema: Some(schema),
                ..Default::default()
            },
        );

        let mut responses = Responses::default();

        for (code, description) in [
            ("400", "Bad request"),
            ("401", "Unauthorized"),
            ("404", "Not found"),
            ("429", "Too many requests"),
            ("500", "Internal server error"),
        ] {
            responses.responses.insert(
                code.to_string(),
                rocket_okapi::okapi::openapi3::RefOr::Object(OpenApiResponse {
                    description: description.to_string(),
                    content: content.clone(),
                    ..Default::default()
                }),
            );
        }

        Ok(responses)
    }
}

use rocket::request::{self, FromRequest, Request, Outcome};
use rocket::http::Status;
use mongodb::bson::oid::ObjectId;

// === OpenAPI (compatible with rocket_okapi 0.8.0 / 0.8.1) ===
use rocket_okapi::request::{OpenApiFromRequest, RequestHeaderInput};
use rocket_okapi::r#gen::OpenApiGenerator;

/// JWT-based authentication guard
pub struct AuthGuard {
    pub user_id: ObjectId,
    pub mobile: String,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for AuthGuard {
    type Error = ();

    async fn from_request(req: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        let token = req.headers().get_one("Authorization");

        match token {
            Some(token) => {
                let token = token.trim_start_matches("Bearer ");

                match crate::services::JwtService::verify_token(token, false) {
                    Ok(claims) => match ObjectId::parse_str(&claims.sub) {
                        Ok(user_id) => Outcome::Success(AuthGuard {
                            user_id,
                            mobile: claims.mobile,
                        }),
                        Err(_) => Outcome::Error((Status::Unauthorized, ())),
                    },
                    Err(_) => Outcome::Error((Status::Unauthorized, ())),
                }
            }
            None => Outcome::Error((Status::Unauthorized, ())),
        }
    }
}

/// === OpenAPI Integration (Fallback for older versions) ===
/// Keeps OpenAPI generation working even without new traits.
impl<'a> OpenApiFromRequest<'a> for AuthGuard {
    fn from_request_input(
        _gen: &mut OpenApiGenerator,
        _name: String,
        _required: bool,
    ) -> rocket_okapi::Result<RequestHeaderInput> {
        // The guard doesn't contribute any special header/parameter for docs
        Ok(RequestHeaderInput::None)
    }
}

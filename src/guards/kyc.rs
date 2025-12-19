use rocket::request::{self, Request, FromRequest, Outcome};
use rocket::http::Status;
use rocket::State;
use crate::db::DbConn;
use crate::guards::AuthGuard;
use mongodb::bson::doc;
use rocket_okapi::request::OpenApiFromRequest;
use rocket_okapi::r#gen::OpenApiGenerator;
use rocket_okapi::request::RequestHeaderInput;

pub struct KycGuard {
    pub auth: AuthGuard,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for KycGuard {
    type Error = ();

    async fn from_request(req: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        let auth_outcome = req.guard::<AuthGuard>().await;
        
        match auth_outcome {
            Outcome::Success(auth) => {
                let db = req.guard::<&State<DbConn>>().await.unwrap();
                
                let user = db.collection::<crate::models::User>("users")
                    .find_one(doc! { "_id": &auth.user_id }, None)
                    .await;
                
                match user {
                    Ok(Some(user)) => {
                        // Allow both Approved AND Submitted KYC status
                        // This lets users proceed while their KYC is under review
                        if matches!(
                            user.kyc_status, 
                            crate::models::KycStatus::Approved | crate::models::KycStatus::Submitted
                        ) {
                            Outcome::Success(KycGuard { auth })
                        } else {
                            // Log the actual status for debugging
                            println!("KYC Guard rejected - status: {:?}", user.kyc_status);
                            Outcome::Error((Status::Forbidden, ()))
                        }
                    }
                    Ok(None) => {
                        println!("KYC Guard rejected - user not found");
                        Outcome::Error((Status::Forbidden, ()))
                    }
                    Err(e) => {
                        println!("KYC Guard rejected - DB error: {:?}", e);
                        Outcome::Error((Status::Forbidden, ()))
                    }
                }
            }
            Outcome::Error(e) => Outcome::Error(e),
            Outcome::Forward(f) => Outcome::Forward(f),
        }
    }
}

impl<'a> OpenApiFromRequest<'a> for KycGuard {
    fn from_request_input(
        _gen: &mut OpenApiGenerator,
        _name: String,
        _required: bool,
    ) -> rocket_okapi::Result<RequestHeaderInput> {
        Ok(RequestHeaderInput::None)
    }
}
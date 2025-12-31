#[macro_use]
extern crate rocket;

mod config;
mod db;
mod guards;
mod models;
mod routes;
mod services;
mod utils;

use dotenvy::dotenv;
use rocket::fairing::{Fairing, Info, Kind};
use rocket::fs::FileServer;
use rocket::http::Header;
use rocket::{Build, Request, Response, Rocket};
use rocket_okapi::swagger_ui::{SwaggerUIConfig, make_swagger_ui};

/* ----------------------------- CORS ----------------------------- */

pub struct CORS;

#[rocket::async_trait]
impl Fairing for CORS {
    fn info(&self) -> Info {
        Info {
            name: "CORS",
            kind: Kind::Response,
        }
    }

    async fn on_response<'r>(&self, request: &'r Request<'_>, response: &mut Response<'r>) {
        if let Some(origin) = request.headers().get_one("Origin") {
            response.set_header(Header::new("Access-Control-Allow-Origin", origin));
        }

        response.set_header(Header::new(
            "Access-Control-Allow-Methods",
            "GET, POST, PUT, DELETE, OPTIONS",
        ));

        response.set_header(Header::new(
            "Access-Control-Allow-Headers",
            "Content-Type, Authorization",
        ));

        response.set_header(Header::new("Access-Control-Allow-Credentials", "true"));
    }
}

/* ----------------------------- OPTIONS ----------------------------- */

#[options("/<_..>")]
fn options_handler() {}

/* ----------------------------- ERRORS ----------------------------- */

#[catch(404)]
fn not_found() -> rocket::serde::json::Value {
    rocket::serde::json::json!({
        "success": false,
        "message": "Resource not found (check /api/v1 prefix)"
    })
}

#[catch(500)]
fn internal_error() -> rocket::serde::json::Value {
    rocket::serde::json::json!({
        "success": false,
        "message": "Internal server error"
    })
}

/* ----------------------------- SWAGGER ----------------------------- */

fn swagger_config() -> SwaggerUIConfig {
    SwaggerUIConfig {
        url: "/openapi.json".to_string(),
        ..Default::default()
    }
}

/* ----------------------------- LAUNCH ----------------------------- */

#[launch]
fn rocket() -> Rocket<Build> {
    dotenv().ok();
    env_logger::init();
    println!(
        "MSG91_TEMPLATE_ID = {:?}",
        std::env::var("MSG91_TEMPLATE_ID")
    );

    println!("🚀 Mento API running");
    println!("📚 Swagger UI → http://localhost:8000/api/docs");

    rocket::build()
        .attach(db::init())
        .attach(CORS)
        .mount("/", routes![options_handler])
        .mount(
            "/api/v1",
            routes![
                // Auth
                routes::auth::send_otp,
                routes::auth::resend_otp,
                routes::auth::verify_otp,
                routes::auth::refresh_token,
                // User
                routes::user::get_profile,
                routes::user::update_profile,
                routes::user::upload_profile_photo,
                routes::user::update_fcm_token,
                routes::user::delete_account,
                // KYC
                routes::kyc::submit_kyc,
                routes::kyc::get_kyc_status,
                routes::kyc::get_all_kyc_submissions,
                routes::kyc::get_kyc_by_id,
                routes::kyc::update_kyc_status,
                // Subscription (NEW)
                routes::worker::create_subscription,
                routes::worker::verify_subscription_payment,
                routes::worker::get_subscription_status,
                // Worker
                routes::worker::create_worker_profile,
                routes::worker::get_worker_profile,
                routes::worker::get_worker_profile_by_id,
                routes::worker::update_worker_profile,
                routes::worker::search_workers,
                routes::worker::find_nearby_workers,
                routes::worker::update_worker_location,
                // Categories
                routes::category::get_all_categories,
                routes::category::get_subcategories,
                // Services
                routes::service::get_all_services,
                routes::service::get_services_by_category,
                routes::service::get_all_categories,
                routes::service::search_services,
                routes::service::get_service_by_id,
                // Uploads
                routes::file_upload::upload_image,
                routes::file_upload::upload_document,
                routes::file_upload::upload_document_base64,
                // Reviews
                routes::review::create_review,
                routes::review::get_worker_reviews,
                routes::review::delete_review,
                // Job Seeker Subscription
                routes::job::create_job_seeker_subscription,
                routes::job::verify_job_seeker_payment,
                routes::job::get_job_seeker_subscription_status,
                // Job Seeker Profile
                routes::job::create_job_seeker_profile,
                routes::job::get_job_seeker_profile,
                routes::job::get_job_seeker_profile_by_id,
                routes::job::update_job_seeker_profile,
                routes::job::search_job_seekers,
                routes::job::delete_job_seeker_profile,
                // Admin Routes - Workers
                routes::admin::get_all_workers,
                routes::admin::verify_worker,
                // Admin Routes - Job Seekers
                routes::admin::get_all_job_seekers,
                routes::admin::verify_job_seeker,
                // Admin Routes - Categories
                routes::admin::create_category,
                routes::admin::update_category,
                routes::admin::delete_category,
                // Admin Routes - Subcategories
                routes::admin::create_subcategory,
                routes::admin::update_subcategory,
                routes::admin::delete_subcategory,
                // Admin Routes - Jobs
                routes::admin::get_all_jobs,
                routes::admin::update_job_status,
                routes::admin::delete_job,
            ],
        )
        .mount("/uploads", FileServer::from("uploads"))
        .mount("/api/docs", make_swagger_ui(&swagger_config()))
        .register("/", catchers![not_found, internal_error])
}

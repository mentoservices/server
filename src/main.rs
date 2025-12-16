#[macro_use]
extern crate rocket;

mod config;
mod db;
mod models;
mod routes;
mod services;
mod guards;
mod utils;

use rocket::{Build, Rocket, Request, Response};
use rocket::fairing::{Fairing, Info, Kind};
use dotenvy::dotenv;
use rocket::http::Header;
use rocket::fs::FileServer;
use std::path::Path;
use std::fs;
use rocket_okapi::swagger_ui::{make_swagger_ui, SwaggerUIConfig};

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

    async fn on_response<'r>(
        &self,
        request: &'r Request<'_>,
        response: &mut Response<'r>,
    ) {
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


fn ensure_upload_dirs() {
    let dirs = [
        "uploads",
    ];

    for dir in dirs {
        if !Path::new(dir).exists() {
            if let Err(e) = fs::create_dir_all(dir) {
                eprintln!("⚠️ Failed to create directory {}: {}", dir, e);
            }
        }
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

// health check
#[get("/health")]
fn health() -> &'static str {
    "OK"
}


#[launch]
fn rocket() -> Rocket<Build> {
    dotenv().ok();
    env_logger::init();
    println!(
    "MSG91_TEMPLATE_ID = {:?}",
    std::env::var("MSG91_TEMPLATE_ID")
);


    ensure_upload_dirs(); //ensures that upload dir is ther
    println!("🚀 Mento API running");
    println!("📚 Swagger UI → http://localhost:8000/api/docs");

    rocket::build()
        .attach(db::init())
        .attach(CORS)

        .mount("/", routes![options_handler])

        .mount(
            "/api/v1",
            routes![

                // health check
                health,

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

                // Jobs
                routes::job::create_job,
                routes::job::get_jobs,
                routes::job::get_job_by_id,
                routes::job::get_my_posted_jobs,
                routes::job::apply_to_job,
                routes::job::update_job_status,
                routes::job::delete_job,

                // Categories
                routes::category::get_all_categories,
                routes::category::get_subcategories,

                // Uploads
                routes::file_upload::upload_image,
                routes::file_upload::upload_document,
                routes::file_upload::upload_document_base64,

                // Reviews
                routes::review::create_review,
                routes::review::get_worker_reviews,
                routes::review::delete_review,
            ],
        )

        .mount("/uploads", FileServer::from("uploads"))
        .mount("/api/docs", make_swagger_ui(&swagger_config()))
        .register("/", catchers![not_found, internal_error])
}
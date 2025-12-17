use rocket::figment::{Figment, providers::{Env, Format, Toml}};
use rocket::Config as RocketConfig;
use std::env;

pub struct Config;

impl Config {
    fn figment() -> Figment {
        // Get the current profile
        let profile = env::var("ROCKET_PROFILE").unwrap_or_else(|_| "development".to_string());
        
        Figment::from(RocketConfig::default())
            .merge(Toml::file("Rocket.toml").nested())
            .select(&profile)
            .merge(Env::prefixed("ROCKET_").split("_"))
    }

    pub fn jwt_secret() -> String {
        Self::figment()
            .extract_inner("jwt_secret")
            .unwrap_or_else(|_| "default-secret".to_string())
    }

    pub fn jwt_refresh_secret() -> String {
        Self::figment()
            .extract_inner("jwt_refresh_secret")
            .unwrap_or_else(|_| "default-refresh-secret".to_string())
    }

    pub fn jwt_expiry() -> i64 {
        Self::figment()
            .extract_inner("jwt_expiry")
            .unwrap_or(900)
    }

    pub fn jwt_refresh_expiry() -> i64 {
        Self::figment()
            .extract_inner("jwt_refresh_expiry")
            .unwrap_or(604800)
    }

    pub fn mongodb_uri() -> String {
        Self::figment()
            .extract_inner("mongodb_uri")
            .unwrap_or_else(|_| "mongodb://localhost:27017/mento-services".to_string())
    }

    pub fn mail_host() -> String {
        Self::figment()
            .extract_inner("mail_host")
            .unwrap_or_else(|_| "smtp.gmail.com".to_string())
    }

    pub fn mail_port() -> u16 {
        Self::figment()
            .extract_inner("mail_port")
            .unwrap_or(587)
    }

    pub fn mail_user() -> String {
        Self::figment()
            .extract_inner("mail_user")
            .unwrap_or_default()
    }

    pub fn mail_password() -> String {
        Self::figment()
            .extract_inner("mail_password")
            .unwrap_or_default()
    }

    pub fn mail_from() -> String {
        Self::figment()
            .extract_inner("mail_from")
            .unwrap_or_else(|_| "Mento Services <noreply@mentoservices.com>".to_string())
    }

    pub fn is_development() -> bool {
        let profile = env::var("ROCKET_PROFILE").unwrap_or_else(|_| "development".to_string());
        profile == "development"
    }

    pub fn razorpay_key_id() -> Option<String> {
        Self::figment()
            .extract_inner("razorpay_key_id")
            .ok()
    }

    pub fn razorpay_key_secret() -> Option<String> {
        Self::figment()
            .extract_inner("razorpay_key_secret")
            .ok()
    }

    pub fn is_razorpay_enabled() -> bool {
        Self::razorpay_key_id().is_some()
            && Self::razorpay_key_secret().is_some()
    }

    pub fn msg91_auth_key() -> Option<String> {
        Self::figment()
            .extract_inner("msg91_auth_key")
            .ok()
    }

    pub fn msg91_sender_id() -> Option<String> {
        Self::figment()
            .extract_inner("msg91_sender_id")
            .ok()
    }

    pub fn msg91_template_id() -> Option<String> {
        Self::figment()
            .extract_inner("msg91_template_id")
            .ok()
    }

    pub fn is_msg91_enabled() -> bool {
        Self::msg91_auth_key().is_some()
            && Self::msg91_template_id().is_some()
    }
}
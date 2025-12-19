use std::env;

pub struct Config;

impl Config {
    pub fn jwt_secret() -> String {
        env::var("JWT_SECRET").unwrap_or_else(|_| "default-secret".to_string())
    }

    pub fn jwt_refresh_secret() -> String {
        env::var("JWT_REFRESH_SECRET").unwrap_or_else(|_| "default-refresh-secret".to_string())
    }

    pub fn jwt_expiry() -> i64 {
        env::var("JWT_EXPIRY")
            .unwrap_or_else(|_| "900".to_string())
            .parse()
            .unwrap_or(900)
    }

    pub fn jwt_refresh_expiry() -> i64 {
        env::var("JWT_REFRESH_EXPIRY")
            .unwrap_or_else(|_| "604800".to_string())
            .parse()
            .unwrap_or(604800)
    }

    pub fn mongodb_uri() -> String {
        env::var("MONGODB_URI")
            .unwrap_or_else(|_| "mongodb://localhost:27017/mento-services".to_string())
    }

    pub fn mail_host() -> String {
        env::var("MAIL_HOST").unwrap_or_else(|_| "smtp.gmail.com".to_string())
    }

    pub fn mail_port() -> u16 {
        env::var("MAIL_PORT")
            .unwrap_or_else(|_| "587".to_string())
            .parse()
            .unwrap_or(587)
    }

    pub fn mail_user() -> String {
        env::var("MAIL_USER").unwrap_or_default()
    }

    pub fn mail_password() -> String {
        env::var("MAIL_PASSWORD").unwrap_or_default()
    }

    pub fn mail_from() -> String {
        env::var("MAIL_FROM").unwrap_or_else(|_| "Mento Services <noreply@mentoservices.com>".to_string())
    }

    pub fn is_development() -> bool {
        env::var("ROCKET_ENV").unwrap_or_default() == "development"
    }
}
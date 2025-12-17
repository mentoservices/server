use reqwest::Client;
use serde_json::json;

use crate::config::Config;

const MSG91_BASE: &str = "https://control.msg91.com/api/v5/otp";

pub struct Msg91Service;

impl Msg91Service {
    fn client() -> Client {
        Client::new()
    }

    fn auth_key() -> Result<String, String> {
        Config::msg91_auth_key()
            .ok_or_else(|| "MSG91_AUTH_KEY not configured".to_string())
    }

    fn template_id() -> Result<String, String> {
        Config::msg91_template_id()
            .ok_or_else(|| "MSG91_TEMPLATE_ID not configured".to_string())
    }

    /// Send OTP
    pub async fn send_otp(mobile: &str) -> Result<(), String> {
        // Optional safety check
        if !Config::is_msg91_enabled() {
            return Err("MSG91 is not enabled".to_string());
        }

        let body = json!({
            "template_id": Self::template_id()?,
            "mobile": format!("91{}", mobile),
            "authkey": Self::auth_key()?,
        });

        let res = Self::client()
            .post(MSG91_BASE)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("MSG91 request failed: {}", e))?;

        if !res.status().is_success() {
            return Err(res.text().await.unwrap_or_else(|_| "MSG91 error".to_string()));
        }

        Ok(())
    }

    /// Verify OTP
    pub async fn verify_otp(mobile: &str, otp: &str) -> Result<(), String> {
        if !Config::is_msg91_enabled() {
            return Err("MSG91 is not enabled".to_string());
        }

        let url = format!(
            "{}/verify?mobile=91{}&otp={}&authkey={}",
            MSG91_BASE,
            mobile,
            otp,
            Self::auth_key()?
        );

        let res = Self::client()
            .post(url)
            .send()
            .await
            .map_err(|e| format!("MSG91 request failed: {}", e))?;

        if !res.status().is_success() {
            return Err(res.text().await.unwrap_or_else(|_| "MSG91 verification failed".to_string()));
        }

        Ok(())
    }
}

use reqwest::Client;
use serde::{Deserialize};
use serde_json::json;

const MSG91_BASE: &str = "https://control.msg91.com/api/v5/otp";

pub struct Msg91Service;

#[derive(Debug, Deserialize)]
struct Msg91VerifyResponse {
    #[serde(rename = "type")]
    response_type: String, // "success" or "error"
    message: String,
}

impl Msg91Service {
    fn client() -> Client {
        Client::new()
    }

    fn auth_key() -> String {
        std::env::var("MSG91_AUTH_KEY")
            .expect("MSG91_AUTH_KEY not set")
    }

    fn template_id() -> String {
        std::env::var("MSG91_TEMPLATE_ID")
            .expect("MSG91_TEMPLATE_ID not set")
    }

    /// Send OTP
    pub async fn send_otp(mobile: &str) -> Result<(), String> {
        let body = json!({
            "template_id": Self::template_id(),
            "mobile": format!("91{}", mobile),
            "authkey": Self::auth_key()
        });

        let res = Self::client()
            .post(MSG91_BASE)
            .json(&body)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !res.status().is_success() {
            return Err(res.text().await.unwrap_or_else(|_| "MSG91 send OTP failed".to_string()));
        }

        Ok(())
    }

    /// Verify OTP
    pub async fn verify_otp(mobile: &str, otp: &str) -> Result<(), String> {
        let url = format!(
            "{}/verify?mobile=91{}&otp={}&authkey={}",
            MSG91_BASE,
            mobile,
            otp,
            Self::auth_key()
        );

        let res = Self::client()
            .post(&url)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        let body: Msg91VerifyResponse = res
            .json()
            .await
            .map_err(|e| e.to_string())?;

        if body.response_type != "success" {
            return Err(body.message);
        }

        Ok(())
    }
}

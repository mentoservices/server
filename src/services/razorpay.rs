use reqwest::Client;
use serde_json::json;

use crate::config::Config;

pub struct RazorpayService;

impl RazorpayService {
    fn key_id() -> Result<String, String> {
        Config::razorpay_key_id()
            .ok_or_else(|| "RAZORPAY_KEY_ID not configured".to_string())
    }

    fn key_secret() -> Result<String, String> {
        Config::razorpay_key_secret()
            .ok_or_else(|| "RAZORPAY_KEY_SECRET not configured".to_string())
    }

    /// Create Razorpay order
    /// `amount` is expected in INR (e.g. 499)
    pub async fn create_order(amount: i64) -> Result<serde_json::Value, String> {
        if amount <= 0 {
            return Err("Amount must be greater than zero".to_string());
        }

        let client = Client::new();

        let res = client
            .post("https://api.razorpay.com/v1/orders")
            .basic_auth(Self::key_id()?, Some(Self::key_secret()?))
            .json(&json!({
                "amount": amount * 100, // Razorpay expects paise
                "currency": "INR",
                "payment_capture": 1
            }))
            .send()
            .await
            .map_err(|e| format!("Razorpay request failed: {}", e))?;

        if !res.status().is_success() {
            return Err(res.text().await.unwrap_or_else(|_| "Razorpay error".to_string()));
        }

        res.json()
            .await
            .map_err(|e| format!("Invalid Razorpay response: {}", e))
    }
}

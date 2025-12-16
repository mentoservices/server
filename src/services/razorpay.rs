use reqwest::Client;
use serde_json::json;

pub struct RazorpayService;

impl RazorpayService {
    fn key_id() -> String {
        std::env::var("RAZORPAY_KEY_ID").unwrap()
    }

    fn key_secret() -> String {
        std::env::var("RAZORPAY_KEY_SECRET").unwrap()
    }

    pub async fn create_order(amount: i64) -> Result<serde_json::Value, String> {
        let client = Client::new();

        let res = client
            .post("https://api.razorpay.com/v1/orders")
            .basic_auth(Self::key_id(), Some(Self::key_secret()))
            .json(&json!({
                "amount": amount * 100,
                "currency": "INR",
                "payment_capture": 1
            }))
            .send()
            .await
            .map_err(|e| e.to_string())?;

        Ok(res.json().await.map_err(|e| e.to_string())?)
    }
}

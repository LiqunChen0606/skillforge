use reqwest::Client;
use serde::{Deserialize, Serialize};

const DEFAULT_BASE_URL: &str = "https://api.anthropic.com";
const API_VERSION: &str = "2023-06-01";

#[derive(Debug)]
pub enum ApiError {
    MissingApiKey,
    Http(reqwest::Error),
    Api { status: u16, message: String },
    Parse(String),
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingApiKey => write!(f, "API key is required"),
            Self::Http(e) => write!(f, "HTTP error: {}", e),
            Self::Api { status, message } => write!(f, "API error ({}): {}", status, message),
            Self::Parse(msg) => write!(f, "Parse error: {}", msg),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Assistant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApiResponse {
    pub content: Vec<ContentBlock>,
    pub stop_reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ContentBlock {
    #[serde(rename = "type")]
    pub block_type: String,
    pub text: Option<String>,
}

impl ApiResponse {
    pub fn text(&self) -> String {
        self.content
            .iter()
            .filter_map(|b| b.text.as_deref())
            .collect::<Vec<_>>()
            .join("")
    }
}

#[derive(Debug)]
pub struct AnthropicClient {
    api_key: String,
    model: String,
    base_url: String,
    client: Client,
}

impl AnthropicClient {
    pub fn new(
        api_key: &str,
        model: &str,
        base_url: Option<&str>,
    ) -> Result<Self, ApiError> {
        if api_key.is_empty() {
            return Err(ApiError::MissingApiKey);
        }
        Ok(Self {
            api_key: api_key.to_string(),
            model: model.to_string(),
            base_url: base_url.unwrap_or(DEFAULT_BASE_URL).to_string(),
            client: Client::new(),
        })
    }

    pub fn model(&self) -> &str {
        &self.model
    }

    pub fn build_request_body(
        &self,
        system: Option<&str>,
        messages: &[Message],
        max_tokens: u32,
    ) -> String {
        let mut body = serde_json::json!({
            "model": self.model,
            "max_tokens": max_tokens,
            "messages": messages,
        });
        if let Some(sys) = system {
            body["system"] = serde_json::Value::String(sys.to_string());
        }
        serde_json::to_string(&body).unwrap()
    }

    pub async fn send(
        &self,
        system: Option<&str>,
        messages: &[Message],
        max_tokens: u32,
    ) -> Result<ApiResponse, ApiError> {
        let url = format!("{}/v1/messages", self.base_url);
        let body = self.build_request_body(system, messages, max_tokens);

        let response = self
            .client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", API_VERSION)
            .header("content-type", "application/json")
            .body(body)
            .send()
            .await
            .map_err(ApiError::Http)?;

        let status = response.status().as_u16();
        if status != 200 {
            let text = response.text().await.unwrap_or_default();
            return Err(ApiError::Api {
                status,
                message: text,
            });
        }

        response
            .json::<ApiResponse>()
            .await
            .map_err(|e| ApiError::Parse(e.to_string()))
    }
}

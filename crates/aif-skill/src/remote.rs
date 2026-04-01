use serde::{Deserialize, Serialize};

/// A skill entry from the remote registry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteEntry {
    pub name: String,
    pub version: String,
    pub hash: String,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub requires: Vec<String>,
    pub author: Option<String>,
    pub published_at: Option<String>,
}

/// Search response from the remote registry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    pub results: Vec<RemoteEntry>,
    pub total: usize,
    pub page: usize,
}

/// Errors from remote registry operations
#[derive(Debug)]
pub enum RemoteError {
    NotConfigured,
    ConnectionFailed(String),
    NotFound(String),
    Unauthorized,
    ServerError(String),
    ParseError(String),
}

impl std::fmt::Display for RemoteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RemoteError::NotConfigured => write!(f, "Remote registry not configured"),
            RemoteError::ConnectionFailed(msg) => write!(f, "Connection failed: {}", msg),
            RemoteError::NotFound(name) => write!(f, "Not found: {}", name),
            RemoteError::Unauthorized => write!(f, "Unauthorized — run `aif auth login` first"),
            RemoteError::ServerError(msg) => write!(f, "Server error: {}", msg),
            RemoteError::ParseError(msg) => write!(f, "Parse error: {}", msg),
        }
    }
}

impl std::error::Error for RemoteError {}

/// Configuration for remote registry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteConfig {
    pub url: String,
    pub token: Option<String>,
    pub cache_dir: Option<String>,
}

impl Default for RemoteConfig {
    fn default() -> Self {
        Self {
            url: "https://registry.aif.dev".to_string(),
            token: None,
            cache_dir: None,
        }
    }
}

impl RemoteConfig {
    /// Load config from environment variable or default
    pub fn from_env() -> Self {
        let url = std::env::var("AIF_REGISTRY_URL")
            .unwrap_or_else(|_| "https://registry.aif.dev".to_string());
        let token = std::env::var("AIF_REGISTRY_TOKEN").ok();
        Self {
            url,
            token,
            cache_dir: None,
        }
    }
}

/// Remote registry client for searching and fetching skills
pub struct RemoteRegistry {
    pub config: RemoteConfig,
}

impl RemoteRegistry {
    pub fn new(config: RemoteConfig) -> Self {
        Self { config }
    }

    /// Build the full URL for an API endpoint
    fn api_url(&self, path: &str) -> String {
        format!("{}/v1{}", self.config.url.trim_end_matches('/'), path)
    }

    /// Search for skills matching a query.
    /// Note: Actual HTTP calls require the `remote-http` feature (reqwest).
    /// Without it, this returns NotConfigured — suitable for local-only usage.
    pub fn search(&self, query: &str, tags: &[&str]) -> Result<SearchResponse, RemoteError> {
        let _url = if tags.is_empty() {
            self.api_url(&format!("/skills?q={}", query))
        } else {
            self.api_url(&format!("/skills?q={}&tags={}", query, tags.join(",")))
        };

        // Without reqwest, we can't make HTTP calls
        Err(RemoteError::NotConfigured)
    }

    /// Fetch metadata for a specific skill
    pub fn fetch_metadata(
        &self,
        name: &str,
        version: Option<&str>,
    ) -> Result<RemoteEntry, RemoteError> {
        let _url = match version {
            Some(v) => self.api_url(&format!("/skills/{}/{}", name, v)),
            None => self.api_url(&format!("/skills/{}", name)),
        };

        Err(RemoteError::NotConfigured)
    }

    /// Download a skill file
    pub fn download(&self, name: &str, version: &str) -> Result<Vec<u8>, RemoteError> {
        let _url = self.api_url(&format!("/skills/{}/{}/download", name, version));
        Err(RemoteError::NotConfigured)
    }

    /// Publish a skill to the remote registry
    pub fn publish(&self, name: &str, version: &str, _data: &[u8]) -> Result<(), RemoteError> {
        if self.config.token.is_none() {
            return Err(RemoteError::Unauthorized);
        }
        let _url = self.api_url(&format!("/skills/{}/{}", name, version));
        Err(RemoteError::NotConfigured)
    }

    /// List all versions of a skill
    pub fn list_versions(&self, name: &str) -> Result<Vec<String>, RemoteError> {
        let _url = self.api_url(&format!("/skills/{}/versions", name));
        Err(RemoteError::NotConfigured)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remote_config_defaults() {
        let config = RemoteConfig::default();
        assert_eq!(config.url, "https://registry.aif.dev");
        assert!(config.token.is_none());
    }

    #[test]
    fn api_url_construction() {
        let registry = RemoteRegistry::new(RemoteConfig {
            url: "https://example.com".to_string(),
            token: None,
            cache_dir: None,
        });
        assert_eq!(registry.api_url("/skills"), "https://example.com/v1/skills");
        assert_eq!(
            registry.api_url("/skills/tdd/1.0.0"),
            "https://example.com/v1/skills/tdd/1.0.0"
        );
    }

    #[test]
    fn api_url_trims_trailing_slash() {
        let registry = RemoteRegistry::new(RemoteConfig {
            url: "https://example.com/".to_string(),
            token: None,
            cache_dir: None,
        });
        assert_eq!(registry.api_url("/skills"), "https://example.com/v1/skills");
    }

    #[test]
    fn search_without_http_returns_not_configured() {
        let registry = RemoteRegistry::new(RemoteConfig::default());
        let result = registry.search("debugging", &[]);
        assert!(matches!(result, Err(RemoteError::NotConfigured)));
    }

    #[test]
    fn publish_without_token_returns_unauthorized() {
        let registry = RemoteRegistry::new(RemoteConfig {
            url: "https://example.com".to_string(),
            token: None,
            cache_dir: None,
        });
        let result = registry.publish("test", "1.0.0", b"data");
        assert!(matches!(result, Err(RemoteError::Unauthorized)));
    }

    #[test]
    fn remote_entry_serialization() {
        let entry = RemoteEntry {
            name: "debugging".to_string(),
            version: "1.2.0".to_string(),
            hash: "sha256:abc123".to_string(),
            description: Some("Debug process".to_string()),
            tags: vec!["process".to_string()],
            requires: vec!["tdd:>=1.0.0".to_string()],
            author: Some("alice".to_string()),
            published_at: Some("2026-03-31T10:00:00Z".to_string()),
        };
        let json = serde_json::to_string(&entry).unwrap();
        let parsed: RemoteEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, "debugging");
        assert_eq!(parsed.requires, vec!["tdd:>=1.0.0"]);
    }

    #[test]
    fn search_response_serialization() {
        let response = SearchResponse {
            results: vec![RemoteEntry {
                name: "tdd".to_string(),
                version: "1.0.0".to_string(),
                hash: "sha256:def456".to_string(),
                description: None,
                tags: vec![],
                requires: vec![],
                author: None,
                published_at: None,
            }],
            total: 1,
            page: 1,
        };
        let json = serde_json::to_string(&response).unwrap();
        let parsed: SearchResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.total, 1);
        assert_eq!(parsed.results[0].name, "tdd");
    }
}

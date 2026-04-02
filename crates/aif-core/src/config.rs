use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Supported LLM providers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LlmProvider {
    Anthropic,
    #[serde(rename = "openai")]
    OpenAi,
    Google,
    Local,
}

impl LlmProvider {
    pub fn parse_provider(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "anthropic" => Some(Self::Anthropic),
            "openai" => Some(Self::OpenAi),
            "google" => Some(Self::Google),
            "local" => Some(Self::Local),
            _ => None,
        }
    }

    pub fn default_model(&self) -> &'static str {
        match self {
            Self::Anthropic => "claude-sonnet-4-6",
            Self::OpenAi => "gpt-4o",
            Self::Google => "gemini-2.5-pro",
            Self::Local => "default",
        }
    }
}

/// LLM provider configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    #[serde(default = "default_provider")]
    pub provider: LlmProvider,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
}

fn default_provider() -> LlmProvider {
    LlmProvider::Anthropic
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            provider: LlmProvider::Anthropic,
            api_key: None,
            model: None,
            base_url: None,
        }
    }
}

impl LlmConfig {
    pub fn apply_env(&mut self, provider: &str, api_key: Option<&str>, model: Option<&str>) {
        if let Some(p) = LlmProvider::parse_provider(provider) {
            self.provider = p;
        }
        if let Some(key) = api_key {
            self.api_key = Some(key.to_string());
        }
        if let Some(m) = model {
            self.model = Some(m.to_string());
        }
    }

    pub fn resolved_model(&self) -> &str {
        self.model
            .as_deref()
            .unwrap_or_else(|| self.provider.default_model())
    }
}

/// Top-level AIF config file structure (~/.aif/config.toml).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AifConfig {
    #[serde(default)]
    pub llm: LlmConfig,
}

impl AifConfig {
    pub fn load(path: &PathBuf) -> Self {
        match std::fs::read_to_string(path) {
            Ok(contents) => toml::from_str(&contents).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    pub fn load_with_env(path: &PathBuf) -> Self {
        let mut config = Self::load(path);

        let provider_env = std::env::var("AIF_LLM_PROVIDER").ok();
        let key_env = std::env::var("AIF_LLM_API_KEY").ok();
        let model_env = std::env::var("AIF_LLM_MODEL").ok();

        if let Some(ref provider) = provider_env {
            config.llm.apply_env(
                provider,
                key_env.as_deref(),
                model_env.as_deref(),
            );
        } else {
            if let Some(ref key) = key_env {
                config.llm.api_key = Some(key.clone());
            }
            if let Some(ref model) = model_env {
                config.llm.model = Some(model.clone());
            }
        }

        config
    }

    pub fn save(&self, path: &PathBuf) -> Result<(), String> {
        let dir = path.parent().ok_or("Invalid config path")?;
        std::fs::create_dir_all(dir).map_err(|e| format!("Failed to create config dir: {}", e))?;
        let toml_str =
            toml::to_string_pretty(self).map_err(|e| format!("Failed to serialize config: {}", e))?;
        std::fs::write(path, toml_str).map_err(|e| format!("Failed to write config: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_anthropic() {
        let config = LlmConfig::default();
        assert_eq!(config.provider, LlmProvider::Anthropic);
        assert!(config.api_key.is_none());
    }

    #[test]
    fn parse_from_toml() {
        let toml_str = r#"
[llm]
provider = "anthropic"
api_key = "sk-test-123"
model = "claude-sonnet-4-6"
"#;
        let config: AifConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.llm.provider, LlmProvider::Anthropic);
        assert_eq!(config.llm.api_key.as_deref(), Some("sk-test-123"));
        assert_eq!(config.llm.model.as_deref(), Some("claude-sonnet-4-6"));
    }

    #[test]
    fn provider_from_string() {
        assert_eq!(LlmProvider::parse_provider("anthropic"), Some(LlmProvider::Anthropic));
        assert_eq!(LlmProvider::parse_provider("openai"), Some(LlmProvider::OpenAi));
        assert_eq!(LlmProvider::parse_provider("ANTHROPIC"), Some(LlmProvider::Anthropic));
        assert_eq!(LlmProvider::parse_provider("unknown"), None);
    }

    #[test]
    fn load_from_env_overrides() {
        let mut config = LlmConfig::default();
        config.apply_env("anthropic", Some("sk-env-key"), Some("claude-opus-4-6"));
        assert_eq!(config.api_key.as_deref(), Some("sk-env-key"));
        assert_eq!(config.model.as_deref(), Some("claude-opus-4-6"));
    }
}

use crate::args::ConfigAction;
use crate::util::dirs_or_default;

pub fn handle_config(action: ConfigAction) {
    let config_path = dirs_or_default().join("config.toml");

    match action {
        ConfigAction::Set { key, value } => {
            let mut config = aif_core::config::AifConfig::load(&config_path);

            match key.as_str() {
                "llm.provider" => match aif_core::config::LlmProvider::parse_provider(&value) {
                    Some(p) => config.llm.provider = p,
                    None => {
                        eprintln!(
                            "Unknown provider: {}. Supported: anthropic, openai, google, local",
                            value
                        );
                        std::process::exit(1);
                    }
                },
                "llm.api-key" | "llm.api_key" => {
                    config.llm.api_key = Some(value);
                }
                "llm.model" => {
                    config.llm.model = Some(value);
                }
                "llm.base-url" | "llm.base_url" => {
                    config.llm.base_url = Some(value);
                }
                _ => {
                    eprintln!(
                        "Unknown config key: {}. Supported: llm.provider, llm.api-key, llm.model, llm.base-url",
                        key
                    );
                    std::process::exit(1);
                }
            }

            config.save(&config_path).unwrap_or_else(|e| {
                eprintln!("Error saving config: {}", e);
                std::process::exit(1);
            });
            println!("Set {} in {}", key, config_path.display());
        }
        ConfigAction::List {} => {
            let config = aif_core::config::AifConfig::load_with_env(&config_path);
            println!("Config (from {}):", config_path.display());
            println!("  llm.provider: {:?}", config.llm.provider);
            println!(
                "  llm.api-key: {}",
                config
                    .llm
                    .api_key
                    .as_ref()
                    .map(|k| format!("{}...", &k[..k.len().min(8)]))
                    .unwrap_or_else(|| "(not set)".into())
            );
            println!(
                "  llm.model: {} {}",
                config.llm.resolved_model(),
                if config.llm.model.is_some() {
                    "(explicit)"
                } else {
                    "(default)"
                }
            );
            if let Some(url) = &config.llm.base_url {
                println!("  llm.base-url: {}", url);
            }
        }
    }
}

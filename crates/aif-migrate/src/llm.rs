//! LLM-powered chunk application for migrations.
//!
//! Provides `apply_with_llm` which matches the engine's `apply_fn` signature
//! by wrapping an async Anthropic API call in a blocking tokio runtime.

use crate::apply::{build_migration_prompt, parse_migration_response};

/// Closure type for the migration apply function.
pub type ApplyFn = Box<dyn Fn(&[String], &str, Option<&str>) -> Option<String>>;

/// Create a closure suitable for `MigrationEngine::run()` that calls the Anthropic API.
///
/// Returns a closure with signature `Fn(&[String], &str, Option<&str>) -> Option<String>`.
/// Each invocation spins up a short-lived tokio runtime for the HTTP call.
pub fn make_llm_apply_fn(
    api_key: String,
    model: String,
) -> ApplyFn {
    Box::new(move |steps: &[String], source: &str, repair_ctx: Option<&str>| {
        let prompt = build_migration_prompt(steps, source, repair_ctx);
        let rt = tokio::runtime::Runtime::new().ok()?;
        rt.block_on(call_anthropic(&api_key, &model, &prompt))
    })
}

/// Call the Anthropic messages API and extract migrated code from the response.
async fn call_anthropic(api_key: &str, model: &str, prompt: &str) -> Option<String> {
    let client = reqwest::Client::new();

    let body = serde_json::json!({
        "model": model,
        "max_tokens": 8192,
        "messages": [{"role": "user", "content": prompt}]
    });

    let resp = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await;

    match resp {
        Ok(r) => {
            let status = r.status();
            let text = r.text().await.unwrap_or_default();
            if !status.is_success() {
                eprintln!("Anthropic API error ({}): {}", status, text);
                return None;
            }
            let parsed: serde_json::Value = serde_json::from_str(&text).unwrap_or_default();
            let content = parsed["content"][0]["text"].as_str().unwrap_or("");
            parse_migration_response(content)
        }
        Err(e) => {
            eprintln!("Anthropic API request failed: {}", e);
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prompt_roundtrip() {
        // Verify that build_migration_prompt produces a non-empty prompt
        // and parse_migration_response can extract code from a synthetic response.
        let steps = vec!["Replace foo with bar".to_string()];
        let source = "fn foo() {}";
        let prompt = build_migration_prompt(&steps, source, None);
        assert!(prompt.contains("Replace foo with bar"));
        assert!(prompt.contains("fn foo() {}"));

        // Simulate an LLM response containing a code block
        let fake_response = "Here is the migrated code:\n\n```rust\nfn bar() {}\n```\n";
        let parsed = parse_migration_response(fake_response);
        assert_eq!(parsed, Some("fn bar() {}".to_string()));
    }

    #[test]
    fn test_prompt_with_repair_context() {
        let steps = vec!["Step 1".to_string()];
        let prompt = build_migration_prompt(&steps, "code", Some("check failed: missing import"));
        assert!(prompt.contains("Repair Context"));
        assert!(prompt.contains("check failed: missing import"));
    }
}

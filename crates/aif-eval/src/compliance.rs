use aif_skill::eval::ComplianceResult;
use crate::anthropic::{AnthropicClient, ApiError, Message, Role};

/// A default compliance check definition.
#[derive(Debug, Clone)]
pub struct ComplianceCheck {
    pub name: String,
    pub description: String,
}

/// The three default compliance checks.
pub struct DefaultChecks;

impl DefaultChecks {
    pub fn all() -> Vec<ComplianceCheck> {
        vec![
            ComplianceCheck {
                name: "skill-acknowledgment".into(),
                description: "Agent acknowledges the skill is loaded and announces using it".into(),
            },
            ComplianceCheck {
                name: "step-order".into(),
                description: "Agent follows steps in the order declared by the skill".into(),
            },
            ComplianceCheck {
                name: "no-skip-mandatory".into(),
                description: "Agent does not skip any steps that are not marked optional".into(),
            },
        ]
    }
}

/// Configuration for compliance checking.
#[derive(Debug, Clone)]
pub struct ComplianceConfig {
    pub max_tokens: u32,
}

impl Default for ComplianceConfig {
    fn default() -> Self {
        Self { max_tokens: 2048 }
    }
}

/// Parse the LLM's compliance evaluation response.
pub fn parse_compliance_response(response: &str) -> Result<Vec<ComplianceResult>, String> {
    let json_str = extract_json(response);

    let parsed: serde_json::Value =
        serde_json::from_str(json_str).map_err(|e| format!("Failed to parse response: {}", e))?;

    let checks = parsed["checks"]
        .as_array()
        .ok_or("Response missing 'checks' array")?;

    let mut results = Vec::new();
    for check in checks {
        results.push(ComplianceResult {
            check_name: check["name"]
                .as_str()
                .unwrap_or("unknown")
                .to_string(),
            passed: check["passed"].as_bool().unwrap_or(false),
            evidence: check["evidence"]
                .as_str()
                .unwrap_or("")
                .to_string(),
        });
    }

    Ok(results)
}

fn extract_json(text: &str) -> &str {
    if let Some(start) = text.find("```json") {
        let after = &text[start + 7..];
        if let Some(end) = after.find("```") {
            return after[..end].trim();
        }
    }
    if let Some(start) = text.find("```") {
        let after = &text[start + 3..];
        if let Some(end) = after.find("```") {
            return after[..end].trim();
        }
    }
    text.trim()
}

/// Behavioral compliance evaluator.
pub struct ComplianceChecker {
    config: ComplianceConfig,
}

impl ComplianceChecker {
    pub fn new(config: ComplianceConfig) -> Self {
        Self { config }
    }

    /// Build the system + user prompts for compliance evaluation.
    pub fn build_prompt(
        &self,
        skill_text: &str,
        task: &str,
        checks: &[ComplianceCheck],
    ) -> (String, String) {
        let check_descriptions: String = checks
            .iter()
            .enumerate()
            .map(|(i, c)| format!("{}. **{}**: {}", i + 1, c.name, c.description))
            .collect::<Vec<_>>()
            .join("\n");

        let system = format!(
            "You are an eval agent that checks behavioral compliance. \
             You will be given a skill (the rules an agent should follow) and a task. \
             Simulate how an agent with this skill loaded would respond to the task, \
             then evaluate compliance against the checks below.\n\n\
             Compliance checks:\n{}\n\n\
             Respond with ONLY a JSON object in this format:\n\
             {{\"checks\": [{{\"name\": \"check-name\", \"passed\": true/false, \"evidence\": \"brief quote or observation\"}}]}}",
            check_descriptions
        );

        let user_msg = format!(
            "## Skill\n\n{}\n\n## Task\n\n{}\n\n\
             Simulate the agent's response to this task with the skill loaded, \
             then evaluate each compliance check.",
            skill_text, task
        );

        (system, user_msg)
    }

    /// Run compliance checks against an LLM.
    pub async fn evaluate(
        &self,
        client: &AnthropicClient,
        skill_text: &str,
        task: &str,
        checks: &[ComplianceCheck],
    ) -> Result<Vec<ComplianceResult>, ApiError> {
        let (system, user_msg) = self.build_prompt(skill_text, task, checks);

        let messages = vec![Message {
            role: Role::User,
            content: user_msg,
        }];

        let response = client
            .send(Some(&system), &messages, self.config.max_tokens)
            .await?;

        let text = response.text();
        parse_compliance_response(&text).map_err(ApiError::Parse)
    }
}

use aif_core::ast::*;
use aif_core::text::{inlines_to_text, TextMode};
use aif_skill::eval::{ScenarioResult, ScenarioType};

use crate::anthropic::{AnthropicClient, ApiError, Message, Role};

/// Extracted scenario specification from a @verify block.
#[derive(Debug, Clone)]
pub struct ScenarioSpec {
    pub name: String,
    pub scenario_type: ScenarioType,
    pub precondition: String,
    pub task: String,
    pub output_contract: String,
}

/// Extract scenario specs from a `@scenario` block (v2: scenarios are
/// direct children of `@skill`) or from a legacy `@verify` container
/// holding named scenario children.
pub fn extract_scenarios(block: &Block) -> Vec<ScenarioSpec> {
    // If block is itself a @scenario, extract from its children.
    if let BlockKind::SkillBlock {
        skill_type: SkillBlockType::Scenario,
        attrs,
        children,
        ..
    } = &block.kind
    {
        if let Some(spec) = extract_single_scenario(attrs, children) {
            return vec![spec];
        }
        return vec![];
    }
    // Else, treat as a container (legacy `@verify`) with @scenario children.
    let children = match &block.kind {
        BlockKind::SkillBlock { children, .. } => children,
        _ => return vec![],
    };

    let mut scenarios = Vec::new();
    for child in children {
        if let BlockKind::SkillBlock {
            attrs,
            children: sub_children,
            ..
        } = &child.kind
        {
            let name = match attrs.get("name") {
                Some(n) => n.to_string(),
                None => continue,
            };

            let scenario_type = match attrs.get("type") {
                Some("pressure") => ScenarioType::Pressure,
                Some("compliance") => ScenarioType::Compliance,
                _ => ScenarioType::Scenario,
            };

            let mut precondition = String::new();
            let mut task = String::new();
            let mut output_contract = String::new();

            for sub in sub_children {
                if let BlockKind::SkillBlock {
                    skill_type,
                    content,
                    ..
                } = &sub.kind
                {
                    let text = inlines_to_text(content, TextMode::Plain);
                    match skill_type {
                        SkillBlockType::Precondition => precondition = text,
                        SkillBlockType::Step => task = text,
                        SkillBlockType::OutputContract => output_contract = text,
                        _ => {}
                    }
                }
            }

            scenarios.push(ScenarioSpec {
                name,
                scenario_type,
                precondition,
                task,
                output_contract,
            });
        }
    }

    scenarios
}

/// Extract a single scenario spec from its attrs and direct children.
fn extract_single_scenario(
    attrs: &aif_core::ast::Attrs,
    sub_children: &[Block],
) -> Option<ScenarioSpec> {
    let name = attrs.get("name")?.to_string();
    let scenario_type = match attrs.get("type") {
        Some("pressure") => ScenarioType::Pressure,
        Some("compliance") => ScenarioType::Compliance,
        _ => ScenarioType::Scenario,
    };
    let mut precondition = String::new();
    let mut task = String::new();
    let mut output_contract = String::new();
    for sub in sub_children {
        if let BlockKind::SkillBlock {
            skill_type,
            content,
            ..
        } = &sub.kind
        {
            let text = inlines_to_text(content, TextMode::Plain);
            match skill_type {
                SkillBlockType::Precondition => precondition = text,
                SkillBlockType::Step => task = text,
                SkillBlockType::OutputContract => output_contract = text,
                _ => {}
            }
        }
    }
    Some(ScenarioSpec {
        name,
        scenario_type,
        precondition,
        task,
        output_contract,
    })
}

/// Parse the LLM's scenario evaluation response.
pub fn parse_scenario_response(
    response: &str,
    name: &str,
    scenario_type: ScenarioType,
) -> Result<ScenarioResult, String> {
    let json_str = extract_json(response);
    let parsed: serde_json::Value =
        serde_json::from_str(json_str).map_err(|e| format!("Failed to parse response: {}", e))?;

    Ok(ScenarioResult {
        name: name.to_string(),
        passed: parsed["passed"].as_bool().unwrap_or(false),
        evidence: parsed["evidence"].as_str().unwrap_or("").to_string(),
        scenario_type,
    })
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

/// Scenario test runner.
pub struct ScenarioRunner {
    max_tokens: u32,
}

impl ScenarioRunner {
    pub fn new(max_tokens: u32) -> Self {
        Self { max_tokens }
    }

    pub fn build_prompt(&self, skill_text: &str, spec: &ScenarioSpec) -> (String, String) {
        let system = "You are an eval agent that tests whether a coding-agent skill produces correct outcomes. \
             You will be given a skill, a scenario (precondition + task + expected output), and must \
             simulate how an agent with this skill loaded would handle the scenario.\n\n\
             Respond with ONLY a JSON object: {\"passed\": true/false, \"evidence\": \"brief explanation\"}\n\n\
             - passed=true means the agent would satisfy the output_contract\n\
             - passed=false means the agent would violate the output_contract\n\
             - evidence should be a 1-2 sentence explanation".to_string();

        let user_msg = format!(
            "## Skill\n\n{}\n\n\
             ## Scenario: {}\n\n\
             **precondition:** {}\n\n\
             **task:** {}\n\n\
             **output_contract:** {}\n\n\
             Simulate the agent's behavior and evaluate against the output_contract.",
            skill_text, spec.name, spec.precondition, spec.task, spec.output_contract
        );

        (system, user_msg)
    }

    pub async fn evaluate_one(
        &self,
        client: &AnthropicClient,
        skill_text: &str,
        spec: &ScenarioSpec,
    ) -> Result<ScenarioResult, ApiError> {
        let (system, user_msg) = self.build_prompt(skill_text, spec);

        let messages = vec![Message {
            role: Role::User,
            content: user_msg,
        }];

        let response = client
            .send(Some(&system), &messages, self.max_tokens)
            .await?;
        let text = response.text();

        parse_scenario_response(&text, &spec.name, spec.scenario_type)
            .map_err(ApiError::Parse)
    }

    pub async fn evaluate_all(
        &self,
        client: &AnthropicClient,
        skill_text: &str,
        scenarios: &[ScenarioSpec],
    ) -> Result<Vec<ScenarioResult>, ApiError> {
        let mut results = Vec::new();
        for spec in scenarios {
            let result = self.evaluate_one(client, skill_text, spec).await?;
            results.push(result);
        }
        Ok(results)
    }
}

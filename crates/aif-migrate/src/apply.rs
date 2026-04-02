use crate::types::SemanticCheck;
use regex::Regex;

pub fn build_migration_prompt(steps: &[String], source: &str, repair_context: Option<&str>) -> String {
    let mut prompt = String::new();
    prompt.push_str("You are a code migration assistant. Apply the following migration steps to the source code.\n\n");

    prompt.push_str("## Migration Steps\n\n");
    for (i, step) in steps.iter().enumerate() {
        prompt.push_str(&format!("{}. {}\n", i + 1, step));
    }

    prompt.push_str("\n## Source Code\n\n```\n");
    prompt.push_str(source);
    prompt.push_str("\n```\n\n");

    if let Some(context) = repair_context {
        prompt.push_str("## Repair Context\n\n");
        prompt.push_str("A previous migration attempt failed. Here's what went wrong:\n\n");
        prompt.push_str(context);
        prompt.push_str("\n\nPlease fix these issues in your migration.\n\n");
    }

    prompt.push_str("Output ONLY the migrated code in a single code block. Do not include explanations before the code block.\n");
    prompt
}

pub fn parse_migration_response(response: &str) -> Option<String> {
    let re = Regex::new(r"(?s)```(?:\w*)\n(.*?)```").unwrap();
    let blocks: Vec<String> = re.captures_iter(response)
        .map(|c| c[1].trim().to_string())
        .collect();
    if blocks.is_empty() {
        None
    } else {
        Some(blocks.join("\n\n"))
    }
}

pub fn build_semantic_verify_prompt(original: &str, migrated: &str, criteria: &[String]) -> String {
    let mut prompt = String::new();
    prompt.push_str("You are a code migration verifier. Check whether the migrated code satisfies each criterion.\n\n");

    prompt.push_str("## Original Code\n\n```\n");
    prompt.push_str(original);
    prompt.push_str("\n```\n\n");

    prompt.push_str("## Migrated Code\n\n```\n");
    prompt.push_str(migrated);
    prompt.push_str("\n```\n\n");

    prompt.push_str("## Verification Criteria\n\n");
    for (i, criterion) in criteria.iter().enumerate() {
        prompt.push_str(&format!("{}. {}\n", i + 1, criterion));
    }

    prompt.push_str("\nFor each criterion, respond with this exact format:\n\n");
    prompt.push_str("## Criterion N: <criterion text>\n");
    prompt.push_str("**PASS** or **FAIL** — confidence: <0.0-1.0>\n");
    prompt.push_str("<reasoning>\n\n");
    prompt
}

pub fn parse_semantic_response(response: &str, criteria: &[String]) -> Vec<SemanticCheck> {
    let section_re = Regex::new(r"(?m)^## Criterion \d+:.*$").unwrap();
    let pass_re = Regex::new(r"(?i)\*\*PASS\*\*").unwrap();
    let fail_re = Regex::new(r"(?i)\*\*FAIL\*\*").unwrap();
    let confidence_re = Regex::new(r"confidence:\s*([\d.]+)").unwrap();

    let section_starts: Vec<usize> = section_re.find_iter(response).map(|m| m.start()).collect();

    let mut checks = Vec::new();
    for (i, criterion) in criteria.iter().enumerate() {
        let section_text = if i < section_starts.len() {
            let start = section_starts[i];
            let end = section_starts.get(i + 1).copied().unwrap_or(response.len());
            &response[start..end]
        } else {
            ""
        };

        let passed = pass_re.is_match(section_text) && !fail_re.is_match(section_text);
        let confidence = confidence_re.captures(section_text)
            .and_then(|c| c[1].parse::<f64>().ok())
            .unwrap_or(0.5);

        let reasoning = section_text.lines().skip(2).collect::<Vec<_>>().join("\n").trim().to_string();

        checks.push(SemanticCheck {
            criterion: criterion.clone(),
            passed,
            reasoning,
            confidence,
        });
    }

    checks
}

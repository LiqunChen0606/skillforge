use crate::types::VerificationResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepairOutcome {
    Pending,
    Fixed,
    Exhausted,
}

#[derive(Debug)]
pub struct RepairState {
    max_iterations: u32,
    attempts: u32,
    last_passed: bool,
}

impl RepairState {
    pub fn new(max_iterations: u32) -> Self {
        Self {
            max_iterations,
            attempts: 0,
            last_passed: false,
        }
    }

    pub fn iteration(&self) -> u32 {
        self.attempts
    }

    pub fn can_retry(&self) -> bool {
        !self.last_passed && self.attempts < self.max_iterations
    }

    pub fn record_attempt(&mut self, passed: bool) {
        self.attempts += 1;
        self.last_passed = passed;
    }

    pub fn outcome(&self) -> RepairOutcome {
        if self.last_passed {
            RepairOutcome::Fixed
        } else if self.attempts >= self.max_iterations {
            RepairOutcome::Exhausted
        } else {
            RepairOutcome::Pending
        }
    }
}

pub fn build_repair_context(
    verification: &VerificationResult,
    fallback_text: Option<&str>,
) -> String {
    let mut context = String::new();

    let failed_static: Vec<_> = verification
        .static_checks
        .iter()
        .filter(|c| !c.passed)
        .collect();
    if !failed_static.is_empty() {
        context.push_str("## Failed Static Checks\n\n");
        for check in failed_static {
            context.push_str(&format!("- **{}**: {}\n", check.name, check.detail));
        }
        context.push('\n');
    }

    let failed_semantic: Vec<_> = verification
        .semantic_checks
        .iter()
        .filter(|c| !c.passed)
        .collect();
    if !failed_semantic.is_empty() {
        context.push_str("## Failed Semantic Checks\n\n");
        for check in failed_semantic {
            context.push_str(&format!(
                "- **{}**: {} (confidence: {:.2})\n",
                check.criterion, check.reasoning, check.confidence
            ));
        }
        context.push('\n');
    }

    if let Some(fallback) = fallback_text {
        context.push_str("## Fallback Guidance\n\n");
        context.push_str(fallback);
        context.push('\n');
    }

    context
}

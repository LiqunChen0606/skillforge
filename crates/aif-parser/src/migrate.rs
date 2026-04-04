//! One-shot text migration from legacy AIF v1 syntax (`@end`) to the current
//! v2 syntax (`@/name` container closers, implicit leaf auto-close).
//!
//! v1 is no longer supported by the parser. This module exists so users can
//! convert older `.aif` files with a single command.
//!
//! The transformation is purely line-based: it walks the source tracking
//! every opened `@`-directive and rewrites each `@end` line based on the
//! type of block it closes.
//!
//! - If `@end` closes a **container** (`@skill`, `@artifact_skill`), it is
//!   replaced with `@/skill` or `@/artifact_skill` (preserving the original
//!   indentation of the `@end` line).
//! - If `@end` closes a **leaf** (every other skill block type), the line
//!   is dropped entirely — v2 leaves auto-close at the next `@` directive.
//!
//! All non-`@end` lines pass through verbatim, so indentation, blank lines,
//! and comments are preserved exactly.

/// Convert a v1 AIF source string to v2.
///
/// Idempotent: running on v2 input returns it unchanged.
pub fn migrate_v1_to_v2(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut stack: Vec<&'static str> = Vec::new();
    let mut lines: Vec<&str> = input.split('\n').collect();
    let trailing_newline = input.ends_with('\n');
    if trailing_newline && lines.last().map(|s| s.is_empty()).unwrap_or(false) {
        lines.pop();
    }
    let last_idx = lines.len().saturating_sub(1);

    for (i, line) in lines.iter().enumerate() {
        let is_last = i == last_idx;
        let trimmed = line.trim();

        if trimmed == "@end" {
            // Pop the topmost opened block.
            match stack.pop() {
                Some(container)
                    if container == "skill"
                        || container == "artifact_skill"
                        || container == "scenario" =>
                {
                    // Preserve original indentation.
                    let indent_len = line.len() - line.trim_start().len();
                    let indent = &line[..indent_len];
                    out.push_str(indent);
                    out.push_str("@/");
                    out.push_str(container);
                    if !is_last || trailing_newline {
                        out.push('\n');
                    }
                }
                Some(_leaf) => {
                    // Drop the `@end` line entirely (leaves auto-close in v2).
                    // Also drop any trailing blank line that immediately followed,
                    // but we keep it simple: just drop `@end` and preserve newline
                    // structure around it.
                    // Nothing to push.
                }
                None => {
                    // Orphan `@end` — pass through as-is to surface the bug.
                    out.push_str(line);
                    if !is_last || trailing_newline {
                        out.push('\n');
                    }
                }
            }
            continue;
        }

        // Track block openings.
        if let Some(directive) = parse_directive_name(line) {
            if is_skill_directive(directive) {
                // Normalize the container name for later replacement.
                let tag: &'static str = match directive {
                    "skill" => "skill",
                    "artifact_skill" => "artifact_skill",
                    "scenario" => "scenario",
                    _ => "leaf",
                };
                stack.push(tag);
            }
        }

        out.push_str(line);
        if !is_last || trailing_newline {
            out.push('\n');
        }
    }

    out
}

/// If `line` is a `@directive ...` line, return the directive name.
/// Skips `@/` closers and `@end`.
fn parse_directive_name(line: &str) -> Option<&str> {
    let t = line.trim_start();
    let rest = t.strip_prefix('@')?;
    if rest.starts_with('/') {
        return None;
    }
    let end = rest
        .find(|c: char| c == '[' || c == ':' || c.is_whitespace())
        .unwrap_or(rest.len());
    let name = &rest[..end];
    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}

/// Returns true if `name` is a skill block directive (container or leaf).
fn is_skill_directive(name: &str) -> bool {
    matches!(
        name,
        "skill"
            | "artifact_skill"
            | "step"
            | "verify"
            | "precondition"
            | "output_contract"
            | "decision"
            | "tool"
            | "fallback"
            | "red_flag"
            | "example"
            | "scenario"
            | "input_schema"
            | "template"
            | "binding"
            | "generate"
            | "export"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrates_leaf_only_skill() {
        let input = "\
@skill[name=\"t\"]
@step[order=1]
First.
@end
@verify
Passes.
@end
@end
";
        let expected = "\
@skill[name=\"t\"]
@step[order=1]
First.
@verify
Passes.
@/skill
";
        assert_eq!(migrate_v1_to_v2(input), expected);
    }

    #[test]
    fn preserves_indentation() {
        let input = "\
@skill[name=\"t\"]
  @step[order=1]
    Indented body.
  @end
@end
";
        let expected = "\
@skill[name=\"t\"]
  @step[order=1]
    Indented body.
@/skill
";
        assert_eq!(migrate_v1_to_v2(input), expected);
    }

    #[test]
    fn idempotent_on_v2() {
        let v2 = "\
@skill[name=\"t\"]
@step[order=1]
Body.
@/skill
";
        assert_eq!(migrate_v1_to_v2(v2), v2);
    }

    #[test]
    fn artifact_skill_closer() {
        let input = "@artifact_skill[name=\"a\"]\n@template\nt\n@end\n@end\n";
        let out = migrate_v1_to_v2(input);
        assert!(out.contains("@/artifact_skill"));
        assert!(!out.contains("@end"));
    }

    #[test]
    fn preserves_non_skill_content() {
        let input = "#title: Doc\n\n@claim\nA claim.\n\n@evidence\nE.\n";
        assert_eq!(migrate_v1_to_v2(input), input);
    }
}

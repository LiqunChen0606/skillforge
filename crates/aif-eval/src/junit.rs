//! JUnit XML report generation for skill CI results.
//!
//! Produces valid JUnit XML from a skill name and slice of `ScenarioResult`,
//! suitable for consumption by CI systems (GitHub Actions, Jenkins, etc.).

use aif_skill::eval::ScenarioResult;

/// Escape XML special characters in text content.
fn xml_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(c),
        }
    }
    out
}

/// Generate a JUnit XML report from skill CI results.
///
/// The output contains a single `<testsuite>` with one `<testcase>` per
/// `ScenarioResult`. Failing scenarios include a `<failure>` child element
/// with the evidence text.
pub fn generate_junit_xml(skill_name: &str, results: &[ScenarioResult]) -> String {
    let total = results.len();
    let failures = results.iter().filter(|r| !r.passed).count();

    let mut xml = String::new();
    xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    xml.push_str(&format!(
        "<testsuite name=\"{}\" tests=\"{}\" failures=\"{}\">\n",
        xml_escape(skill_name),
        total,
        failures
    ));

    for result in results {
        xml.push_str(&format!(
            "  <testcase name=\"{}\" classname=\"{}\"",
            xml_escape(&result.name),
            xml_escape(skill_name),
        ));

        if result.passed {
            xml.push_str(" />\n");
        } else {
            xml.push_str(">\n");
            xml.push_str(&format!(
                "    <failure message=\"{}\">{}</failure>\n",
                xml_escape(&result.name),
                xml_escape(&result.evidence),
            ));
            xml.push_str("  </testcase>\n");
        }
    }

    xml.push_str("</testsuite>\n");
    xml
}

#[cfg(test)]
mod tests {
    use super::*;
    use aif_skill::eval::ScenarioType;

    #[test]
    fn junit_xml_structure() {
        let results = vec![
            ScenarioResult {
                name: "basic-test".into(),
                passed: true,
                evidence: "Agent ran tests".into(),
                scenario_type: ScenarioType::Scenario,
            },
            ScenarioResult {
                name: "edge-case".into(),
                passed: false,
                evidence: "Agent skipped validation".into(),
                scenario_type: ScenarioType::Compliance,
            },
        ];

        let xml = generate_junit_xml("my-skill", &results);

        assert!(xml.starts_with("<?xml version=\"1.0\""));
        assert!(xml.contains("<testsuite name=\"my-skill\" tests=\"2\" failures=\"1\">"));
        assert!(xml.contains("<testcase name=\"basic-test\" classname=\"my-skill\" />"));
        assert!(xml.contains("<testcase name=\"edge-case\" classname=\"my-skill\">"));
        assert!(xml.contains("<failure message=\"edge-case\">Agent skipped validation</failure>"));
        assert!(xml.contains("</testsuite>"));
    }

    #[test]
    fn junit_xml_escapes_special_chars() {
        let results = vec![ScenarioResult {
            name: "test <with> & \"quotes\" 'apostrophe'".into(),
            passed: false,
            evidence: "Error: x < y & z > w".into(),
            scenario_type: ScenarioType::Scenario,
        }];

        let xml = generate_junit_xml("skill & <name>", &results);

        assert!(xml.contains("name=\"skill &amp; &lt;name&gt;\""));
        assert!(xml.contains(
            "name=\"test &lt;with&gt; &amp; &quot;quotes&quot; &apos;apostrophe&apos;\""
        ));
        assert!(xml.contains("Error: x &lt; y &amp; z &gt; w"));
    }

    #[test]
    fn junit_xml_empty_results() {
        let xml = generate_junit_xml("empty-skill", &[]);
        assert!(xml.contains("tests=\"0\" failures=\"0\""));
        assert!(xml.contains("</testsuite>"));
    }

    #[test]
    fn junit_xml_all_passing() {
        let results = vec![
            ScenarioResult {
                name: "test-1".into(),
                passed: true,
                evidence: "ok".into(),
                scenario_type: ScenarioType::Scenario,
            },
            ScenarioResult {
                name: "test-2".into(),
                passed: true,
                evidence: "ok".into(),
                scenario_type: ScenarioType::Scenario,
            },
        ];

        let xml = generate_junit_xml("passing-skill", &results);
        assert!(xml.contains("failures=\"0\""));
        assert!(!xml.contains("<failure"));
    }
}

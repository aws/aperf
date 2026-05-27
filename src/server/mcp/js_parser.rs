//! Parse APerf JS report files.
//!
//! APerf report files are JavaScript files containing a single variable assignment:
//!   `processed_foo_data = { ... JSON ... }`
//! or findings:
//!   `foo_findings = { ... JSON ... }`
//!
//! This module strips the variable prefix and parses the JSON payload.

use regex::Regex;

/// Strip the JS variable assignment prefix and return the raw JSON string.
///
/// Handles patterns like:
///   `processed_cpu_utilization_data = {...};`
///   `cpu_utilization_findings = {...}`
///   `runs_raw = [...]`
pub fn extract_json_from_js(content: &str) -> Option<String> {
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(idx) = trimmed.find('=') {
            let after_eq = trimmed[idx + 1..].trim();
            let json_str = after_eq.trim_end_matches(';').trim();
            if json_str.starts_with('{') || json_str.starts_with('[') {
                return Some(json_str.to_string());
            }
        }
    }
    None
}

/// Extract all variable assignments from a JS file.
/// Returns Vec<(variable_name, json_string)>.
#[allow(dead_code)]
pub fn extract_all_variables(content: &str) -> Vec<(String, String)> {
    let mut results = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(idx) = trimmed.find('=') {
            let var_name = trimmed[..idx].trim().to_string();
            let after_eq = trimmed[idx + 1..].trim();
            let json_str = after_eq.trim_end_matches(';').trim();
            if json_str.starts_with('{') || json_str.starts_with('[') {
                results.push((var_name, json_str.to_string()));
            }
        }
    }
    results
}

/// Find all `*_findings` variables in a JS file content.
/// Returns Vec<(category_name, json_string)> where category_name has `_findings` stripped.
pub fn extract_findings_variables(content: &str) -> Vec<(String, String)> {
    let re = Regex::new(r"(\w+_findings)\s*=\s*").unwrap();
    let mut results = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(caps) = re.captures(trimmed) {
            let var_name = caps.get(1).unwrap().as_str();
            let category = var_name.trim_end_matches("_findings").to_string();
            if let Some(idx) = trimmed.find('=') {
                let after_eq = trimmed[idx + 1..].trim();
                let json_str = after_eq.trim_end_matches(';').trim();
                if json_str.starts_with('{') {
                    results.push((category, json_str.to_string()));
                }
            }
        }
    }
    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_json_processed_data() {
        let content = r#"processed_cpu_utilization_data = {"data_name":"cpu_utilization","data_format":"time_series"}"#;
        let json = extract_json_from_js(content).unwrap();
        assert!(json.starts_with('{'));
        assert!(json.contains("cpu_utilization"));
    }

    #[test]
    fn test_extract_json_with_semicolon() {
        let content = r#"processed_foo_data = {"key":"value"};"#;
        let json = extract_json_from_js(content).unwrap();
        assert_eq!(json, r#"{"key":"value"}"#);
    }

    #[test]
    fn test_extract_json_array() {
        let content = r#"runs_raw = ["run1","run2"]"#;
        let json = extract_json_from_js(content).unwrap();
        assert_eq!(json, r#"["run1","run2"]"#);
    }

    #[test]
    fn test_extract_all_variables() {
        let content = "processed_foo_data = {\"a\":1}\nfoo_findings = {\"b\":2}\n";
        let vars = extract_all_variables(content);
        assert_eq!(vars.len(), 2);
        assert_eq!(vars[0].0, "processed_foo_data");
        assert_eq!(vars[1].0, "foo_findings");
    }

    #[test]
    fn test_extract_findings_variables() {
        let content = "cpu_utilization_findings = {\"per_run_findings\":{}}\n";
        let findings = extract_findings_variables(content);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].0, "cpu_utilization");
    }

    #[test]
    fn test_no_json_returns_none() {
        let content = "// just a comment\n";
        assert!(extract_json_from_js(content).is_none());
    }
}

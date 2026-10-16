use crate::models::{Environment, EnvironmentVariable};
use regex::Regex;
use std::collections::HashMap;
use std::sync::LazyLock;

static VAR_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\{\{([a-zA-Z0-9_.-]+)\}\}").unwrap());

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariableSpan {
    pub start: usize,
    pub end: usize,
    pub name: String,
    pub is_resolved: bool,
    pub value: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterpolationResult {
    pub text: String,
    pub unresolved_variables: Vec<String>,
    pub spans: Vec<VariableSpan>,
}

/// Pure function to interpolate `{{var}}` placeholders using variables from the given environment.
/// Any unresolved variables remain in their `{{var}}` form in the output string,
/// and are reported in `unresolved_variables`.
pub fn interpolate(text: &str, env: Option<&Environment>) -> InterpolationResult {
    let mut var_map = HashMap::new();
    if let Some(env) = env {
        for v in &env.variables {
            var_map.insert(v.key.as_str(), v.resolved_value());
        }
    }
    interpolate_with_map(text, &var_map)
}

pub fn interpolate_with_vars(text: &str, vars: &[EnvironmentVariable]) -> InterpolationResult {
    let mut var_map = HashMap::new();
    for v in vars {
        var_map.insert(v.key.as_str(), v.resolved_value());
    }
    interpolate_with_map(text, &var_map)
}

pub fn interpolate_with_map(text: &str, var_map: &HashMap<&str, &str>) -> InterpolationResult {
    let mut result = String::with_capacity(text.len());
    let mut last_match_end = 0;
    let mut unresolved_variables = Vec::new();
    let mut spans = Vec::new();

    for caps in VAR_REGEX.captures_iter(text) {
        let whole_match = caps.get(0).unwrap();
        let var_name = caps.get(1).unwrap().as_str();

        // Push text preceding the match
        result.push_str(&text[last_match_end..whole_match.start()]);

        let start_pos = result.len();
        if let Some(&val) = var_map.get(var_name) {
            result.push_str(val);
            let end_pos = result.len();
            spans.push(VariableSpan {
                start: start_pos,
                end: end_pos,
                name: var_name.to_string(),
                is_resolved: true,
                value: Some(val.to_string()),
            });
        } else {
            result.push_str(whole_match.as_str());
            let end_pos = result.len();
            unresolved_variables.push(var_name.to_string());
            spans.push(VariableSpan {
                start: start_pos,
                end: end_pos,
                name: var_name.to_string(),
                is_resolved: false,
                value: None,
            });
        }

        last_match_end = whole_match.end();
    }

    result.push_str(&text[last_match_end..]);

    InterpolationResult {
        text: result,
        unresolved_variables,
        spans,
    }
}

/// Find variable tokens in raw text before substitution (useful for editor syntax highlighting)
pub fn scan_variables(raw_text: &str, env: Option<&Environment>) -> Vec<VariableSpan> {
    let mut var_map = HashMap::new();
    if let Some(env) = env {
        for v in &env.variables {
            var_map.insert(v.key.as_str(), v.resolved_value());
        }
    }

    let mut spans = Vec::new();
    for caps in VAR_REGEX.captures_iter(raw_text) {
        let whole_match = caps.get(0).unwrap();
        let var_name = caps.get(1).unwrap().as_str();
        let is_resolved = var_map.contains_key(var_name);
        let value = var_map.get(var_name).map(|v| v.to_string());

        spans.push(VariableSpan {
            start: whole_match.start(),
            end: whole_match.end(),
            name: var_name.to_string(),
            is_resolved,
            value,
        });
    }
    spans
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interpolation_basic() {
        let mut env = Environment::new("ws1", "local");
        env.variables.push(EnvironmentVariable {
            key: "host".to_string(),
            initial_value: "localhost:8000".to_string(),
            current_value: "".to_string(),
            secret: false,
        });
        env.variables.push(EnvironmentVariable {
            key: "token".to_string(),
            initial_value: "xyz123".to_string(),
            current_value: "".to_string(),
            secret: true,
        });

        let res = interpolate("http://{{host}}/api/v1?token={{token}}", Some(&env));
        assert_eq!(res.text, "http://localhost:8000/api/v1?token=xyz123");
        assert!(res.unresolved_variables.is_empty());
        assert_eq!(res.spans.len(), 2);
        assert!(res.spans[0].is_resolved);
        assert!(res.spans[1].is_resolved);
    }

    #[test]
    fn test_interpolation_unresolved() {
        let env = Environment::new("ws1", "local");
        let res = interpolate("http://{{missing_host}}/api/{{missing_path}}", Some(&env));
        assert_eq!(res.text, "http://{{missing_host}}/api/{{missing_path}}");
        assert_eq!(res.unresolved_variables, vec!["missing_host", "missing_path"]);
        assert_eq!(res.spans.len(), 2);
        assert!(!res.spans[0].is_resolved);
        assert!(!res.spans[1].is_resolved);
    }

    #[test]
    fn test_interpolation_prefers_current_value() {
        let mut env = Environment::new("ws1", "local");
        env.variables.push(EnvironmentVariable {
            key: "host".to_string(),
            initial_value: "git.example.com".to_string(),
            current_value: "localhost:8000".to_string(),
            secret: false,
        });

        let res = interpolate("http://{{host}}/test", Some(&env));
        assert_eq!(res.text, "http://localhost:8000/test");
        assert!(res.unresolved_variables.is_empty());
    }

    #[test]
    fn test_scan_variables() {
        let mut env = Environment::new("ws1", "local");
        env.variables.push(EnvironmentVariable {
            key: "port".to_string(),
            initial_value: "8080".to_string(),
            current_value: "".to_string(),
            secret: false,
        });

        let raw = "http://localhost:{{port}}/items/{{item_id}}";
        let spans = scan_variables(raw, Some(&env));
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].name, "port");
        assert!(spans[0].is_resolved);
        assert_eq!(spans[1].name, "item_id");
        assert!(!spans[1].is_resolved);
    }
}

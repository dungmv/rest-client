use crate::state::WorkspaceState;
use anyhow::Result;
use gpui::{App, Entity, Task, Window};
use gpui_component::input::{CompletionProvider, InlineToken, InputContent};
use gpui_component::RopeExt;
use lsp_types::{
    CompletionContext, CompletionItem, CompletionItemKind, CompletionResponse,
    CompletionTextEdit, Documentation, Range as LspRange, TextEdit,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvVarCandidate {
    pub name: String,
    pub value: String,
    pub scope: String,
    pub is_secret: bool,
}

/// Detects if `text` ends with an unclosed `{{` or `{{prefix`
/// Returns `Some((start_pos, query_after_braces))`
pub fn detect_unclosed_env_var(text: &str) -> Option<(usize, String)> {
    if let Some(pos) = text.rfind("{{") {
        let after = &text[pos + 2..];
        // Must not be closed with }} and should not have spaces, newlines, or quotes
        if !after.contains("}}") && !after.contains(' ') && !after.contains('\n') && !after.contains('"') {
            return Some((pos, after.to_string()));
        }
    }
    None
}

/// Returns all available environment variables matching `query` (case-insensitive)
pub fn get_env_completions(ws: &WorkspaceState, query: &str) -> Vec<EnvVarCandidate> {
    let q = query.to_lowercase();
    let mut candidates = Vec::new();
    let mut seen_keys = std::collections::HashSet::new();

    // 1. Active Environment variables (highest priority)
    if let Some(active_env) = ws.active_environment() {
        for v in &active_env.variables {
            if !seen_keys.contains(&v.key) {
                if q.is_empty() || v.key.to_lowercase().contains(&q) {
                    candidates.push(EnvVarCandidate {
                        name: v.key.clone(),
                        value: if v.secret { "••••••••".to_string() } else { v.resolved_value().to_string() },
                        scope: format!("Active: {}", active_env.name),
                        is_secret: v.secret,
                    });
                }
                seen_keys.insert(v.key.clone());
            }
        }
    }

    // 2. Global Environment variables
    for v in &ws.global_environment.variables {
        if !seen_keys.contains(&v.key) {
            if q.is_empty() || v.key.to_lowercase().contains(&q) {
                candidates.push(EnvVarCandidate {
                    name: v.key.clone(),
                    value: if v.secret { "••••••••".to_string() } else { v.resolved_value().to_string() },
                    scope: "Global".to_string(),
                    is_secret: v.secret,
                });
            }
            seen_keys.insert(v.key.clone());
        }
    }

    // 3. Dynamic variables ($guid, $timestamp, etc.)
    let dynamic_vars = [
        ("$guid", "Generates random UUID v4"),
        ("$uuid", "Generates random UUID v4"),
        ("$timestamp", "Current UNIX timestamp in seconds"),
        ("$isoTimestamp", "Current UTC timestamp in ISO 8601"),
        ("$randomInt", "Random integer between 1 and 1000"),
        ("$randomEmail", "Random email e.g. user1234@example.com"),
        ("$randomName", "Random sample full name"),
        ("$randomPhoneNumber", "Random phone number"),
    ];

    for (d_name, d_desc) in dynamic_vars {
        if q.is_empty()
            || d_name.to_lowercase().contains(&q)
            || d_name.trim_start_matches('$').to_lowercase().contains(&q)
        {
            candidates.push(EnvVarCandidate {
                name: d_name.to_string(),
                value: d_desc.to_string(),
                scope: "Dynamic".to_string(),
                is_secret: false,
            });
        }
    }

    candidates
}

/// Resolves variable details: `(value, scope, is_resolved)`
pub fn get_variable_details(ws: &WorkspaceState, var_name: &str) -> (String, String, bool) {
    // 1. Check Active Env
    if let Some(active_env) = ws.active_environment() {
        if let Some(v) = active_env.variables.iter().find(|x| x.key == var_name) {
            let val = if v.secret { "••••••••".to_string() } else { v.resolved_value().to_string() };
            return (val, format!("Active: {}", active_env.name), true);
        }
    }

    // 2. Check Global Env
    if let Some(v) = ws.global_environment.variables.iter().find(|x| x.key == var_name) {
        let val = if v.secret { "••••••••".to_string() } else { v.resolved_value().to_string() };
        return (val, "Global".to_string(), true);
    }

    // 3. Check dynamic variables
    if var_name.starts_with('$') {
        let fake = restclient_core::scripting::fake_vars::resolve_fake_vars(&format!("{{{{{}}}}}", var_name));
        if fake != format!("{{{{{}}}}}", var_name) {
            return (format!("Dynamic: {}", fake), "Dynamic".to_string(), true);
        }
    }

    ("Unresolved variable".to_string(), "Missing".to_string(), false)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnvVarKind {
    Active,
    Global,
    Dynamic,
    Unresolved,
}

/// Returns `(EnvVarKind, text_and_border_color, background_tint_color)`
pub fn get_variable_kind_and_color(scope: &str, is_res: bool) -> (EnvVarKind, gpui::Hsla, gpui::Hsla) {
    if !is_res {
        // Red / Unresolved
        (
            EnvVarKind::Unresolved,
            crate::theme::hex_to_hsla(0xef4444),
            crate::theme::hex_alpha_to_hsla(0xef4444, 0.14),
        )
    } else if scope.starts_with("Active") {
        // Sky Blue / Active Env
        (
            EnvVarKind::Active,
            crate::theme::hex_to_hsla(0x38bdf8),
            crate::theme::hex_alpha_to_hsla(0x38bdf8, 0.14),
        )
    } else if scope == "Global" {
        // Purple / Global Env
        (
            EnvVarKind::Global,
            crate::theme::hex_to_hsla(0xc084fc),
            crate::theme::hex_alpha_to_hsla(0xc084fc, 0.14),
        )
    } else {
        // Amber / Dynamic ($uuid, $timestamp, etc.)
        (
            EnvVarKind::Dynamic,
            crate::theme::hex_to_hsla(0xf59e0b),
            crate::theme::hex_alpha_to_hsla(0xf59e0b, 0.14),
        )
    }
}

/// Builds `InputContent` with inline tokens for all variable spans `{{var}}` found in `text`.
pub fn build_tokenized_content(text: &str, ws: &WorkspaceState) -> InputContent {
    let mut content = InputContent::new(text);
    let effective_env = ws.effective_environment();
    let spans = restclient_core::interpolation::scan_variables(text, Some(&effective_env));
    for span in spans {
        // Plain label with NO icons
        let token = InlineToken::new(span.name.clone(), format!("{{{{{}}}}}", span.name))
            .with_label(span.name.clone());
        if let Ok(new_content) = content.clone().with_token(span.start..span.end, token) {
            content = new_content;
        }
    }
    content
}

pub struct EnvCompletionProvider {
    workspace_state: Entity<WorkspaceState>,
}

impl EnvCompletionProvider {
    pub fn new(workspace_state: Entity<WorkspaceState>) -> Self {
        Self { workspace_state }
    }
}

impl CompletionProvider for EnvCompletionProvider {
    fn completions(
        &self,
        rope: &gpui_component::input::Rope,
        offset: usize,
        _trigger: CompletionContext,
        _window: &mut Window,
        cx: &mut App,
    ) -> Task<Result<CompletionResponse>> {
        let full_text = rope.to_string();
        let safe_offset = offset.min(full_text.len());
        let slice = &full_text[..safe_offset];

        if let Some(brace_pos) = slice.rfind("{{") {
            let after = &slice[brace_pos + 2..];
            if !after.contains("}}") && !after.contains('\n') && !after.contains('"') {
                let ws = self.workspace_state.read(cx);
                let candidates = get_env_completions(ws, after);

                let start_pos = rope.offset_to_position(brace_pos);
                let end_pos = rope.offset_to_position(safe_offset);

                let items: Vec<CompletionItem> = candidates
                    .into_iter()
                    .map(|c| {
                        let label = format!("{{{{{}}}}}", c.name);
                        let insert_text = format!("{{{{{}}}}}", c.name);
                        CompletionItem {
                            label: label.clone(),
                            filter_text: Some(c.name.clone()),
                            kind: Some(CompletionItemKind::VARIABLE),
                            detail: Some(format!("{} • {}", c.value, c.scope)),
                            documentation: Some(Documentation::String(format!(
                                "Environment Variable: {}\nValue: {}\nScope: {}",
                                c.name, c.value, c.scope
                            ))),
                            text_edit: Some(CompletionTextEdit::Edit(TextEdit {
                                range: LspRange {
                                    start: start_pos,
                                    end: end_pos,
                                },
                                new_text: insert_text,
                            })),
                            ..Default::default()
                        }
                    })
                    .collect();

                return Task::ready(Ok(CompletionResponse::Array(items)));
            }
        }

        Task::ready(Ok(CompletionResponse::Array(vec![])))
    }

    fn is_completion_trigger(&self, _offset: usize, new_text: &str, _cx: &mut App) -> bool {
        new_text.contains('{') || new_text.chars().any(|c| c.is_alphanumeric() || c == '_' || c == '$')
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_unclosed_env_var() {
        assert_eq!(detect_unclosed_env_var("https://api.com/{{"), Some((16, "".to_string())));
        assert_eq!(detect_unclosed_env_var("https://api.com/{{host"), Some((16, "host".to_string())));
        assert_eq!(detect_unclosed_env_var("https://api.com/{{host}}"), None);
        assert_eq!(detect_unclosed_env_var("https://api.com/{{host}}/{{user"), Some((25, "user".to_string())));
        assert_eq!(detect_unclosed_env_var("no braces"), None);
        assert_eq!(detect_unclosed_env_var("https://api.com/{{invalid with spaces"), None);
    }

    #[test]
    fn test_get_env_completions_and_details() {
        let mut ws = WorkspaceState::new();
        ws.global_environment.variables.push(restclient_core::models::EnvironmentVariable {
            key: "apiKey".to_string(),
            initial_value: "secret-123".to_string(),
            current_value: "".to_string(),
            secret: true,
        });

        let mut custom_env = restclient_core::models::Environment::new("default", "Staging");
        custom_env.variables.push(restclient_core::models::EnvironmentVariable {
            key: "baseUrl".to_string(),
            initial_value: "https://staging.api.com".to_string(),
            current_value: "".to_string(),
            secret: false,
        });
        let env_id = custom_env.id.clone();
        ws.environments.push(custom_env);
        ws.active_environment_id = Some(env_id);

        // 1. Completion test
        let completions = get_env_completions(&ws, "base");
        assert!(completions.iter().any(|c| c.name == "baseUrl" && c.scope.contains("Staging")));

        let global_comp = get_env_completions(&ws, "api");
        assert!(global_comp.iter().any(|c| c.name == "apiKey" && c.is_secret));

        // Dynamic completion test
        let dyn_comp = get_env_completions(&ws, "guid");
        assert!(dyn_comp.iter().any(|c| c.name == "$guid" && c.scope == "Dynamic"));

        // 2. Variable details test
        let (val, scope, is_res) = get_variable_details(&ws, "baseUrl");
        assert_eq!(val, "https://staging.api.com");
        assert!(scope.contains("Staging"));
        assert!(is_res);

        let (val, scope, is_res) = get_variable_details(&ws, "apiKey");
        assert_eq!(val, "••••••••");
        assert_eq!(scope, "Global");
        assert!(is_res);

        let (_, _, is_res) = get_variable_details(&ws, "nonExistent");
        assert!(!is_res);
    }
}

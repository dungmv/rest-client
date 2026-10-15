use gpui::Context;
use restclient_core::http::parse_query_from_url;
use restclient_core::models::{
    Auth, HttpMethod, HttpRequestItem, KeyValuePair, MultipartItem, RequestBody, RequestSettings,
};
use std::collections::HashSet;
use tokio::sync::oneshot;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RequestTabKind {
    #[default]
    Body,
    Params,
    Headers,
    Auth,
    PreRequestScript,
    Tests,
}

pub struct EditorState {
    pub active_request: Option<HttpRequestItem>,
    pub open_tabs: Vec<String>, // Request IDs
    pub pinned_tabs: HashSet<String>,
    pub active_tab_kind: RequestTabKind,
    pub is_sending: bool,
    pub send_progress: Option<String>,
    pub cancel_tx: Option<oneshot::Sender<()>>,
    pub is_dirty: bool,
    pub method_dropdown_open: bool,
    pub body_dropdown_open: bool,
    pub auth_dropdown_open: bool,
    pub script_error: Option<String>,
    pub body_version: usize,
}

impl EditorState {
    pub fn new() -> Self {
        Self {
            active_request: Some(HttpRequestItem::new(
                "scratch",
                "Untitled Request",
                HttpMethod::Get,
                "",
            )),
            open_tabs: Vec::new(),
            pinned_tabs: HashSet::new(),
            active_tab_kind: RequestTabKind::Body,
            is_sending: false,
            send_progress: None,
            cancel_tx: None,
            is_dirty: false,
            method_dropdown_open: false,
            body_dropdown_open: false,
            auth_dropdown_open: false,
            script_error: None,
            body_version: 0,
        }
    }

    pub fn ensure_active_request(&mut self) -> &mut HttpRequestItem {
        if self.active_request.is_none() {
            self.active_request = Some(HttpRequestItem::new(
                "scratch",
                "Untitled Request",
                HttpMethod::Get,
                "",
            ));
        }
        self.active_request.as_mut().unwrap()
    }

    pub fn set_active_request(&mut self, mut req: HttpRequestItem, pin: bool, cx: &mut Context<Self>) {
        let req_id = req.id.clone();
        if !self.open_tabs.contains(&req_id) {
            self.open_tabs.push(req_id.clone());
        }
        if pin {
            self.pinned_tabs.insert(req_id);
        }
        if req.url.contains('?') {
            let (base, parsed) = parse_query_from_url(&req.url);
            req.url = base;
            if req.params.is_empty() && !parsed.is_empty() {
                req.params = parsed;
            }
        }
        self.active_request = Some(req);
        self.is_dirty = false;
        self.body_version = self.body_version.wrapping_add(1);
        cx.notify();
    }

    pub fn pin_tab(&mut self, req_id: String, cx: &mut Context<Self>) {
        self.pinned_tabs.insert(req_id);
        cx.notify();
    }

    pub fn close_tab(&mut self, req_id: &str, cx: &mut Context<Self>) -> Option<String> {
        let idx = self.open_tabs.iter().position(|id| id == req_id);
        if let Some(i) = idx {
            self.open_tabs.remove(i);
        }
        self.pinned_tabs.remove(req_id);

        let next_active = if self.active_request.as_ref().map(|r| r.id.as_str()) == Some(req_id) {
            self.active_request = None;
            if !self.open_tabs.is_empty() {
                let new_idx = idx.unwrap_or(0).min(self.open_tabs.len() - 1);
                Some(self.open_tabs[new_idx].clone())
            } else {
                None
            }
        } else {
            self.active_request.as_ref().map(|r| r.id.clone())
        };

        cx.notify();
        next_active
    }

    pub fn set_method(&mut self, method: HttpMethod, cx: &mut Context<Self>) {
        if let Some(req) = &mut self.active_request {
            req.method = method;
            self.is_dirty = true;
            self.method_dropdown_open = false;
            cx.notify();
        }
    }

    pub fn set_name(&mut self, name: String, cx: &mut Context<Self>) {
        if let Some(req) = &mut self.active_request {
            if req.name != name {
                req.name = name;
                self.is_dirty = true;
                cx.notify();
            }
        }
    }

    /// Core logic to update URL and synchronize query parameters to params table.
    /// When query parameters are present in the URL, they are extracted to `req.params`,
    /// and stripped from `req.url` to keep the URL input clean and prevent duplication.
    pub fn apply_url(&mut self, url: String) {
        if let Some(req) = &mut self.active_request {
            if url.contains('?') {
                let (base, parsed_params) = parse_query_from_url(&url);
                req.url = base;
                if !parsed_params.is_empty() {
                    let mut disabled: Vec<KeyValuePair> =
                        req.params.iter().filter(|p| !p.enabled).cloned().collect();
                    let mut final_params = parsed_params;
                    final_params.append(&mut disabled);
                    req.params = final_params;
                    self.active_tab_kind = RequestTabKind::Params;
                }
            } else {
                req.url = url;
            }
            self.is_dirty = true;
        }
    }

    /// Sets URL and synchronizes query parameters to params table.
    pub fn set_url(&mut self, url: String, cx: &mut Context<Self>) {
        self.apply_url(url);
        cx.notify();
    }

    pub fn set_active_tab_kind(&mut self, kind: RequestTabKind, cx: &mut Context<Self>) {
        self.active_tab_kind = kind;
        cx.notify();
    }

    pub fn toggle_method_dropdown(&mut self, cx: &mut Context<Self>) {
        self.method_dropdown_open = !self.method_dropdown_open;
        cx.notify();
    }

    pub fn toggle_body_dropdown(&mut self, cx: &mut Context<Self>) {
        self.body_dropdown_open = !self.body_dropdown_open;
        cx.notify();
    }

    pub fn toggle_auth_dropdown(&mut self, cx: &mut Context<Self>) {
        self.auth_dropdown_open = !self.auth_dropdown_open;
        cx.notify();
    }

    // Params
    pub fn apply_update_param(&mut self, index: usize, key: String, val: String, enabled: bool) {
        if let Some(req) = &mut self.active_request {
            if index < req.params.len() {
                req.params[index].key = key;
                req.params[index].value = val;
                req.params[index].enabled = enabled;
                self.is_dirty = true;
            }
        }
    }

    pub fn update_param(&mut self, index: usize, key: String, val: String, enabled: bool, cx: &mut Context<Self>) {
        self.apply_update_param(index, key, val, enabled);
        cx.notify();
    }

    pub fn add_param(&mut self, cx: &mut Context<Self>) {
        if let Some(req) = &mut self.active_request {
            req.params.push(KeyValuePair::new("", ""));
            self.is_dirty = true;
            cx.notify();
        }
    }

    pub fn apply_remove_param(&mut self, index: usize) {
        if let Some(req) = &mut self.active_request {
            if index < req.params.len() {
                req.params.remove(index);
                self.is_dirty = true;
            }
        }
    }

    pub fn remove_param(&mut self, index: usize, cx: &mut Context<Self>) {
        self.apply_remove_param(index);
        cx.notify();
    }

    // Headers
    pub fn update_header(&mut self, index: usize, key: String, val: String, enabled: bool, cx: &mut Context<Self>) {
        if let Some(req) = &mut self.active_request {
            if index < req.headers.len() {
                req.headers[index].key = key;
                req.headers[index].value = val;
                req.headers[index].enabled = enabled;
                self.is_dirty = true;
                cx.notify();
            }
        }
    }

    pub fn add_header(&mut self, cx: &mut Context<Self>) {
        if let Some(req) = &mut self.active_request {
            req.headers.push(KeyValuePair::new("", ""));
            self.is_dirty = true;
            cx.notify();
        }
    }

    pub fn remove_header(&mut self, index: usize, cx: &mut Context<Self>) {
        if let Some(req) = &mut self.active_request {
            if index < req.headers.len() {
                req.headers.remove(index);
                self.is_dirty = true;
                cx.notify();
            }
        }
    }

    // Body
    pub fn set_body_json(&mut self, json_str: String, cx: &mut Context<Self>) {
        if let Some(req) = &mut self.active_request {
            req.body = RequestBody::Json(json_str);
            self.is_dirty = true;
            cx.notify();
        }
    }

    pub fn set_body_type(&mut self, body: RequestBody, cx: &mut Context<Self>) {
        if let Some(req) = &mut self.active_request {
            req.body = body;
            self.body_dropdown_open = false;
            self.is_dirty = true;
            self.body_version = self.body_version.wrapping_add(1);
            cx.notify();
        }
    }

    // Form Urlencoded
    pub fn update_form_param(&mut self, index: usize, key: String, val: String, enabled: bool, cx: &mut Context<Self>) {
        if let Some(req) = &mut self.active_request {
            if let RequestBody::FormUrlEncoded(pairs) = &mut req.body {
                if index < pairs.len() {
                    pairs[index].key = key;
                    pairs[index].value = val;
                    pairs[index].enabled = enabled;
                    self.is_dirty = true;
                    cx.notify();
                }
            }
        }
    }

    pub fn add_form_param(&mut self, cx: &mut Context<Self>) {
        if let Some(req) = &mut self.active_request {
            if let RequestBody::FormUrlEncoded(pairs) = &mut req.body {
                pairs.push(KeyValuePair::new("", ""));
                self.is_dirty = true;
                cx.notify();
            }
        }
    }

    pub fn remove_form_param(&mut self, index: usize, cx: &mut Context<Self>) {
        if let Some(req) = &mut self.active_request {
            if let RequestBody::FormUrlEncoded(pairs) = &mut req.body {
                if index < pairs.len() {
                    pairs.remove(index);
                    self.is_dirty = true;
                    cx.notify();
                }
            }
        }
    }

    // Multipart
    #[allow(clippy::too_many_arguments)]
    pub fn update_multipart_item(&mut self, index: usize, key: String, val: String, is_file: bool, file_path: Option<String>, enabled: bool, cx: &mut Context<Self>) {
        if let Some(req) = &mut self.active_request {
            if let RequestBody::Multipart(items) = &mut req.body {
                if index < items.len() {
                    items[index].key = key;
                    items[index].value = val;
                    items[index].is_file = is_file;
                    items[index].file_path = file_path;
                    items[index].enabled = enabled;
                    self.is_dirty = true;
                    cx.notify();
                }
            }
        }
    }

    pub fn add_multipart_item(&mut self, cx: &mut Context<Self>) {
        if let Some(req) = &mut self.active_request {
            if let RequestBody::Multipart(items) = &mut req.body {
                items.push(MultipartItem::default());
                self.is_dirty = true;
                cx.notify();
            }
        }
    }

    pub fn remove_multipart_item(&mut self, index: usize, cx: &mut Context<Self>) {
        if let Some(req) = &mut self.active_request {
            if let RequestBody::Multipart(items) = &mut req.body {
                if index < items.len() {
                    items.remove(index);
                    self.is_dirty = true;
                    cx.notify();
                }
            }
        }
    }

    // Raw body
    pub fn set_raw_body(&mut self, content: String, content_type: String, cx: &mut Context<Self>) {
        if let Some(req) = &mut self.active_request {
            req.body = RequestBody::Raw { content, content_type };
            self.is_dirty = true;
            cx.notify();
        }
    }

    // Binary body
    pub fn set_binary_file(&mut self, file_path: String, cx: &mut Context<Self>) {
        if let Some(req) = &mut self.active_request {
            req.body = RequestBody::Binary { file_path };
            self.is_dirty = true;
            cx.notify();
        }
    }

    // Auth
    pub fn set_auth(&mut self, auth: Auth, cx: &mut Context<Self>) {
        if let Some(req) = &mut self.active_request {
            req.auth = auth;
            self.auth_dropdown_open = false;
            self.is_dirty = true;
            cx.notify();
        }
    }

    pub fn set_bearer_token(&mut self, token: String, cx: &mut Context<Self>) {
        if let Some(req) = &mut self.active_request {
            req.auth = Auth::Bearer { token };
            self.is_dirty = true;
            cx.notify();
        }
    }

    pub fn set_basic_auth(&mut self, username: String, password: String, cx: &mut Context<Self>) {
        if let Some(req) = &mut self.active_request {
            req.auth = Auth::Basic { username, password };
            self.is_dirty = true;
            cx.notify();
        }
    }

    pub fn set_api_key(&mut self, key: String, value: String, in_header: bool, cx: &mut Context<Self>) {
        if let Some(req) = &mut self.active_request {
            req.auth = Auth::ApiKey { key, value, in_header };
            self.is_dirty = true;
            cx.notify();
        }
    }

    // Settings
    pub fn update_settings(&mut self, settings: RequestSettings, cx: &mut Context<Self>) {
        if let Some(req) = &mut self.active_request {
            req.settings = settings;
            self.is_dirty = true;
            cx.notify();
        }
    }

    pub fn set_timeout_secs(&mut self, secs: u64, cx: &mut Context<Self>) {
        if let Some(req) = &mut self.active_request {
            req.settings.timeout_secs = secs;
            self.is_dirty = true;
            cx.notify();
        }
    }

    pub fn set_follow_redirects(&mut self, follow: bool, cx: &mut Context<Self>) {
        if let Some(req) = &mut self.active_request {
            req.settings.follow_redirects = follow;
            self.is_dirty = true;
            cx.notify();
        }
    }

    pub fn set_validate_tls(&mut self, validate: bool, cx: &mut Context<Self>) {
        if let Some(req) = &mut self.active_request {
            req.settings.validate_tls = validate;
            self.is_dirty = true;
            cx.notify();
        }
    }

    // Sending state
    pub fn start_sending(&mut self, cancel_tx: oneshot::Sender<()>, cx: &mut Context<Self>) {
        self.is_sending = true;
        self.send_progress = Some("Connecting...".to_string());
        self.cancel_tx = Some(cancel_tx);
        cx.notify();
    }

    pub fn update_progress(&mut self, progress: String, cx: &mut Context<Self>) {
        self.send_progress = Some(progress);
        cx.notify();
    }

    pub fn stop_sending(&mut self, cx: &mut Context<Self>) {
        self.is_sending = false;
        self.send_progress = None;
        self.cancel_tx = None;
        cx.notify();
    }

    pub fn set_script_error(&mut self, err: Option<String>, cx: &mut Context<Self>) {
        self.script_error = err;
        cx.notify();
    }

    pub fn set_pre_script(&mut self, code: String, cx: &mut Context<Self>) {
        if let Some(req) = &mut self.active_request {
            let script = req.script.get_or_insert_with(Default::default);
            script.pre_request = if code.trim().is_empty() {
                None
            } else {
                Some(code)
            };
            self.is_dirty = true;
            cx.notify();
        }
    }

    pub fn set_test_script(&mut self, code: String, cx: &mut Context<Self>) {
        if let Some(req) = &mut self.active_request {
            let script = req.script.get_or_insert_with(Default::default);
            script.post_response = if code.trim().is_empty() {
                None
            } else {
                Some(code)
            };
            self.is_dirty = true;
            cx.notify();
        }
    }
}

pub const MAX_DEFAULT_DOMAIN_LEN: usize = 28;
pub const MAX_DISPLAY_TITLE_LEN: usize = 36;

pub fn truncate_text(s: &str, max_chars: usize) -> String {
    let trimmed = s.trim();
    if trimmed.chars().count() > max_chars {
        let truncated: String = trimmed.chars().take(max_chars.saturating_sub(1)).collect();
        format!("{}…", truncated)
    } else {
        trimmed.to_string()
    }
}

/// Extracts the domain (host) from a URL and limits its length.
pub fn default_name_from_url(url: &str) -> String {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return "Untitled Request".to_string();
    }

    let domain = if let Ok(parsed) = url::Url::parse(trimmed) {
        parsed.host_str().unwrap_or("").to_string()
    } else {
        let without_scheme = trimmed
            .trim_start_matches("https://")
            .trim_start_matches("http://");
        without_scheme
            .split(['/', '?', '#', ':'])
            .next()
            .unwrap_or("")
            .to_string()
    };

    let domain = domain.trim();
    if domain.is_empty() {
        return "Untitled Request".to_string();
    }

    truncate_text(domain, MAX_DEFAULT_DOMAIN_LEN)
}

#[allow(dead_code)]
pub fn is_legacy_url_name(name: &str, url: &str) -> bool {
    let clean = url
        .trim()
        .trim_start_matches("https://")
        .trim_start_matches("http://");
    if clean.is_empty() {
        return false;
    }
    let without_query = clean.split('?').next().unwrap_or(clean);
    let default_dom = default_name_from_url(url);
    (name == clean || name == without_query) && name != default_dom
}

#[allow(dead_code)]
pub fn is_auto_name(name: &str, old_url: &str) -> bool {
    let trimmed = name.trim();
    trimmed.is_empty()
        || trimmed == "New Request"
        || trimmed == "Untitled Request"
        || trimmed == default_name_from_url(old_url)
        || is_legacy_url_name(trimmed, old_url)
}

/// Returns the display name of a request (custom name if set, otherwise "Untitled Request"),
/// capped at `MAX_DISPLAY_TITLE_LEN` characters.
pub fn effective_request_name(req: &HttpRequestItem) -> String {
    let raw_name = req.name.trim();
    if raw_name.is_empty() {
        "Untitled Request".to_string()
    } else {
        truncate_text(raw_name, MAX_DISPLAY_TITLE_LEN)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_name_from_url() {
        assert_eq!(default_name_from_url("https://speedtest.vn/get-servers?loc=vn"), "speedtest.vn");
        assert_eq!(default_name_from_url("http://api.github.com/users/octocat"), "api.github.com");
        assert_eq!(default_name_from_url("https://localhost:8080/api/v1"), "localhost");
        assert_eq!(default_name_from_url("speedtest.vn/get-servers"), "speedtest.vn");
        assert_eq!(default_name_from_url(""), "Untitled Request");
        assert_eq!(default_name_from_url("   "), "Untitled Request");
    }

    #[test]
    fn test_domain_length_truncation() {
        let long_url = "https://very-long-subdomain-name-that-exceeds-maximum-allowed-length.example.com/api";
        let name = default_name_from_url(long_url);
        assert!(name.chars().count() <= MAX_DEFAULT_DOMAIN_LEN);
        assert!(name.ends_with('…'));
    }

    #[test]
    fn test_effective_request_name() {
        let mut req = HttpRequestItem::new("test", "New Request", HttpMethod::Get, "https://speedtest.vn/api");
        // Request name remains fixed even when URL has a domain
        assert_eq!(effective_request_name(&req), "New Request");

        // Custom name takes precedence
        req.name = "Get Speedtest Servers".to_string();
        assert_eq!(effective_request_name(&req), "Get Speedtest Servers");

        // Empty name falls back to Untitled Request
        req.name = "".to_string();
        assert_eq!(effective_request_name(&req), "Untitled Request");

        req.name = "   ".to_string();
        assert_eq!(effective_request_name(&req), "Untitled Request");
    }

    #[test]
    fn test_apply_url_with_query_params() {
        let mut editor = EditorState::new();
        editor.apply_url("https://api.example.com/search?q=rust&limit=10".to_string());

        let req = editor.active_request.as_ref().unwrap();
        // Query must be removed from URL to prevent duplication
        assert_eq!(req.url, "https://api.example.com/search");
        // Params must be extracted
        assert_eq!(req.params.len(), 2);
        assert_eq!(req.params[0].key, "q");
        assert_eq!(req.params[0].value, "rust");
        assert_eq!(req.params[1].key, "limit");
        assert_eq!(req.params[1].value, "10");
        assert_eq!(editor.active_tab_kind, RequestTabKind::Params);
    }

    #[test]
    fn test_apply_url_without_query_preserves_params() {
        let mut editor = EditorState::new();
        editor.apply_url("https://api.example.com/search?q=rust".to_string());
        assert_eq!(editor.active_request.as_ref().unwrap().params.len(), 1);

        // User edits base URL path without '?'
        editor.apply_url("https://api.example.com/search_v2".to_string());
        let req = editor.active_request.as_ref().unwrap();
        assert_eq!(req.url, "https://api.example.com/search_v2");
        // Existing query params should NOT be wiped out
        assert_eq!(req.params.len(), 1);
        assert_eq!(req.params[0].key, "q");
        assert_eq!(req.params[0].value, "rust");
    }

    #[test]
    fn test_update_param_does_not_append_to_url() {
        let mut editor = EditorState::new();
        editor.apply_url("https://api.example.com/items?page=1".to_string());
        assert_eq!(editor.active_request.as_ref().unwrap().url, "https://api.example.com/items");

        // Update param in params table
        editor.apply_update_param(0, "page".to_string(), "2".to_string(), true);
        let req = editor.active_request.as_ref().unwrap();
        // URL should still remain the clean base URL, avoiding duplicate
        assert_eq!(req.url, "https://api.example.com/items");
        assert_eq!(req.params[0].value, "2");

        // Remove param
        editor.apply_remove_param(0);
        let req = editor.active_request.as_ref().unwrap();
        assert_eq!(req.url, "https://api.example.com/items");
        assert_eq!(req.params.len(), 0);
    }
}


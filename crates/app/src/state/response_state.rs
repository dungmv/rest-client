use gpui::Context;
use restclient_core::models::{
    ConsoleLogEntry, ConsoleLogLevel, ConsoleLogSource, HttpResponseRecord, TestResult,
};
use restclient_core::storage::models::RequestExampleFile;
use restclient_core::storage::WorkspaceStore;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone, Debug, Default)]
pub struct FormattedResponseBody {
    pub display_text: Arc<String>,
    pub lines: Arc<Vec<String>>,
    pub is_json: bool,
    pub is_xml: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ResponseTabKind {
    #[default]
    Response,
    Request,
    Headers,
    Cookies,
    Tests,
    Console,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConsoleFilter {
    #[default]
    All,
    PreRequest,
    PostResponse,
    Errors,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ResponseViewMode {
    #[default]
    Pretty,
    Raw,
    Preview,
    Hex,
}

pub struct ResponseState {
    pub current_response: Option<HttpResponseRecord>,
    pub previous_response: Option<HttpResponseRecord>,
    pub session_responses: HashMap<String, HttpResponseRecord>,
    pub previous_responses: HashMap<String, HttpResponseRecord>,
    pub active_example_name: Option<String>,
    pub active_tab: ResponseTabKind,
    pub view_mode: ResponseViewMode,
    pub search_query: String,
    pub is_searching: bool,
    pub show_examples_dropdown: bool,
    pub show_timing_popover: bool,
    pub test_results: Vec<TestResult>,
    pub console_logs: Vec<ConsoleLogEntry>,
    pub console_filter: ConsoleFilter,
    pub formatted_body: Option<FormattedResponseBody>,
    pub scroll_handle: gpui::UniformListScrollHandle,
    pub wrap_lines: bool,
}

impl ResponseState {
    pub fn new() -> Self {
        Self {
            current_response: None,
            previous_response: None,
            session_responses: HashMap::new(),
            previous_responses: HashMap::new(),
            active_example_name: None,
            active_tab: ResponseTabKind::Response,
            view_mode: ResponseViewMode::Pretty,
            search_query: String::new(),
            is_searching: false,
            show_examples_dropdown: false,
            show_timing_popover: false,
            test_results: Vec::new(),
            console_logs: Vec::new(),
            console_filter: ConsoleFilter::All,
            formatted_body: None,
            scroll_handle: gpui::UniformListScrollHandle::new(),
            wrap_lines: false,
        }
    }

    pub fn load_for_request_internal(
        &mut self,
        req_id: &str,
        store: &dyn WorkspaceStore,
    ) {
        self.active_example_name = None;
        if let Some(cached) = self.session_responses.get(req_id).cloned() {
            self.test_results = cached.test_results.clone();
            self.console_logs = cached.console_logs.clone();
            self.current_response = Some(cached);
            self.previous_response = self.previous_responses.get(req_id).cloned();
        } else if let Ok(Some(loc)) = store.load_response_cache(req_id) {
            let record = loc.to_record(req_id);
            self.test_results = record.test_results.clone();
            self.console_logs = record.console_logs.clone();
            self.session_responses.insert(req_id.to_string(), record.clone());
            self.current_response = Some(record);
            self.previous_response = None;
        } else {
            self.test_results.clear();
            self.console_logs.clear();
            self.current_response = None;
            self.previous_response = None;
        }
        self.scroll_handle = gpui::UniformListScrollHandle::new();
        self.refresh_formatted_body();
    }

    pub fn load_for_request(
        &mut self,
        req_id: &str,
        store: &dyn WorkspaceStore,
        cx: &mut Context<Self>,
    ) {
        self.load_for_request_internal(req_id, store);
        cx.notify();
    }

    pub fn set_response_for_request(
        &mut self,
        req_id: String,
        response: HttpResponseRecord,
        cx: &mut Context<Self>,
    ) {
        self.active_example_name = None;
        self.test_results = response.test_results.clone();
        self.console_logs = response.console_logs.clone();

        let old = self.session_responses.insert(req_id.clone(), response.clone());
        if let Some(prev) = old {
            self.previous_responses.insert(req_id, prev.clone());
            self.previous_response = Some(prev);
        } else {
            self.previous_response = None;
        }
        self.current_response = Some(response);
        self.scroll_handle = gpui::UniformListScrollHandle::new();
        self.refresh_formatted_body();
        cx.notify();
    }

    pub fn rename_request(&mut self, old_id: &str, new_id: &str) {
        if let Some(r) = self.session_responses.remove(old_id) {
            self.session_responses.insert(new_id.to_string(), r);
        }
        if let Some(r) = self.previous_responses.remove(old_id) {
            self.previous_responses.insert(new_id.to_string(), r);
        }
    }

    pub fn view_example(&mut self, example: &RequestExampleFile, cx: &mut Context<Self>) {
        self.active_example_name = Some(example.name.clone());
        let headers: Vec<(String, String)> = example
            .headers
            .iter()
            .map(|h| (h.name.clone(), h.value.clone()))
            .collect();
        let body = example.body.as_deref().unwrap_or("").as_bytes().to_vec();
        let size_bytes = body.len();
        let record = HttpResponseRecord {
            id: format!("example-{}", example.name),
            request_id: String::new(),
            status_code: example.status,
            status_text: format!("Example: {}", example.name),
            headers,
            cookies: Vec::new(),
            body,
            duration_ms: 0,
            size_bytes,
            timing: Default::default(),
            request_sent: None,
            test_results: Vec::new(),
            console_logs: Vec::new(),
            created_at: chrono::Utc::now(),
        };
        self.current_response = Some(record);
        self.scroll_handle = gpui::UniformListScrollHandle::new();
        self.refresh_formatted_body();
        cx.notify();
    }

    pub fn restore_live_response(&mut self, req_id: &str, cx: &mut Context<Self>) {
        self.active_example_name = None;
        if let Some(resp) = self.session_responses.get(req_id).cloned() {
            self.current_response = Some(resp);
        }
        self.scroll_handle = gpui::UniformListScrollHandle::new();
        self.refresh_formatted_body();
        cx.notify();
    }

    pub fn rename_example(&mut self, old_name: &str, new_name: &str, cx: &mut Context<Self>) {
        if self.active_example_name.as_deref() == Some(old_name) {
            self.active_example_name = Some(new_name.to_string());
            if let Some(r) = &mut self.current_response {
                r.status_text = format!("Example: {}", new_name);
            }
            cx.notify();
        }
    }

    pub fn refresh_formatted_body(&mut self) {
        if let Some(resp) = &self.current_response {
            let raw_str = String::from_utf8_lossy(&resp.body).to_string();
            let parsed_json = serde_json::from_str::<serde_json::Value>(&raw_str).ok();
            let is_json = parsed_json.is_some()
                || resp.is_json()
                || raw_str.trim_start().starts_with('{')
                || raw_str.trim_start().starts_with('[');

            let is_xml = !is_json && (
                resp.is_xml()
                || raw_str.trim_start().starts_with("<?xml")
                || (raw_str.trim_start().starts_with('<') && raw_str.trim_end().ends_with('>'))
            );

            let display_text = if matches!(self.view_mode, ResponseViewMode::Pretty) {
                if let Some(parsed) = &parsed_json {
                    serde_json::to_string_pretty(parsed).unwrap_or(raw_str)
                } else if is_xml {
                    crate::components::body_editor::format_xml(&raw_str).unwrap_or(raw_str)
                } else {
                    raw_str
                }
            } else {
                raw_str
            };

            let lines: Vec<String> = display_text.lines().map(|s| s.to_string()).collect();
            self.formatted_body = Some(FormattedResponseBody {
                display_text: Arc::new(display_text),
                lines: Arc::new(lines),
                is_json,
                is_xml,
            });
        } else {
            self.formatted_body = None;
        }
    }

    pub fn set_response(
        &mut self,
        response: Option<HttpResponseRecord>,
        cx: &mut Context<Self>,
    ) {
        if let Some(r) = &response {
            self.test_results = r.test_results.clone();
            self.console_logs = r.console_logs.clone();
        }
        self.previous_response = self.current_response.take();
        self.current_response = response;
        self.scroll_handle = gpui::UniformListScrollHandle::new();
        self.refresh_formatted_body();
        cx.notify();
    }

    pub fn set_active_tab(&mut self, tab: ResponseTabKind, cx: &mut Context<Self>) {
        self.active_tab = tab;
        cx.notify();
    }

    pub fn set_view_mode(&mut self, mode: ResponseViewMode, cx: &mut Context<Self>) {
        self.view_mode = mode;
        self.refresh_formatted_body();
        cx.notify();
    }

    pub fn toggle_examples_dropdown(&mut self, cx: &mut Context<Self>) {
        self.show_examples_dropdown = !self.show_examples_dropdown;
        cx.notify();
    }

    pub fn toggle_timing_popover(&mut self, cx: &mut Context<Self>) {
        self.show_timing_popover = !self.show_timing_popover;
        cx.notify();
    }

    pub fn toggle_wrap_lines(&mut self, cx: &mut Context<Self>) {
        self.wrap_lines = !self.wrap_lines;
        cx.notify();
    }

    pub fn set_search_query(&mut self, query: String, cx: &mut Context<Self>) {
        self.search_query = query;
        cx.notify();
    }

    pub fn toggle_search(&mut self, cx: &mut Context<Self>) {
        self.is_searching = !self.is_searching;
        if !self.is_searching {
            self.search_query.clear();
        }
        cx.notify();
    }

    pub fn set_console_filter(&mut self, filter: ConsoleFilter, cx: &mut Context<Self>) {
        self.console_filter = filter;
        cx.notify();
    }

    pub fn clear_console_logs(&mut self, cx: &mut Context<Self>) {
        self.console_logs.clear();
        cx.notify();
    }

    pub fn set_pre_request_logs(
        &mut self,
        logs: Vec<ConsoleLogEntry>,
        cx: &mut Context<Self>,
    ) {
        self.console_logs = logs;
        cx.notify();
    }

    pub fn set_test_results(
        &mut self,
        results: Vec<TestResult>,
        logs: Vec<ConsoleLogEntry>,
        cx: &mut Context<Self>,
    ) {
        self.test_results = results;
        self.console_logs = logs;
        cx.notify();
    }

    pub fn clear_test_results(&mut self, cx: &mut Context<Self>) {
        self.test_results.clear();
        self.console_logs.clear();
        cx.notify();
    }

    pub fn filtered_console_logs(&self) -> Vec<&ConsoleLogEntry> {
        self.console_logs
            .iter()
            .filter(|entry| match self.console_filter {
                ConsoleFilter::All => true,
                ConsoleFilter::PreRequest => entry.source == ConsoleLogSource::PreRequest,
                ConsoleFilter::PostResponse => entry.source == ConsoleLogSource::PostResponse,
                ConsoleFilter::Errors => entry.level == ConsoleLogLevel::Error,
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_console_logs_filtering() {
        let mut state = ResponseState::new();
        state.console_logs = vec![
            ConsoleLogEntry {
                source: ConsoleLogSource::PreRequest,
                level: ConsoleLogLevel::Log,
                message: "Pre script log".to_string(),
                timestamp: "10:00:00".to_string(),
            },
            ConsoleLogEntry {
                source: ConsoleLogSource::PostResponse,
                level: ConsoleLogLevel::Warn,
                message: "Post script warn".to_string(),
                timestamp: "10:00:01".to_string(),
            },
            ConsoleLogEntry {
                source: ConsoleLogSource::PostResponse,
                level: ConsoleLogLevel::Error,
                message: "Post script error".to_string(),
                timestamp: "10:00:02".to_string(),
            },
        ];

        // Default filter: All
        state.console_filter = ConsoleFilter::All;
        assert_eq!(state.filtered_console_logs().len(), 3);

        // PreRequest filter
        state.console_filter = ConsoleFilter::PreRequest;
        let pre_logs = state.filtered_console_logs();
        assert_eq!(pre_logs.len(), 1);
        assert_eq!(pre_logs[0].message, "Pre script log");

        // PostResponse filter
        state.console_filter = ConsoleFilter::PostResponse;
        let post_logs = state.filtered_console_logs();
        assert_eq!(post_logs.len(), 2);

        // Errors filter
        state.console_filter = ConsoleFilter::Errors;
        let err_logs = state.filtered_console_logs();
        assert_eq!(err_logs.len(), 1);
        assert_eq!(err_logs[0].message, "Post script error");
    }

    #[test]
    fn test_console_tab_kind() {
        let mut state = ResponseState::new();
        assert_eq!(state.active_tab, ResponseTabKind::Response);
        state.active_tab = ResponseTabKind::Console;
        assert_eq!(state.active_tab, ResponseTabKind::Console);
    }

    #[test]
    fn test_large_response_body_formatting() {
        let mut state = ResponseState::new();

        // Create a 500-item JSON array
        let items: Vec<String> = (0..500).map(|i| format!(r#"{{"id": {}, "title": "item {}"}}"#, i, i)).collect();
        let json_body = format!("[\n{}\n]", items.join(",\n"));

        let rec = HttpResponseRecord {
            id: "resp-1".to_string(),
            request_id: "req-1".to_string(),
            status_code: 200,
            status_text: "OK".to_string(),
            duration_ms: 25,
            size_bytes: json_body.len(),
            headers: vec![("content-type".to_string(), "application/json".to_string())],
            cookies: vec![],
            body: json_body.into_bytes(),
            timing: restclient_core::models::TimingBreakdown::default(),
            request_sent: None,
            test_results: vec![],
            console_logs: vec![],
            created_at: chrono::Utc::now(),
        };

        state.current_response = Some(rec);
        state.refresh_formatted_body();

        let fmt = state.formatted_body.as_ref().expect("formatted body should be present");
        assert!(fmt.is_json);
        // Each item has 4 lines in pretty JSON (bracket, id, title, bracket), so > 1000 lines
        assert!(fmt.lines.len() >= 500);

        // Switch to Raw mode
        state.view_mode = ResponseViewMode::Raw;
        state.refresh_formatted_body();
        let fmt_raw = state.formatted_body.as_ref().unwrap();
        assert!(!fmt_raw.lines.is_empty());
    }

    #[test]
    fn test_load_for_request_from_cache() {
        let temp = tempfile::tempdir().unwrap();
        let store = restclient_core::storage::FsWorkspaceStore::new(temp.path());
        store.init_workspace("Test WS").unwrap();
        let (_, rel_path) = store
            .create_request(None, "Login", restclient_core::models::HttpMethod::Post, "http://localhost:8000/login")
            .unwrap();

        let loc_file = restclient_core::storage::models::LocFile {
            sent_at: "2026-10-15T12:00:00Z".to_string(),
            request: restclient_core::storage::models::LocRequestInfo {
                method: restclient_core::models::HttpMethod::Post,
                url: "http://localhost:8000/login".to_string(),
                headers: vec![],
                body: Some(r#"{"user":"alice"}"#.to_string()),
            },
            response: restclient_core::storage::models::LocResponseInfo {
                status: 200,
                duration_ms: 42,
                size: 15,
                headers: vec![("content-type".to_string(), "application/json".to_string())],
                body: Some(r#"{"token":"xyz"}"#.to_string()),
                truncated: false,
                binary: false,
            },
        };
        store.save_response_cache(&rel_path, &loc_file).unwrap();

        let mut state = ResponseState::new();
        assert!(state.current_response.is_none());

        state.load_for_request_internal(&rel_path, &store);
        assert!(state.current_response.is_some());
        let resp = state.current_response.as_ref().unwrap();
        assert_eq!(resp.status_code, 200);
        assert_eq!(resp.duration_ms, 42);
        assert_eq!(String::from_utf8_lossy(&resp.body), r#"{"token":"xyz"}"#);
        assert!(state.formatted_body.is_some());
    }

    #[test]
    fn test_xml_response_body_formatting() {
        let mut state = ResponseState::new();
        let record = HttpResponseRecord {
            id: "resp-xml".to_string(),
            request_id: "req-1".to_string(),
            status_code: 200,
            status_text: "OK".to_string(),
            headers: vec![("content-type".to_string(), "application/xml".to_string())],
            cookies: vec![],
            body: "<root><user><name>Bob</name></user></root>".as_bytes().to_vec(),
            duration_ms: 10,
            size_bytes: 42,
            timing: Default::default(),
            request_sent: None,
            test_results: vec![],
            console_logs: vec![],
            created_at: chrono::Utc::now(),
        };

        state.current_response = Some(record);
        state.view_mode = ResponseViewMode::Pretty;
        state.refresh_formatted_body();

        let fmt = state.formatted_body.expect("Formatted body should exist");
        assert!(fmt.is_xml, "Should detect XML response");
        assert!(fmt.display_text.contains("<root>\n"));
        assert!(fmt.display_text.contains("  <user>\n"));
    }

    #[test]
    fn test_wrap_lines_initial_and_toggle() {
        let mut state = ResponseState::new();
        assert!(!state.wrap_lines, "wrap_lines should default to false");
        state.wrap_lines = true;
        assert!(state.wrap_lines);
    }

    #[test]
    fn test_toolbar_icons_exist_in_all_assets() {
        use gpui::AssetSource as _;
        use gpui_kit::assets::{AllAssets, IconName};

        let assets = AllAssets;
        let icons = [
            IconName::TextWrap.path(),
            IconName::Copy.path(),
            IconName::Download.path(),
            IconName::Save.path(),
        ];

        for icon_path in icons {
            let res = assets.load(icon_path.as_ref());
            assert!(
                res.is_ok() && res.unwrap().is_some(),
                "Asset {} must exist in AllAssets",
                icon_path
            );
        }
    }
}


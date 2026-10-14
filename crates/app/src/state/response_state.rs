use gpui::Context;
use restclient_core::models::{
    ConsoleLogEntry, ConsoleLogLevel, ConsoleLogSource, HttpResponseRecord, TestResult,
};

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
    pub history: Vec<HttpResponseRecord>,
    pub active_tab: ResponseTabKind,
    pub view_mode: ResponseViewMode,
    pub search_query: String,
    pub is_searching: bool,
    pub show_history_dropdown: bool,
    pub show_timing_popover: bool,
    pub test_results: Vec<TestResult>,
    pub console_logs: Vec<ConsoleLogEntry>,
    pub console_filter: ConsoleFilter,
}

impl ResponseState {
    pub fn new() -> Self {
        Self {
            current_response: None,
            history: Vec::new(),
            active_tab: ResponseTabKind::Response,
            view_mode: ResponseViewMode::Pretty,
            search_query: String::new(),
            is_searching: false,
            show_history_dropdown: false,
            show_timing_popover: false,
            test_results: Vec::new(),
            console_logs: Vec::new(),
            console_filter: ConsoleFilter::All,
        }
    }

    pub fn set_response(
        &mut self,
        response: Option<HttpResponseRecord>,
        history: Vec<HttpResponseRecord>,
        cx: &mut Context<Self>,
    ) {
        if let Some(r) = &response {
            self.test_results = r.test_results.clone();
            self.console_logs = r.console_logs.clone();
        }
        self.current_response = response;
        self.history = history;
        cx.notify();
    }

    pub fn select_history_record(&mut self, record_id: &str, cx: &mut Context<Self>) {
        if let Some(rec) = self.history.iter().find(|r| r.id == record_id).cloned() {
            self.test_results = rec.test_results.clone();
            self.console_logs = rec.console_logs.clone();
            self.current_response = Some(rec);
            self.show_history_dropdown = false;
            cx.notify();
        }
    }

    pub fn set_active_tab(&mut self, tab: ResponseTabKind, cx: &mut Context<Self>) {
        self.active_tab = tab;
        cx.notify();
    }

    pub fn set_view_mode(&mut self, mode: ResponseViewMode, cx: &mut Context<Self>) {
        self.view_mode = mode;
        cx.notify();
    }

    pub fn toggle_history_dropdown(&mut self, cx: &mut Context<Self>) {
        self.show_history_dropdown = !self.show_history_dropdown;
        cx.notify();
    }

    pub fn toggle_timing_popover(&mut self, cx: &mut Context<Self>) {
        self.show_timing_popover = !self.show_timing_popover;
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
}


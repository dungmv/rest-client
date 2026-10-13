use gpui::Context;
use restclient_core::models::HttpResponseRecord;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ResponseTabKind {
    #[default]
    Response,
    Request,
    Headers,
    Cookies,
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
        }
    }

    pub fn set_response(
        &mut self,
        response: Option<HttpResponseRecord>,
        history: Vec<HttpResponseRecord>,
        cx: &mut Context<Self>,
    ) {
        self.current_response = response;
        self.history = history;
        cx.notify();
    }

    pub fn select_history_record(&mut self, record_id: &str, cx: &mut Context<Self>) {
        if let Some(rec) = self.history.iter().find(|r| r.id == record_id).cloned() {
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
}

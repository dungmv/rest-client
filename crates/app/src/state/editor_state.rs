use gpui::Context;
use restclient_core::models::{
    Auth, HttpMethod, HttpRequestItem, KeyValuePair, RequestBody, RequestSettings,
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
    Settings,
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
}

impl EditorState {
    pub fn new() -> Self {
        Self {
            active_request: None,
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
        }
    }

    pub fn set_active_request(&mut self, req: HttpRequestItem, pin: bool, cx: &mut Context<Self>) {
        let req_id = req.id.clone();
        if !self.open_tabs.contains(&req_id) {
            self.open_tabs.push(req_id.clone());
        }
        if pin {
            self.pinned_tabs.insert(req_id);
        }
        self.active_request = Some(req);
        self.is_dirty = false;
        cx.notify();
    }

    pub fn pin_tab(&mut self, req_id: String, cx: &mut Context<Self>) {
        self.pinned_tabs.insert(req_id);
        cx.notify();
    }

    pub fn close_tab(&mut self, req_id: &str, cx: &mut Context<Self>) -> Option<String> {
        let idx = self.open_tabs.iter().position(|id| id == req_id)?;
        self.open_tabs.remove(idx);
        self.pinned_tabs.remove(req_id);

        let next_active = if self.active_request.as_ref().map(|r| r.id.as_str()) == Some(req_id) {
            if !self.open_tabs.is_empty() {
                let new_idx = idx.min(self.open_tabs.len() - 1);
                Some(self.open_tabs[new_idx].clone())
            } else {
                self.active_request = None;
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

    pub fn set_url(&mut self, url: String, cx: &mut Context<Self>) {
        if let Some(req) = &mut self.active_request {
            req.url = url;
            self.is_dirty = true;
            cx.notify();
        }
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
    pub fn update_param(&mut self, index: usize, key: String, val: String, enabled: bool, cx: &mut Context<Self>) {
        if let Some(req) = &mut self.active_request {
            if index < req.params.len() {
                req.params[index].key = key;
                req.params[index].value = val;
                req.params[index].enabled = enabled;
                self.is_dirty = true;
                cx.notify();
            }
        }
    }

    pub fn add_param(&mut self, cx: &mut Context<Self>) {
        if let Some(req) = &mut self.active_request {
            req.params.push(KeyValuePair::new("", ""));
            self.is_dirty = true;
            cx.notify();
        }
    }

    pub fn remove_param(&mut self, index: usize, cx: &mut Context<Self>) {
        if let Some(req) = &mut self.active_request {
            if index < req.params.len() {
                req.params.remove(index);
                self.is_dirty = true;
                cx.notify();
            }
        }
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

    // Settings
    pub fn update_settings(&mut self, settings: RequestSettings, cx: &mut Context<Self>) {
        if let Some(req) = &mut self.active_request {
            req.settings = settings;
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
}

#![allow(dead_code, clippy::type_complexity)]

mod components;
mod platform;
mod state;
mod theme;

use components::*;
use gpui::prelude::FluentBuilder as _;
use gpui::*;
use gpui_component::input::{Input, InputEvent, InputState, TextareaState};
use gpui_component::resizable::{h_resizable, resizable_panel, v_resizable};
use gpui_kit::assets::Assets;
use platform::default_window_options;
use restclient_core::http::{HttpError, HttpExecutor, HttpProgress, ReqwestExecutor, ResolvedRequest};
use restclient_core::models::{Auth, HttpMethod, HttpResponseRecord, RequestBody};
use state::*;
use std::sync::Arc;
use theme::theme;

actions!(restclient, [SendRequest, ToggleCommandPalette]);

pub enum HttpSendEvent {
    Progress(String),
    Success(Box<HttpResponseRecord>),
    Cancelled,
    Error(String),
}

pub struct AppView {
    workspace_state: Entity<WorkspaceState>,
    editor_state: Entity<EditorState>,
    response_state: Entity<ResponseState>,
    toast_state: Entity<ToastState>,
    name_input: Entity<InputState>,
    url_input: Entity<InputState>,
    json_textarea: Entity<TextareaState>,
    raw_textarea: Entity<TextareaState>,
    bearer_token_input: Entity<InputState>,
    basic_user_input: Entity<InputState>,
    basic_pass_input: Entity<InputState>,
    apikey_key_input: Entity<InputState>,
    apikey_val_input: Entity<InputState>,
    executor: Arc<ReqwestExecutor>,
    tokio_rt: Arc<tokio::runtime::Runtime>,
    _subscriptions: Vec<Subscription>,
}

impl AppView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let workspace_state = cx.new(|_| WorkspaceState::new());
        let editor_state = cx.new(|_| EditorState::new());
        let response_state = cx.new(|_| ResponseState::new());
        let toast_state = cx.new(|_| ToastState::new());

        let name_input = cx.new(|cx| InputState::new(window, cx).placeholder("Request name (default: domain)"));
        let url_input = cx.new(|cx| InputState::new(window, cx).placeholder("Enter request URL (e.g. https://httpbin.org/get)"));
        let json_textarea = cx.new(|cx| TextareaState::new(window, cx));
        let raw_textarea = cx.new(|cx| TextareaState::new(window, cx));

        let bearer_token_input = cx.new(|cx| InputState::new(window, cx).placeholder("Enter Bearer token..."));
        let basic_user_input = cx.new(|cx| InputState::new(window, cx).placeholder("Username..."));
        let basic_pass_input = cx.new(|cx| InputState::new(window, cx).placeholder("Password..."));
        let apikey_key_input = cx.new(|cx| InputState::new(window, cx).placeholder("X-API-Key"));
        let apikey_val_input = cx.new(|cx| InputState::new(window, cx).placeholder("API key value..."));

        let executor = Arc::new(ReqwestExecutor::new());
        let tokio_rt = Arc::new(
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .expect("Failed to create tokio runtime"),
        );

        // Select initial request from workspace if the user has saved requests
        let initial_req = workspace_state.read(cx).requests.first().cloned();
        if let Some(req) = &initial_req {
            editor_state.update(cx, |ed, cx| {
                ed.set_active_request(req.clone(), true, cx);
            });

            let req_name = req.name.clone();
            name_input.update(cx, |inp, cx| {
                inp.set_value(req_name, window, cx);
            });

            let req_url = req.url.clone();
            url_input.update(cx, |inp, cx| {
                inp.set_value(req_url, window, cx);
            });

            if let RequestBody::Json(json_content) = &req.body {
                json_textarea.update(cx, |txt, cx| {
                    txt.set_value(json_content.clone(), window, cx);
                });
            } else if let RequestBody::Raw { content, .. } = &req.body {
                raw_textarea.update(cx, |txt, cx| {
                    txt.set_value(content.clone(), window, cx);
                });
            }

            // Load initial history
            let history = workspace_state
                .read(cx)
                .history_storage
                .get_responses(&req.id, 20)
                .unwrap_or_default();
            let latest = history.first().cloned();
            response_state.update(cx, |resp, cx| {
                resp.set_response(latest, history, cx);
            });
        }

        // Set up subscriptions:
        let mut _subscriptions = Vec::new();

        // 0. Subscribe to Request Name input change
        let ed_name = editor_state.clone();
        _subscriptions.push(cx.subscribe(&name_input, move |_, input, event, cx| {
            if let InputEvent::Change = event {
                let val = input.read(cx).value().to_string();
                ed_name.update(cx, |ed, cx| {
                    ed.set_name(val, cx);
                });
            }
        }));

        // 1. Subscribe to URL input change
        let ed_url = editor_state.clone();
        _subscriptions.push(cx.subscribe(&url_input, move |this, input, event, cx| {
            match event {
                InputEvent::Change => {
                    let val = input.read(cx).value().to_string();
                    ed_url.update(cx, |ed, cx| {
                        ed.set_url(val, cx);
                    });
                }
                InputEvent::PressEnter { .. } => {
                    this.send_current_request(cx);
                }
                _ => {}
            }
        }));

        // 2. Subscribe to JSON textarea change
        let ed_json = editor_state.clone();
        _subscriptions.push(cx.subscribe(&json_textarea, move |_, txt, event, cx| {
            if let InputEvent::Change = event {
                let val = txt.read(cx).value().to_string();
                ed_json.update(cx, |ed, cx| {
                    ed.set_body_json(val, cx);
                });
            }
        }));

        // 3. Subscribe to Raw textarea change
        let ed_raw = editor_state.clone();
        _subscriptions.push(cx.subscribe(&raw_textarea, move |_, txt, event, cx| {
            if let InputEvent::Change = event {
                let val = txt.read(cx).value().to_string();
                ed_raw.update(cx, |ed, cx| {
                    let content_type = if let Some(req) = &ed.active_request {
                        if let RequestBody::Raw { content_type, .. } = &req.body {
                            content_type.clone()
                        } else {
                            "text/plain".to_string()
                        }
                    } else {
                        "text/plain".to_string()
                    };
                    ed.set_raw_body(val, content_type, cx);
                });
            }
        }));

        // 3b. Subscribe to Auth inputs
        let ed_bearer = editor_state.clone();
        _subscriptions.push(cx.subscribe(&bearer_token_input, move |_, inp, event, cx| {
            if let InputEvent::Change = event {
                let val = inp.read(cx).value().to_string();
                ed_bearer.update(cx, |ed, cx| {
                    ed.set_bearer_token(val, cx);
                });
            }
        }));

        let ed_basic_u = editor_state.clone();
        let pass_ref = basic_pass_input.clone();
        _subscriptions.push(cx.subscribe(&basic_user_input, move |_, inp, event, cx| {
            if let InputEvent::Change = event {
                let u = inp.read(cx).value().to_string();
                let p = pass_ref.read(cx).value().to_string();
                ed_basic_u.update(cx, |ed, cx| {
                    ed.set_basic_auth(u, p, cx);
                });
            }
        }));

        let ed_basic_p = editor_state.clone();
        let user_ref = basic_user_input.clone();
        _subscriptions.push(cx.subscribe(&basic_pass_input, move |_, inp, event, cx| {
            if let InputEvent::Change = event {
                let u = user_ref.read(cx).value().to_string();
                let p = inp.read(cx).value().to_string();
                ed_basic_p.update(cx, |ed, cx| {
                    ed.set_basic_auth(u, p, cx);
                });
            }
        }));

        let ed_ak_k = editor_state.clone();
        let ak_val_ref = apikey_val_input.clone();
        _subscriptions.push(cx.subscribe(&apikey_key_input, move |_, inp, event, cx| {
            if let InputEvent::Change = event {
                let k = inp.read(cx).value().to_string();
                let v = ak_val_ref.read(cx).value().to_string();
                ed_ak_k.update(cx, |ed, cx| {
                    let in_header = match ed.active_request.as_ref().map(|r| &r.auth) {
                        Some(Auth::ApiKey { in_header, .. }) => *in_header,
                        _ => true,
                    };
                    ed.set_api_key(k, v, in_header, cx);
                });
            }
        }));

        let ed_ak_v = editor_state.clone();
        let ak_key_ref = apikey_key_input.clone();
        _subscriptions.push(cx.subscribe(&apikey_val_input, move |_, inp, event, cx| {
            if let InputEvent::Change = event {
                let k = ak_key_ref.read(cx).value().to_string();
                let v = inp.read(cx).value().to_string();
                ed_ak_v.update(cx, |ed, cx| {
                    let in_header = match ed.active_request.as_ref().map(|r| &r.auth) {
                        Some(Auth::ApiKey { in_header, .. }) => *in_header,
                        _ => true,
                    };
                    ed.set_api_key(k, v, in_header, cx);
                });
            }
        }));

        // 4. Observe editor_state: reload response history when active request switches & auto-save
        let ws_clone = workspace_state.clone();
        let resp_clone = response_state.clone();
        let mut last_req_id = initial_req.as_ref().map(|r| r.id.clone());
        _subscriptions.push(cx.observe(&editor_state, move |_, ed, cx| {
            let active_req = ed.read(cx).active_request.clone();
            if let Some(req) = active_req {
                let req_id = req.id.clone();
                if last_req_id.as_ref() != Some(&req_id) {
                    last_req_id = Some(req_id.clone());
                    let history = ws_clone
                        .read(cx)
                        .history_storage
                        .get_responses(&req_id, 20)
                        .unwrap_or_default();
                    let latest = history.first().cloned();
                    resp_clone.update(cx, |r_state, cx| {
                        r_state.set_response(latest, history, cx);
                    });
                }

                // Auto-save active request to workspace storage only if it belongs to a real workspace
                if req.workspace_id != "scratch" {
                    ws_clone.update(cx, |ws, cx| {
                        ws.update_request(req, cx);
                    });
                }
            }
        }));

        Self {
            workspace_state,
            editor_state,
            response_state,
            toast_state,
            name_input,
            url_input,
            json_textarea,
            raw_textarea,
            bearer_token_input,
            basic_user_input,
            basic_pass_input,
            apikey_key_input,
            apikey_val_input,
            executor,
            tokio_rt,
            _subscriptions,
        }
    }

    pub fn send_current_request(&self, cx: &mut Context<Self>) {
        let ed_entity = self.editor_state.clone();
        let ws_entity = self.workspace_state.clone();
        let executor = self.executor.clone();
        let rt = self.tokio_rt.clone();

        let ed = ed_entity.read(cx);
        let Some(req_item) = ed.active_request.clone() else {
            return;
        };

        if ed.is_sending {
            return;
        }

        let ws = ws_entity.read(cx);
        let env = ws.active_environment().cloned();

        let resolved = match ResolvedRequest::from_item(&req_item, env.as_ref()) {
            Ok(r) => r,
            Err(e) => {
                let err_msg = e.to_string();
                self.toast_state.update(cx, |ts, cx| {
                    ts.push_error("Request Error", err_msg, cx);
                });
                return;
            }
        };

        let (cancel_tx, cancel_rx) = tokio::sync::oneshot::channel();
        ed_entity.update(cx, |state, cx| {
            state.start_sending(cancel_tx, cx);
        });

        let (progress_tx, mut progress_rx) = tokio::sync::mpsc::unbounded_channel();
        let (send_event_tx, mut send_event_rx) = tokio::sync::mpsc::unbounded_channel();

        // Forward progress events to send_event_tx
        let progress_event_tx = send_event_tx.clone();
        rt.spawn(async move {
            while let Some(prog) = progress_rx.recv().await {
                let text = match prog {
                    HttpProgress::Connecting => "Connecting...".to_string(),
                    HttpProgress::Sending => "Sending...".to_string(),
                    HttpProgress::WaitingResponse => "Waiting for response...".to_string(),
                    HttpProgress::Downloading { received_bytes, total_bytes } => {
                        if let Some(total) = total_bytes {
                            format!("Downloading ({}/{} B)...", received_bytes, total)
                        } else {
                            format!("Downloading ({} B)...", received_bytes)
                        }
                    }
                };
                let _ = progress_event_tx.send(HttpSendEvent::Progress(text));
            }
        });

        // Spawn background HTTP send task on tokio runtime
        rt.spawn(async move {
            tokio::select! {
                _ = cancel_rx => {
                    let _ = send_event_tx.send(HttpSendEvent::Cancelled);
                }
                res = executor.send(resolved, Some(progress_tx)) => {
                    match res {
                        Ok(record) => {
                            let _ = send_event_tx.send(HttpSendEvent::Success(Box::new(record)));
                        }
                        Err(HttpError::Cancelled) => {
                            let _ = send_event_tx.send(HttpSendEvent::Cancelled);
                        }
                        Err(e) => {
                            let _ = send_event_tx.send(HttpSendEvent::Error(e.to_string()));
                        }
                    }
                }
            }
        });

        // Spawn GPUI background task to receive events from channel and update UI
        cx.spawn(async move |this, cx| {
            while let Some(event) = send_event_rx.recv().await {
                if let Some(this) = this.upgrade() {
                    this.update(cx, |this, cx| {
                        match event {
                            HttpSendEvent::Progress(text) => {
                                this.editor_state.update(cx, |ed, cx| {
                                    ed.update_progress(text, cx);
                                });
                            }
                            HttpSendEvent::Success(record) => {
                                let status = record.status_code;
                                let req_id = record.request_id.clone();
                                let history_storage = this.workspace_state.read(cx).history_storage.clone();
                                let _ = history_storage.save_response(&record, 20);
                                let history = history_storage.get_responses(&req_id, 20).unwrap_or_default();

                                this.response_state.update(cx, |resp, cx| {
                                    resp.set_response(Some(*record), history, cx);
                                });

                                this.workspace_state.update(cx, |ws, _cx| {
                                    if let Some(r) = ws.requests.iter_mut().find(|r| r.id == req_id) {
                                        r.last_status_code = Some(status);
                                        let _ = ws.fs_storage.save_request(r);
                                    }
                                });

                                this.editor_state.update(cx, |ed, cx| {
                                    if let Some(r) = &mut ed.active_request {
                                        if r.id == req_id {
                                            r.last_status_code = Some(status);
                                        }
                                    }
                                    ed.stop_sending(cx);
                                });
                            }
                            HttpSendEvent::Cancelled => {
                                this.editor_state.update(cx, |ed, cx| {
                                    ed.stop_sending(cx);
                                });
                            }
                            HttpSendEvent::Error(err_msg) => {
                                this.editor_state.update(cx, |ed, cx| {
                                    ed.stop_sending(cx);
                                });
                                this.toast_state.update(cx, |ts, cx| {
                                    ts.push_error("Send Failed", err_msg, cx);
                                });
                            }
                        }
                    });
                }
            }
        }).detach();
    }
}

impl Render for AppView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = theme();

        let sidebar_collapsed = self.workspace_state.read(cx).sidebar_collapsed;
        let is_vertical_split = self.workspace_state.read(cx).split_layout_vertical;
        let show_env_modal = self.workspace_state.read(cx).show_env_modal;
        let show_cookies_modal = self.workspace_state.read(cx).show_cookies_modal;
        let show_command_palette = self.workspace_state.read(cx).show_command_palette;

        let active_tab_kind = self.editor_state.read(cx).active_tab_kind;
        let method_dropdown_open = self.editor_state.read(cx).method_dropdown_open;
        let active_req = self.editor_state.read(cx).active_request.clone();

        // Synchronize active request fields into InputState / TextareaState when switched or updated via query params
        if let Some(req) = &active_req {
            if self.name_input.read(cx).value().as_str() != req.name {
                self.name_input.update(cx, |inp, cx| {
                    inp.set_value(req.name.clone(), window, cx);
                });
            }
            if self.url_input.read(cx).value().as_str() != req.url {
                self.url_input.update(cx, |inp, cx| {
                    inp.set_value(req.url.clone(), window, cx);
                });
            }
            match &req.body {
                RequestBody::Json(json_str) => {
                    if self.json_textarea.read(cx).value().as_str() != json_str {
                        self.json_textarea.update(cx, |txt, cx| {
                            txt.set_value(json_str.clone(), window, cx);
                        });
                    }
                }
                RequestBody::Raw { content, .. } => {
                    if self.raw_textarea.read(cx).value().as_str() != content {
                        self.raw_textarea.update(cx, |txt, cx| {
                            txt.set_value(content.clone(), window, cx);
                        });
                    }
                }
                _ => {}
            }
            match &req.auth {
                Auth::Bearer { token } => {
                    if self.bearer_token_input.read(cx).value().as_str() != token {
                        self.bearer_token_input.update(cx, |inp, cx| {
                            inp.set_value(token.clone(), window, cx);
                        });
                    }
                }
                Auth::Basic { username, password } => {
                    if self.basic_user_input.read(cx).value().as_str() != username {
                        self.basic_user_input.update(cx, |inp, cx| {
                            inp.set_value(username.clone(), window, cx);
                        });
                    }
                    if self.basic_pass_input.read(cx).value().as_str() != password {
                        self.basic_pass_input.update(cx, |inp, cx| {
                            inp.set_value(password.clone(), window, cx);
                        });
                    }
                }
                Auth::ApiKey { key, value, .. } => {
                    if self.apikey_key_input.read(cx).value().as_str() != key {
                        self.apikey_key_input.update(cx, |inp, cx| {
                            inp.set_value(key.clone(), window, cx);
                        });
                    }
                    if self.apikey_val_input.read(cx).value().as_str() != value {
                        self.apikey_val_input.update(cx, |inp, cx| {
                            inp.set_value(value.clone(), window, cx);
                        });
                    }
                }
                Auth::None => {}
            }
        }

        let ws_entity = self.workspace_state.clone();
        let ed_entity = self.editor_state.clone();
        let resp_entity = self.response_state.clone();
        let toast_entity = self.toast_state.clone();

        let app_view_entity = cx.entity().clone();
        let req_id_str = active_req.as_ref().map(|r| r.id.clone()).unwrap_or_else(|| "scratch".to_string());

        div()
            .size_full()
            .bg(t.bg_app)
            .text_color(t.text_primary)
            .flex()
            .flex_col()
            // Keyboard shortcut action handlers
            .on_action({
                let view_handle = app_view_entity.clone();
                move |_: &SendRequest, _, cx| {
                    view_handle.update(cx, |view, cx| {
                        view.send_current_request(cx);
                    });
                }
            })
            .on_action({
                let ws_c = ws_entity.clone();
                move |_: &ToggleCommandPalette, _, cx| {
                    ws_c.update(cx, |state, cx| {
                        state.toggle_command_palette(cx);
                    });
                }
            })
            // Custom Title Bar
            .child(AppTitleBar::new(ws_entity.clone(), ed_entity.clone()))
            // Main workspace content area
            .child(
                div()
                    .flex_1()
                    .w_full()
                    .overflow_hidden()
                    .child(
                        // Outer horizontal resizable: Sidebar | Editor & Response Split
                        h_resizable("outer-split")
                            .when(!sidebar_collapsed, |group| {
                                group.child(
                                    resizable_panel()
                                        .size(px(260.))
                                        .child(SidebarTree::new(ws_entity.clone(), ed_entity.clone())),
                                )
                            })
                            .child(
                                resizable_panel().child(
                                    div().size_full().child({
                                        // Editor & Response split: Horizontal or Vertical
                                        let editor_pane = div()
                                            .size_full()
                                            .bg(t.bg_editor)
                                            .relative()
                                            .flex()
                                            .flex_col()
                                            // Top Request Bar: Method + URL + Send
                                            .child({
                                                let view_handle = app_view_entity.clone();
                                                RequestBar::new(
                                                    ws_entity.clone(),
                                                    ed_entity.clone(),
                                                    self.name_input.clone(),
                                                    self.url_input.clone(),
                                                    move |_, cx| {
                                                        view_handle.update(cx, |view, cx| {
                                                            view.send_current_request(cx);
                                                        });
                                                    },
                                                )
                                            })
                                            // Request Tabs with badges: Body v, Params [count], Headers [count], Auth, Settings
                                            .child(
                                                div()
                                                    .h(px(34.))
                                                    .px(px(8.))
                                                    .border_b_1()
                                                    .border_color(t.border)
                                                    .flex()
                                                    .items_center()
                                                    .gap(px(4.))
                                                    .child({
                                                        let ed_c = ed_entity.clone();
                                                        TabItem::new("tab-body", "Body")
                                                            .active(active_tab_kind == RequestTabKind::Body)
                                                            .with_dropdown(true)
                                                            .on_click(move |_, _, cx| {
                                                                ed_c.update(cx, |state, cx| {
                                                                    state.set_active_tab_kind(RequestTabKind::Body, cx);
                                                                });
                                                            })
                                                    })
                                                    .child({
                                                        let ed_c = ed_entity.clone();
                                                        let param_count = active_req.as_ref().map(|r| r.params.len()).unwrap_or(0);
                                                        TabItem::new("tab-params", "Params")
                                                            .active(active_tab_kind == RequestTabKind::Params)
                                                            .badge(format!("{}", param_count))
                                                            .on_click(move |_, _, cx| {
                                                                ed_c.update(cx, |state, cx| {
                                                                    state.set_active_tab_kind(RequestTabKind::Params, cx);
                                                                });
                                                            })
                                                    })
                                                    .child({
                                                        let ed_c = ed_entity.clone();
                                                        let header_count = active_req.as_ref().map(|r| r.headers.len()).unwrap_or(0);
                                                        TabItem::new("tab-headers", "Headers")
                                                            .active(active_tab_kind == RequestTabKind::Headers)
                                                            .badge(format!("{}", header_count))
                                                            .on_click(move |_, _, cx| {
                                                                ed_c.update(cx, |state, cx| {
                                                                    state.set_active_tab_kind(RequestTabKind::Headers, cx);
                                                                });
                                                            })
                                                    })
                                                    .child({
                                                        let ed_c = ed_entity.clone();
                                                        let auth_label = match active_req.as_ref().map(|r| &r.auth) {
                                                            Some(Auth::Bearer { .. }) => "Bearer",
                                                            Some(Auth::Basic { .. }) => "Basic",
                                                            Some(Auth::ApiKey { .. }) => "API Key",
                                                            _ => "Auth",
                                                        };
                                                        TabItem::new("tab-auth", auth_label)
                                                            .active(active_tab_kind == RequestTabKind::Auth)
                                                            .with_dropdown(true)
                                                            .on_click(move |_, _, cx| {
                                                                ed_c.update(cx, |state, cx| {
                                                                    state.set_active_tab_kind(RequestTabKind::Auth, cx);
                                                                });
                                                            })
                                                    })
                                                    .child({
                                                        let ed_c = ed_entity.clone();
                                                        TabItem::new("tab-settings", "Settings")
                                                            .active(active_tab_kind == RequestTabKind::Settings)
                                                            .on_click(move |_, _, cx| {
                                                                ed_c.update(cx, |state, cx| {
                                                                    state.set_active_tab_kind(RequestTabKind::Settings, cx);
                                                                });
                                                            })
                                                    }),
                                            )
                                            // Request Tab Content
                                            .child(
                                                div()
                                                    .flex_1()
                                                    .overflow_hidden()
                                                    .child({
                                                        match active_tab_kind {
                                                            RequestTabKind::Body => {
                                                                div().size_full().child(
                                                                    BodyEditor::new(
                                                                        ed_entity.clone(),
                                                                        self.json_textarea.clone(),
                                                                        self.raw_textarea.clone(),
                                                                    )
                                                                )
                                                            }
                                                            RequestTabKind::Params => {
                                                                let params = active_req.as_ref().map(|r| r.params.clone()).unwrap_or_default();
                                                                let ed_up = ed_entity.clone();
                                                                let ed_add = ed_entity.clone();
                                                                let ed_del = ed_entity.clone();
                                                                div().size_full().child(
                                                                    KvTable::new(
                                                                        format!("params-kv-{}", req_id_str),
                                                                        params,
                                                                        move |idx, k, v, enabled, _, cx| {
                                                                            ed_up.update(cx, |state, cx| {
                                                                                state.update_param(idx, k, v, enabled, cx);
                                                                            });
                                                                        },
                                                                        move |_, cx| {
                                                                            ed_add.update(cx, |state, cx| {
                                                                                state.add_param(cx);
                                                                            });
                                                                        },
                                                                        move |idx, _, cx| {
                                                                            ed_del.update(cx, |state, cx| {
                                                                                state.remove_param(idx, cx);
                                                                            });
                                                                        },
                                                                    )
                                                                )
                                                            }
                                                            RequestTabKind::Headers => {
                                                                let headers = active_req.as_ref().map(|r| r.headers.clone()).unwrap_or_default();
                                                                let ed_up = ed_entity.clone();
                                                                let ed_add = ed_entity.clone();
                                                                let ed_del = ed_entity.clone();
                                                                div().size_full().child(
                                                                    KvTable::new(
                                                                        format!("headers-kv-{}", req_id_str),
                                                                        headers,
                                                                        move |idx, k, v, enabled, _, cx| {
                                                                            ed_up.update(cx, |state, cx| {
                                                                                state.update_header(idx, k, v, enabled, cx);
                                                                            });
                                                                        },
                                                                        move |_, cx| {
                                                                            ed_add.update(cx, |state, cx| {
                                                                                state.add_header(cx);
                                                                            });
                                                                        },
                                                                        move |idx, _, cx| {
                                                                            ed_del.update(cx, |state, cx| {
                                                                                state.remove_header(idx, cx);
                                                                            });
                                                                        },
                                                                    )
                                                                )
                                                            }
                                                            RequestTabKind::Auth => {
                                                                let auth = active_req.as_ref().map(|r| r.auth.clone()).unwrap_or(Auth::None);
                                                                let ed_c = ed_entity.clone();
                                                                div()
                                                                    .p(px(16.))
                                                                    .flex()
                                                                    .flex_col()
                                                                    .gap(px(12.))
                                                                    .child(div().text_sm().font_weight(gpui::FontWeight::SEMIBOLD).child("Authentication"))
                                                                    .child(
                                                                        div()
                                                                            .flex()
                                                                            .gap(px(8.))
                                                                            .child({
                                                                                let ed_s = ed_c.clone();
                                                                                div()
                                                                                    .id("auth-type-none")
                                                                                    .px(px(10.))
                                                                                    .py(px(4.))
                                                                                    .rounded(px(4.))
                                                                                    .bg(if matches!(auth, Auth::None) { t.primary } else { t.bg_card })
                                                                                    .text_color(if matches!(auth, Auth::None) { t.primary_foreground } else { t.text_secondary })
                                                                                    .text_xs()
                                                                                    .cursor_pointer()
                                                                                    .on_click(move |_, _, cx| {
                                                                                        ed_s.update(cx, |state, cx| {
                                                                                            state.set_auth(Auth::None, cx);
                                                                                        });
                                                                                    })
                                                                                    .child("None")
                                                                            })
                                                                            .child({
                                                                                let ed_s = ed_c.clone();
                                                                                div()
                                                                                    .id("auth-type-bearer")
                                                                                    .px(px(10.))
                                                                                    .py(px(4.))
                                                                                    .rounded(px(4.))
                                                                                    .bg(if matches!(auth, Auth::Bearer { .. }) { t.primary } else { t.bg_card })
                                                                                    .text_color(if matches!(auth, Auth::Bearer { .. }) { t.primary_foreground } else { t.text_secondary })
                                                                                    .text_xs()
                                                                                    .cursor_pointer()
                                                                                    .on_click(move |_, _, cx| {
                                                                                        ed_s.update(cx, |state, cx| {
                                                                                            state.set_auth(Auth::Bearer { token: "".to_string() }, cx);
                                                                                        });
                                                                                    })
                                                                                    .child("Bearer Token")
                                                                            })
                                                                            .child({
                                                                                let ed_s = ed_c.clone();
                                                                                div()
                                                                                    .id("auth-type-basic")
                                                                                    .px(px(10.))
                                                                                    .py(px(4.))
                                                                                    .rounded(px(4.))
                                                                                    .bg(if matches!(auth, Auth::Basic { .. }) { t.primary } else { t.bg_card })
                                                                                    .text_color(if matches!(auth, Auth::Basic { .. }) { t.primary_foreground } else { t.text_secondary })
                                                                                    .text_xs()
                                                                                    .cursor_pointer()
                                                                                    .on_click(move |_, _, cx| {
                                                                                        ed_s.update(cx, |state, cx| {
                                                                                            state.set_auth(Auth::Basic { username: "".to_string(), password: "".to_string() }, cx);
                                                                                        });
                                                                                    })
                                                                                    .child("Basic Auth")
                                                                            })
                                                                            .child({
                                                                                let ed_s = ed_c.clone();
                                                                                div()
                                                                                    .id("auth-type-apikey")
                                                                                    .px(px(10.))
                                                                                    .py(px(4.))
                                                                                    .rounded(px(4.))
                                                                                    .bg(if matches!(auth, Auth::ApiKey { .. }) { t.primary } else { t.bg_card })
                                                                                    .text_color(if matches!(auth, Auth::ApiKey { .. }) { t.primary_foreground } else { t.text_secondary })
                                                                                    .text_xs()
                                                                                    .cursor_pointer()
                                                                                    .on_click(move |_, _, cx| {
                                                                                        ed_s.update(cx, |state, cx| {
                                                                                            state.set_auth(Auth::ApiKey { key: "X-API-Key".to_string(), value: "".to_string(), in_header: true }, cx);
                                                                                        });
                                                                                    })
                                                                                    .child("API Key")
                                                                            })
                                                                    )
                                                                    .child({
                                                                        match auth {
                                                                            Auth::None => {
                                                                                div()
                                                                                    .pt(px(12.))
                                                                                    .text_xs()
                                                                                    .italic()
                                                                                    .text_color(t.text_muted)
                                                                                    .child("No authentication will be added to this request.")
                                                                            }
                                                                            Auth::Bearer { .. } => {
                                                                                div()
                                                                                    .pt(px(12.))
                                                                                    .flex()
                                                                                    .flex_col()
                                                                                    .gap(px(8.))
                                                                                    .child(div().text_xs().font_weight(gpui::FontWeight::SEMIBOLD).text_color(t.text_muted).child("Token:"))
                                                                                    .child(
                                                                                        div()
                                                                                            .h(px(32.))
                                                                                            .px(px(6.))
                                                                                            .rounded(px(4.))
                                                                                            .bg(t.bg_input)
                                                                                            .border_1()
                                                                                            .border_color(t.border)
                                                                                            .flex()
                                                                                            .items_center()
                                                                                            .child(
                                                                                                Input::new(&self.bearer_token_input)
                                                                                                    .id("auth-bearer-token-input")
                                                                                                    .appearance(false)
                                                                                                    .bordered(false)
                                                                                                    .focus_bordered(false)
                                                                                                    .cleanable(true),
                                                                                            ),
                                                                                    )
                                                                            }
                                                                            Auth::Basic { .. } => {
                                                                                div()
                                                                                    .pt(px(12.))
                                                                                    .flex()
                                                                                    .flex_col()
                                                                                    .gap(px(8.))
                                                                                    .child(div().text_xs().font_weight(gpui::FontWeight::SEMIBOLD).text_color(t.text_muted).child("Username:"))
                                                                                    .child(
                                                                                        div()
                                                                                            .h(px(32.))
                                                                                            .px(px(6.))
                                                                                            .rounded(px(4.))
                                                                                            .bg(t.bg_input)
                                                                                            .border_1()
                                                                                            .border_color(t.border)
                                                                                            .flex()
                                                                                            .items_center()
                                                                                            .child(
                                                                                                Input::new(&self.basic_user_input)
                                                                                                    .id("auth-basic-user-input")
                                                                                                    .appearance(false)
                                                                                                    .bordered(false)
                                                                                                    .focus_bordered(false)
                                                                                                    .cleanable(true),
                                                                                            ),
                                                                                    )
                                                                                    .child(div().text_xs().font_weight(gpui::FontWeight::SEMIBOLD).text_color(t.text_muted).child("Password:"))
                                                                                    .child(
                                                                                        div()
                                                                                            .h(px(32.))
                                                                                            .px(px(6.))
                                                                                            .rounded(px(4.))
                                                                                            .bg(t.bg_input)
                                                                                            .border_1()
                                                                                            .border_color(t.border)
                                                                                            .flex()
                                                                                            .items_center()
                                                                                            .child(
                                                                                                Input::new(&self.basic_pass_input)
                                                                                                    .id("auth-basic-pass-input")
                                                                                                    .appearance(false)
                                                                                                    .bordered(false)
                                                                                                    .focus_bordered(false)
                                                                                                    .cleanable(true),
                                                                                            ),
                                                                                    )
                                                                            }
                                                                            Auth::ApiKey { key, value, in_header } => {
                                                                                let ed_loc_h = ed_c.clone();
                                                                                let ed_loc_q = ed_c.clone();
                                                                                let k_h = key.clone();
                                                                                let v_h = value.clone();
                                                                                let k_q = key.clone();
                                                                                let v_q = value.clone();
                                                                                div()
                                                                                    .pt(px(12.))
                                                                                    .flex()
                                                                                    .flex_col()
                                                                                    .gap(px(8.))
                                                                                    .child(div().text_xs().font_weight(gpui::FontWeight::SEMIBOLD).text_color(t.text_muted).child("Header / Param Key:"))
                                                                                    .child(
                                                                                        div()
                                                                                            .h(px(32.))
                                                                                            .px(px(6.))
                                                                                            .rounded(px(4.))
                                                                                            .bg(t.bg_input)
                                                                                            .border_1()
                                                                                            .border_color(t.border)
                                                                                            .flex()
                                                                                            .items_center()
                                                                                            .child(
                                                                                                Input::new(&self.apikey_key_input)
                                                                                                    .id("auth-apikey-key-input")
                                                                                                    .appearance(false)
                                                                                                    .bordered(false)
                                                                                                    .focus_bordered(false)
                                                                                                    .cleanable(true),
                                                                                            ),
                                                                                    )
                                                                                    .child(div().text_xs().font_weight(gpui::FontWeight::SEMIBOLD).text_color(t.text_muted).child("Value:"))
                                                                                    .child(
                                                                                        div()
                                                                                            .h(px(32.))
                                                                                            .px(px(6.))
                                                                                            .rounded(px(4.))
                                                                                            .bg(t.bg_input)
                                                                                            .border_1()
                                                                                            .border_color(t.border)
                                                                                            .flex()
                                                                                            .items_center()
                                                                                            .child(
                                                                                                Input::new(&self.apikey_val_input)
                                                                                                    .id("auth-apikey-val-input")
                                                                                                    .appearance(false)
                                                                                                    .bordered(false)
                                                                                                    .focus_bordered(false)
                                                                                                    .cleanable(true),
                                                                                            ),
                                                                                    )
                                                                                    .child(
                                                                                        div()
                                                                                            .pt(px(4.))
                                                                                            .flex()
                                                                                            .items_center()
                                                                                            .gap(px(8.))
                                                                                            .text_xs()
                                                                                            .text_color(t.text_muted)
                                                                                            .child("Add to:")
                                                                                            .child(
                                                                                                div()
                                                                                                    .id("auth-apikey-loc-header")
                                                                                                    .px(px(8.))
                                                                                                    .py(px(3.))
                                                                                                    .rounded(px(4.))
                                                                                                    .bg(if in_header { t.primary } else { t.bg_card })
                                                                                                    .text_color(if in_header { t.primary_foreground } else { t.text_secondary })
                                                                                                    .cursor_pointer()
                                                                                                    .on_click(move |_, _, cx| {
                                                                                                        ed_loc_h.update(cx, |st, cx| {
                                                                                                            st.set_api_key(k_h.clone(), v_h.clone(), true, cx);
                                                                                                        });
                                                                                                    })
                                                                                                    .child("Header"),
                                                                                            )
                                                                                            .child(
                                                                                                div()
                                                                                                    .id("auth-apikey-loc-query")
                                                                                                    .px(px(8.))
                                                                                                    .py(px(3.))
                                                                                                    .rounded(px(4.))
                                                                                                    .bg(if !in_header { t.primary } else { t.bg_card })
                                                                                                    .text_color(if !in_header { t.primary_foreground } else { t.text_secondary })
                                                                                                    .cursor_pointer()
                                                                                                    .on_click(move |_, _, cx| {
                                                                                                        ed_loc_q.update(cx, |st, cx| {
                                                                                                            st.set_api_key(k_q.clone(), v_q.clone(), false, cx);
                                                                                                        });
                                                                                                    })
                                                                                                    .child("Query Param"),
                                                                                            ),
                                                                                    )
                                                                            }
                                                                        }
                                                                    })
                                                            }
                                                            RequestTabKind::Settings => {
                                                                let ed_s = ed_entity.clone();
                                                                let settings = active_req.as_ref().map(|r| r.settings.clone()).unwrap_or_default();

                                                                div()
                                                                    .p(px(16.))
                                                                    .flex()
                                                                    .flex_col()
                                                                    .gap(px(14.))
                                                                    .child(div().text_sm().font_weight(gpui::FontWeight::SEMIBOLD).child("Request Configuration & Network Settings"))
                                                                    // Timeout presets
                                                                    .child(
                                                                        div()
                                                                            .flex()
                                                                            .flex_col()
                                                                            .gap(px(6.))
                                                                            .child(div().text_xs().font_weight(gpui::FontWeight::SEMIBOLD).text_color(t.text_muted).child("Request Timeout:"))
                                                                            .child(
                                                                                div()
                                                                                    .flex()
                                                                                    .gap(px(6.))
                                                                                    .children([10, 30, 60, 120].into_iter().map(|s| {
                                                                                        let is_cur = settings.timeout_secs == s;
                                                                                        let ed_set = ed_s.clone();
                                                                                        div()
                                                                                            .id(format!("timeout-{}s", s))
                                                                                            .px(px(8.))
                                                                                            .py(px(4.))
                                                                                            .rounded(px(4.))
                                                                                            .bg(if is_cur { t.primary } else { t.bg_card })
                                                                                            .text_color(if is_cur { t.primary_foreground } else { t.text_secondary })
                                                                                            .cursor_pointer()
                                                                                            .text_xs()
                                                                                            .on_click(move |_, _, cx| {
                                                                                                ed_set.update(cx, |st, cx| {
                                                                                                    st.set_timeout_secs(s, cx);
                                                                                                });
                                                                                            })
                                                                                            .child(format!("{}s", s))
                                                                                    }))
                                                                            )
                                                                    )
                                                                    // Follow Redirects toggle
                                                                    .child({
                                                                        let ed_set = ed_s.clone();
                                                                        let cur_follow = settings.follow_redirects;
                                                                        div()
                                                                            .flex()
                                                                            .items_center()
                                                                            .gap(px(8.))
                                                                            .child(
                                                                                div()
                                                                                    .id("toggle-follow-redirects")
                                                                                    .px(px(8.))
                                                                                    .py(px(4.))
                                                                                    .rounded(px(4.))
                                                                                    .bg(if cur_follow { t.primary } else { t.bg_card })
                                                                                    .text_color(if cur_follow { t.primary_foreground } else { t.text_secondary })
                                                                                    .cursor_pointer()
                                                                                    .text_xs()
                                                                                    .on_click(move |_, _, cx| {
                                                                                        ed_set.update(cx, |st, cx| {
                                                                                            st.set_follow_redirects(!cur_follow, cx);
                                                                                        });
                                                                                    })
                                                                                    .child(if cur_follow { "✓ Follow Redirects Enabled" } else { "Follow Redirects Disabled" })
                                                                            )
                                                                    })
                                                                    // Validate TLS Certificates toggle
                                                                    .child({
                                                                        let ed_set = ed_s.clone();
                                                                        let cur_tls = settings.validate_tls;
                                                                        div()
                                                                            .flex()
                                                                            .items_center()
                                                                            .gap(px(8.))
                                                                            .child(
                                                                                div()
                                                                                    .id("toggle-validate-tls")
                                                                                    .px(px(8.))
                                                                                    .py(px(4.))
                                                                                    .rounded(px(4.))
                                                                                    .bg(if cur_tls { t.primary } else { t.bg_card })
                                                                                    .text_color(if cur_tls { t.primary_foreground } else { t.text_secondary })
                                                                                    .cursor_pointer()
                                                                                    .text_xs()
                                                                                    .on_click(move |_, _, cx| {
                                                                                        ed_set.update(cx, |st, cx| {
                                                                                            st.set_validate_tls(!cur_tls, cx);
                                                                                        });
                                                                                    })
                                                                                    .child(if cur_tls { "✓ Validate TLS Certificates Enabled" } else { "Validate TLS Certificates Disabled" })
                                                                            )
                                                                    })
                                                            }
                                                        }
                                                    }),
                                            )
                                            // Method dropdown if open (rendered LAST inside editor_pane so it paints on top of tabs and editor content)
                                            .when(method_dropdown_open, |this| {
                                                let ed_c = ed_entity.clone();
                                                this.child(
                                                    div()
                                                        .absolute()
                                                        .top(px(42.))
                                                        .left(px(12.))
                                                        .w(px(110.))
                                                        .bg(t.bg_card)
                                                        .border_1()
                                                        .border_color(t.border)
                                                        .rounded(px(6.))
                                                        .shadow_xl()
                                                        .p(px(4.))
                                                        .flex()
                                                        .flex_col()
                                                        .gap(px(2.))
                                                        .children([
                                                            HttpMethod::Get,
                                                            HttpMethod::Post,
                                                            HttpMethod::Put,
                                                            HttpMethod::Delete,
                                                            HttpMethod::Patch,
                                                            HttpMethod::Head,
                                                            HttpMethod::Options,
                                                        ].into_iter().map(|m| {
                                                            let ed_set = ed_c.clone();
                                                            div()
                                                                .id(format!("select-method-{}", m.as_str()))
                                                                .px(px(8.))
                                                                .py(px(5.))
                                                                .rounded(px(4.))
                                                                .cursor_pointer()
                                                                .hover(|s| s.bg(t.bg_hover))
                                                                .text_xs()
                                                                .font_weight(gpui::FontWeight::BOLD)
                                                                .text_color(t.method_color(m))
                                                                .on_click(move |_, _, cx| {
                                                                    ed_set.update(cx, |state, cx| {
                                                                        state.set_method(m, cx);
                                                                    });
                                                                })
                                                                .child(m.as_str())
                                                        }))
                                                )
                                            });

                                        let response_pane = ResponseViewer::new(ed_entity.clone(), resp_entity.clone());

                                        if is_vertical_split {
                                            div().size_full().child(
                                                v_resizable("inner-vertical-split")
                                                    .child(resizable_panel().child(editor_pane))
                                                    .child(resizable_panel().child(response_pane)),
                                            )
                                        } else {
                                            div().size_full().child(
                                                h_resizable("inner-horizontal-split")
                                                    .child(resizable_panel().child(editor_pane))
                                                    .child(resizable_panel().child(response_pane)),
                                            )
                                        }
                                    }),
                                ),
                            ),
                    ),
            )
            // Modals & Overlays
            .when(show_env_modal, |this| {
                this.child(EnvironmentModal::new(ws_entity.clone()))
            })
            .when(show_cookies_modal, |this| {
                this.child(CookiesModal::new(ws_entity.clone()))
            })
            .when(show_command_palette, |this| {
                let view_handle = app_view_entity.clone();
                this.child(CommandPalette::new(ws_entity.clone(), ed_entity.clone(), move |_, cx| {
                    view_handle.update(cx, |view, cx| {
                        view.send_current_request(cx);
                    });
                }))
            })
            // Toast Notifications container bottom-right
            .child(ToastContainer::new(toast_entity))
    }
}

fn main() {
    let app = gpui_kit::application().with_assets(Assets);

    app.run(move |cx| {
        gpui_kit::init(cx);
        gpui_component::Theme::change(gpui_component::ThemeMode::Dark, None, cx);

        // Register keyboard shortcuts
        cx.bind_keys([
            KeyBinding::new("ctrl-enter", SendRequest, None),
            KeyBinding::new("cmd-enter", SendRequest, None),
            KeyBinding::new("ctrl-k", ToggleCommandPalette, None),
            KeyBinding::new("cmd-k", ToggleCommandPalette, None),
        ]);

        cx.on_window_closed(|cx, _window_id| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();

        let window_options = default_window_options(cx);

        gpui_kit::open_window(window_options, cx, |window, cx| {
            cx.new(|cx| AppView::new(window, cx))
        })
        .expect("Failed to open main window");
    });
}

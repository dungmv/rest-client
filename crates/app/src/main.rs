#![allow(dead_code, clippy::type_complexity)]

mod components;
mod platform;
mod state;
mod theme;

use components::*;
use gpui::prelude::FluentBuilder as _;
use gpui::*;
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
    executor: Arc<ReqwestExecutor>,
    tokio_rt: Arc<tokio::runtime::Runtime>,
    _subscriptions: Vec<Subscription>,
}

impl AppView {
    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let workspace_state = cx.new(|_| WorkspaceState::new());
        let editor_state = cx.new(|_| EditorState::new());
        let response_state = cx.new(|_| ResponseState::new());
        let toast_state = cx.new(|_| ToastState::new());

        let executor = Arc::new(ReqwestExecutor::new());
        let tokio_rt = Arc::new(
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .expect("Failed to create tokio runtime"),
        );

        // Select initial request from workspace into editor
        let initial_req = workspace_state.read(cx).requests.first().cloned();
        if let Some(req) = initial_req {
            editor_state.update(cx, |ed, cx| {
                ed.set_active_request(req.clone(), true, cx);
            });
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

        // Add sample update toast matching reference screenshot
        toast_state.update(cx, |ts, cx| {
            ts.push_toast(
                ToastItem {
                    id: "update-banner".to_string(),
                    kind: ToastKind::Info,
                    title: "Yaak 2026.9.0 is available".to_string(),
                    message: "Do you want to install the update?".to_string(),
                    action_label: Some("Install Now".to_string()),
                    action_fn: None,
                },
                cx,
            );
        });

        // Set up subscriptions: when editor changes active request, reload response history
        let mut _subscriptions = Vec::new();
        let ws_clone = workspace_state.clone();
        let resp_clone = response_state.clone();
        _subscriptions.push(cx.observe(&editor_state, move |_, ed, cx| {
            if let Some(req) = &ed.read(cx).active_request {
                let req_id = req.id.clone();
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
        }));

        Self {
            workspace_state,
            editor_state,
            response_state,
            toast_state,
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
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = theme();
        let ws = self.workspace_state.read(cx);
        let ed = self.editor_state.read(cx);

        let sidebar_collapsed = ws.sidebar_collapsed;
        let is_vertical_split = ws.split_layout_vertical;
        let active_tab_kind = ed.active_tab_kind;
        let active_req = ed.active_request.clone();

        let ws_entity = self.workspace_state.clone();
        let ed_entity = self.editor_state.clone();
        let resp_entity = self.response_state.clone();
        let toast_entity = self.toast_state.clone();

        let app_view_entity = cx.entity().clone();

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
                                            .flex()
                                            .flex_col()
                                            // Top Request Bar: Method + URL + Send
                                            .child({
                                                let view_handle = app_view_entity.clone();
                                                RequestBar::new(ws_entity.clone(), ed_entity.clone(), move |_, cx| {
                                                    view_handle.update(cx, |view, cx| {
                                                        view.send_current_request(cx);
                                                    });
                                                })
                                            })
                                            // Method dropdown if open
                                            .when(ed.method_dropdown_open, |this| {
                                                let ed_c = ed_entity.clone();
                                                this.child(
                                                    div()
                                                        .absolute()
                                                        .top(px(40.))
                                                        .left(px(8.))
                                                        .bg(t.bg_card)
                                                        .border_1()
                                                        .border_color(t.border)
                                                        .rounded(px(6.))
                                                        .shadow_lg()
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
                                                                .py(px(4.))
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
                                            })
                                            // Request Tabs with badges: Body v, Params [1], Headers [3], Bearer v, Settings
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
                                                        let param_count = active_req.as_ref().map(|r| r.params.len()).unwrap_or(1);
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
                                                        let header_count = active_req.as_ref().map(|r| r.headers.len()).unwrap_or(3);
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
                                                                let body = active_req.as_ref().map(|r| r.body.clone()).unwrap_or(RequestBody::None);
                                                                let ed_c = ed_entity.clone();
                                                                div().size_full().child(
                                                                    BodyEditor::new(body, move |json_str, _, cx| {
                                                                        ed_c.update(cx, |state, cx| {
                                                                            state.set_body_json(json_str, cx);
                                                                        });
                                                                    })
                                                                )
                                                            }
                                                            RequestTabKind::Params => {
                                                                let params = active_req.as_ref().map(|r| r.params.clone()).unwrap_or_default();
                                                                let ed_up = ed_entity.clone();
                                                                let ed_add = ed_entity.clone();
                                                                let ed_del = ed_entity.clone();
                                                                div().size_full().child(
                                                                    KvTable::new(
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
                                                                ))
                                                            }
                                                            RequestTabKind::Headers => {
                                                                let headers = active_req.as_ref().map(|r| r.headers.clone()).unwrap_or_default();
                                                                let ed_up = ed_entity.clone();
                                                                let ed_add = ed_entity.clone();
                                                                let ed_del = ed_entity.clone();
                                                                div().size_full().child(
                                                                    KvTable::new(
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
                                                                ))
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
                                                                                    .px(px(8.))
                                                                                    .py(px(4.))
                                                                                    .rounded(px(4.))
                                                                                    .bg(if matches!(auth, Auth::None) { t.primary } else { t.bg_card })
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
                                                                                    .px(px(8.))
                                                                                    .py(px(4.))
                                                                                    .rounded(px(4.))
                                                                                    .bg(if matches!(auth, Auth::Bearer { .. }) { t.primary } else { t.bg_card })
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
                                                                                    .px(px(8.))
                                                                                    .py(px(4.))
                                                                                    .rounded(px(4.))
                                                                                    .bg(if matches!(auth, Auth::Basic { .. }) { t.primary } else { t.bg_card })
                                                                                    .text_xs()
                                                                                    .cursor_pointer()
                                                                                    .on_click(move |_, _, cx| {
                                                                                        ed_s.update(cx, |state, cx| {
                                                                                            state.set_auth(Auth::Basic { username: "".to_string(), password: "".to_string() }, cx);
                                                                                        });
                                                                                    })
                                                                                    .child("Basic Auth")
                                                                            })
                                                                    )
                                                            }
                                                            RequestTabKind::Settings => {
                                                                div()
                                                                    .p(px(16.))
                                                                    .flex()
                                                                    .flex_col()
                                                                    .gap(px(8.))
                                                                    .child(div().text_sm().font_weight(gpui::FontWeight::SEMIBOLD).child("Request Settings"))
                                                                    .child(div().text_xs().text_color(t.text_secondary).child("• Timeout: 30s"))
                                                                    .child(div().text_xs().text_color(t.text_secondary).child("• Follow Redirects: Enabled"))
                                                                    .child(div().text_xs().text_color(t.text_secondary).child("• Validate TLS: Enabled"))
                                                            }
                                                        }
                                                    }),
                                            );

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
            .when(ws.show_env_modal, |this| {
                this.child(EnvironmentModal::new(ws_entity.clone()))
            })
            .when(ws.show_cookies_modal, |this| {
                this.child(CookiesModal::new(ws_entity.clone()))
            })
            .when(ws.show_command_palette, |this| {
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

        // Register keyboard shortcuts
        cx.bind_keys([
            KeyBinding::new("ctrl-enter", SendRequest, None),
            KeyBinding::new("cmd-enter", SendRequest, None),
            KeyBinding::new("ctrl-k", ToggleCommandPalette, None),
            KeyBinding::new("cmd-k", ToggleCommandPalette, None),
        ]);

        let window_options = default_window_options(cx);

        gpui_kit::open_window(window_options, cx, |window, cx| {
            cx.new(|cx| AppView::new(window, cx))
        })
        .expect("Failed to open main window");
    });
}

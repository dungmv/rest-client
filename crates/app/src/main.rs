#![allow(dead_code, clippy::type_complexity)]

mod components;
mod platform;
pub mod settings;
mod state;
mod theme;

use components::*;
use gpui::prelude::FluentBuilder as _;
use gpui::*;
use gpui_component::input::{
    EditorState as CodeEditorState, Input, InputEvent, InputState, TextareaState,
};
use gpui_component::resizable::{h_resizable, resizable_panel, v_resizable};
use gpui_kit::assets::Assets;
use platform::default_window_options;
use restclient_core::http::{HttpError, HttpExecutor, HttpProgress, ReqwestExecutor, ResolvedRequest};
use restclient_core::models::{
    Auth, Environment, EnvironmentVariable, HttpMethod, HttpResponseRecord, RequestBody,
};
use restclient_core::scripting::{
    resolve_fake_vars, PostResponseContext, PostResponseOutput, PreRequestContext, PreRequestOutput,
    ScriptSandbox,
};
use state::*;
use std::sync::Arc;
use theme::theme;

actions!(restclient, [SendRequest, ToggleCommandPalette, ToggleSidebar, ToggleSettings, NavigateBack, NavigateForward]);

pub enum HttpSendEvent {
    Progress(String),
    Success(Box<HttpResponseRecord>, PostResponseOutput),
    Cancelled,
    Error(String),
}

pub struct AppView {
    workspace_state: Entity<WorkspaceState>,
    editor_state: Entity<EditorState>,
    response_state: Entity<ResponseState>,
    toast_state: Entity<ToastState>,
    url_input: Entity<InputState>,
    json_editor: Entity<CodeEditorState>,
    raw_textarea: Entity<TextareaState>,
    pre_script_editor: Entity<CodeEditorState>,
    test_script_editor: Entity<CodeEditorState>,
    bearer_token_input: Entity<InputState>,
    basic_user_input: Entity<InputState>,
    basic_pass_input: Entity<InputState>,
    apikey_key_input: Entity<InputState>,
    apikey_val_input: Entity<InputState>,
    executor: Arc<ReqwestExecutor>,
    tokio_rt: Arc<tokio::runtime::Runtime>,
    last_synced_request_id: Option<String>,
    last_synced_body_version: usize,
    _subscriptions: Vec<Subscription>,
}

impl AppView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let workspace_state = cx.new(|cx| {
            let mut s = WorkspaceState::new();
            s.start_watcher(cx);
            s
        });
        let editor_state = cx.new(|_| EditorState::new());
        let response_state = cx.new(|_| ResponseState::new());
        let toast_state = cx.new(|_| ToastState::new());

        let url_input = cx.new(|cx| InputState::new(window, cx).placeholder("Enter request URL (e.g. https://httpbin.org/get)"));
        let json_editor = cx.new(|cx| {
            let mut state = CodeEditorState::new(window, cx);
            state.set_highlighter_factory(
                std::rc::Rc::new(|lang: &str| {
                    if lang == "json" {
                        Some(Box::new(crate::components::body_editor::JsonHighlighter::new()))
                    } else {
                        None
                    }
                }),
                cx,
            );
            state.lsp_mut().completion_provider = Some(std::rc::Rc::new(
                crate::components::env_autocomplete::EnvCompletionProvider::new(workspace_state.clone()),
            ));
            state.language("json").folding(false)
        });
        let raw_textarea = cx.new(|cx| TextareaState::new(window, cx));
        let pre_script_editor = cx.new(|cx| {
            let mut state = CodeEditorState::new(window, cx);
            state.set_highlighter_factory(
                std::rc::Rc::new(|lang: &str| {
                    if lang == "javascript" || lang == "js" {
                        Some(Box::new(JavaScriptHighlighter::new()))
                    } else {
                        None
                    }
                }),
                cx,
            );
            state.lsp_mut().completion_provider = Some(std::rc::Rc::new(
                ScriptCompletionProvider::new(ScriptEditorKind::PreRequest),
            ));
            state.language("javascript").line_number(true).folding(false)
        });
        let test_script_editor = cx.new(|cx| {
            let mut state = CodeEditorState::new(window, cx);
            state.set_highlighter_factory(
                std::rc::Rc::new(|lang: &str| {
                    if lang == "javascript" || lang == "js" {
                        Some(Box::new(JavaScriptHighlighter::new()))
                    } else {
                        None
                    }
                }),
                cx,
            );
            state.lsp_mut().completion_provider = Some(std::rc::Rc::new(
                ScriptCompletionProvider::new(ScriptEditorKind::Tests),
            ));
            state.language("javascript").line_number(true).folding(false)
        });

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

            let req_url = req.url.clone();
            let tokenized_url = {
                let ws = workspace_state.read(cx);
                crate::components::env_autocomplete::build_tokenized_content(&req_url, ws)
            };
            url_input.update(cx, |inp, cx| {
                inp.set_value(tokenized_url, window, cx);
            });

            if let RequestBody::Json(json_content) = &req.body {
                json_editor.update(cx, |txt, cx| {
                    txt.set_value(json_content.clone(), window, cx);
                });
            } else if let RequestBody::Raw { content, .. } = &req.body {
                raw_textarea.update(cx, |txt, cx| {
                    txt.set_value(content.clone(), window, cx);
                });
            }

            if let Some(script) = &req.script {
                if let Some(pre) = &script.pre_request {
                    pre_script_editor.update(cx, |txt, cx| {
                        txt.set_value(pre.clone(), window, cx);
                    });
                }
                if let Some(post) = &script.post_response {
                    test_script_editor.update(cx, |txt, cx| {
                        txt.set_value(post.clone(), window, cx);
                    });
                }
            }

            response_state.update(cx, |resp, cx| {
                resp.set_response(None, cx);
            });
        }

        // Set up subscriptions:
        let mut _subscriptions = Vec::new();

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

        // 2. Subscribe to JSON editor change
        let ed_json = editor_state.clone();
        _subscriptions.push(cx.subscribe(&json_editor, move |_, txt, event, cx| {
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

        // 3c. Subscribe to Script editors
        let ed_pre = editor_state.clone();
        _subscriptions.push(cx.subscribe(&pre_script_editor, move |_, txt, event, cx| {
            if let InputEvent::Change = event {
                let val = txt.read(cx).value().to_string();
                ed_pre.update(cx, |ed, cx| {
                    ed.set_pre_script(val, cx);
                });
            }
        }));

        let ed_test = editor_state.clone();
        _subscriptions.push(cx.subscribe(&test_script_editor, move |_, txt, event, cx| {
            if let InputEvent::Change = event {
                let val = txt.read(cx).value().to_string();
                ed_test.update(cx, |ed, cx| {
                    ed.set_test_script(val, cx);
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
                    let ws_store = ws_clone.read(cx).store.clone();
                    resp_clone.update(cx, |r_state, cx| {
                        r_state.load_for_request(&req_id, ws_store.as_ref(), cx);
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

        // 5. Observe workspace_state: synchronize renamed request to editor_state.active_request
        let ed_sync = editor_state.clone();
        _subscriptions.push(cx.observe(&workspace_state, move |_, ws, cx| {
            let active_opt = ed_sync.read(cx).active_request.as_ref().map(|r| (r.id.clone(), r.name.clone()));
            if let Some((active_id, active_name)) = active_opt {
                let saved_name = ws.read(cx).requests.iter().find(|r| r.id == active_id).map(|r| r.name.clone());
                if let Some(saved) = saved_name {
                    if saved != active_name {
                        ed_sync.update(cx, |ed, cx| {
                            ed.set_name(saved, cx);
                        });
                    }
                }
            }
        }));

        let last_synced_request_id = initial_req.as_ref().map(|r| r.id.clone());
        let last_synced_body_version = editor_state.read(cx).body_version;

        if workspace_state.read(cx).git_tracked_warning {
            toast_state.update(cx, |t, cx| {
                t.push_error(
                    "Security Warning",
                    "A git repository was detected with .local/ or *.loc tracked in git. Secrets or response tokens may leak!",
                    cx,
                );
            });
        }

        Self {
            workspace_state,
            editor_state,
            response_state,
            toast_state,
            url_input,
            json_editor,
            raw_textarea,
            pre_script_editor,
            test_script_editor,
            bearer_token_input,
            basic_user_input,
            basic_pass_input,
            apikey_key_input,
            apikey_val_input,
            executor,
            tokio_rt,
            last_synced_request_id,
            last_synced_body_version,
            _subscriptions,
        }
    }

    pub fn send_current_request(&self, cx: &mut Context<Self>) {
        let ed_entity = self.editor_state.clone();
        let ws_entity = self.workspace_state.clone();
        let executor = self.executor.clone();
        let rt = self.tokio_rt.clone();

        let ed = ed_entity.read(cx);
        let Some(mut req_item) = ed.active_request.clone() else {
            return;
        };

        if ed.is_sending {
            return;
        }

        // 1. Resolve Dynamic Fake Variables ({{$uuid}}, {{$timestamp}}, etc.)
        req_item.url = resolve_fake_vars(&req_item.url);
        for p in &mut req_item.params {
            p.key = resolve_fake_vars(&p.key);
            p.value = resolve_fake_vars(&p.value);
        }
        for h in &mut req_item.headers {
            h.key = resolve_fake_vars(&h.key);
            h.value = resolve_fake_vars(&h.value);
        }
        match &mut req_item.body {
            RequestBody::Json(s) => *s = resolve_fake_vars(s),
            RequestBody::Raw { content, .. } => *content = resolve_fake_vars(content),
            RequestBody::FormUrlEncoded(pairs) => {
                for p in pairs {
                    p.key = resolve_fake_vars(&p.key);
                    p.value = resolve_fake_vars(&p.value);
                }
            }
            RequestBody::Multipart(items) => {
                for item in items {
                    item.key = resolve_fake_vars(&item.key);
                    item.value = resolve_fake_vars(&item.value);
                }
            }
            _ => {}
        }

        let ws = ws_entity.read(cx);
        let env = Some(ws.effective_environment());

        // 2. Pre-request Script execution
        let pre_script_code = req_item.script.as_ref().and_then(|s| {
            if s.enabled {
                s.pre_request.clone()
            } else {
                None
            }
        });
        let post_script_code = req_item.script.as_ref().and_then(|s| {
            if s.enabled {
                s.post_response.clone()
            } else {
                None
            }
        });

        let pre_output = if let Some(code) = pre_script_code {
            let pre_ctx = PreRequestContext::from_request_and_env(&req_item, env.as_ref());
            ScriptSandbox::run_pre_request(&code, pre_ctx)
        } else {
            PreRequestOutput::default()
        };

        if let Some(err) = &pre_output.error {
            ed_entity.update(cx, |ed, cx| {
                ed.set_script_error(Some(err.clone()), cx);
            });
        } else {
            ed_entity.update(cx, |ed, cx| {
                ed.set_script_error(None, cx);
            });
        }

        // 3. Merge pre-script env mutations into effective environment
        let mut effective_env = env.clone();
        if !pre_output.env_mutations.is_empty() {
            let mut temp_env = effective_env.unwrap_or_else(|| Environment::new("scratch", "Temp"));
            for (k, v_opt) in &pre_output.env_mutations {
                if let Some(val) = v_opt {
                    if let Some(v_item) = temp_env.variables.iter_mut().find(|v| v.key == *k) {
                        v_item.value = val.clone();
                    } else {
                        temp_env.variables.push(EnvironmentVariable {
                            enabled: true,
                            key: k.clone(),
                            value: val.clone(),
                            secret: false,
                        });
                    }
                } else {
                    temp_env.variables.retain(|v| v.key != *k);
                }
            }
            effective_env = Some(temp_env);
        }

        // 4. Resolve Request with effective environment & apply pre-script mutations
        let mut resolved = match ResolvedRequest::from_item(&req_item, effective_env.as_ref()) {
            Ok(r) => r,
            Err(e) => {
                let err_msg = e.to_string();
                self.toast_state.update(cx, |ts, cx| {
                    ts.push_error("Request Error", err_msg, cx);
                });
                return;
            }
        };
        resolved.apply_pre_script_mutations(&pre_output);

        let (cancel_tx, cancel_rx) = tokio::sync::oneshot::channel();
        ed_entity.update(cx, |state, cx| {
            state.start_sending(cancel_tx, cx);
        });
        let initial_logs = pre_output.console_entries.clone();
        self.response_state.update(cx, |resp, cx| {
            resp.set_pre_request_logs(initial_logs, cx);
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

        let eff_env_for_post = effective_env.clone();
        // Spawn background HTTP send task on tokio runtime
        rt.spawn(async move {
            tokio::select! {
                _ = cancel_rx => {
                    let _ = send_event_tx.send(HttpSendEvent::Cancelled);
                }
                res = executor.send(resolved, Some(progress_tx)) => {
                    match res {
                        Ok(mut record) => {
                            // 5. Post-response / Test Script
                            let post_output = if let Some(code) = post_script_code {
                                let post_ctx = PostResponseContext::from_response_and_env(&record, eff_env_for_post.as_ref());
                                ScriptSandbox::run_post_response(&code, post_ctx)
                            } else {
                                PostResponseOutput::default()
                            };

                            record.test_results = post_output.test_results.clone();
                            let _ = send_event_tx.send(HttpSendEvent::Success(Box::new(record), post_output));
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
                            HttpSendEvent::Success(record, post_output) => {
                                let status = record.status_code;
                                let req_id = record.request_id.clone();

                                // Update workspace environment with mutations (Request chaining)
                                let all_mutations: Vec<_> = pre_output.env_mutations.iter().chain(post_output.env_mutations.iter()).cloned().collect();
                                if !all_mutations.is_empty() {
                                    this.workspace_state.update(cx, |ws, cx| {
                                        if let Some(act_env) = ws.active_environment_mut() {
                                            for (k, v_opt) in &all_mutations {
                                                if let Some(val) = v_opt {
                                                    if let Some(var) = act_env.variables.iter_mut().find(|v| v.key == *k) {
                                                        var.value = val.clone();
                                                    } else {
                                                        act_env.variables.push(EnvironmentVariable {
                                                            enabled: true,
                                                            key: k.clone(),
                                                            value: val.clone(),
                                                            secret: false,
                                                        });
                                                    }
                                                } else {
                                                    act_env.variables.retain(|v| v.key != *k);
                                                }
                                            }
                                            let envs = ws.environments.clone();
                                            ws.save_environments(envs, cx);
                                        } else {
                                            for (k, v_opt) in &all_mutations {
                                                if let Some(val) = v_opt {
                                                    if let Some(var) = ws.global_environment.variables.iter_mut().find(|v| v.key == *k) {
                                                        var.value = val.clone();
                                                    } else {
                                                        ws.global_environment.variables.push(EnvironmentVariable {
                                                            enabled: true,
                                                            key: k.clone(),
                                                            value: val.clone(),
                                                            secret: false,
                                                        });
                                                    }
                                                } else {
                                                    ws.global_environment.variables.retain(|v| v.key != *k);
                                                }
                                            }
                                            let global = ws.global_environment.clone();
                                            ws.save_global_environment(global, cx);
                                        }
                                    });
                                }

                                if let Some(err) = &post_output.error {
                                    this.editor_state.update(cx, |ed, cx| {
                                        ed.set_script_error(Some(err.clone()), cx);
                                    });
                                }

                                let mut combined_console_logs = pre_output.console_entries.clone();
                                combined_console_logs.extend(post_output.console_entries.clone());
                                let mut final_record = *record;
                                final_record.console_logs = combined_console_logs.clone();

                                let loc_file = restclient_core::storage::models::LocFile::from_record(&final_record);
                                this.workspace_state.update(cx, |ws, _cx| {
                                    if let Some(r) = ws.requests.iter_mut().find(|r| r.id == req_id) {
                                        r.last_status_code = Some(status);
                                    }
                                    let _ = ws.store.save_response_cache(&req_id, &loc_file);
                                });

                                this.response_state.update(cx, |resp, cx| {
                                    resp.set_test_results(post_output.test_results, combined_console_logs, cx);
                                    resp.set_response_for_request(req_id.clone(), final_record, cx);
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
        let sidebar_width = self.workspace_state.read(cx).sidebar_width;
        let is_vertical_split = self.workspace_state.read(cx).split_layout_vertical;
        let show_env_modal = self.workspace_state.read(cx).show_env_modal;
        let show_cookies_modal = self.workspace_state.read(cx).show_cookies_modal;
        let show_settings_modal = self.workspace_state.read(cx).show_settings_modal;
        let show_command_palette = self.workspace_state.read(cx).show_command_palette;

        let active_tab_kind = self.editor_state.read(cx).active_tab_kind;
        let method_dropdown_open = self.editor_state.read(cx).method_dropdown_open;
        let active_req = self.editor_state.read(cx).active_request.clone();

        // Synchronize active request fields into InputState / TextareaState when switched or updated via query params
        if let Some(req) = &active_req {
            let req_switched = self.last_synced_request_id.as_deref() != Some(&req.id);
            let ed_body_ver = self.editor_state.read(cx).body_version;
            let body_ver_changed = self.last_synced_body_version != ed_body_ver;

            if req_switched {
                self.last_synced_request_id = Some(req.id.clone());
                self.last_synced_body_version = ed_body_ver;
            }

            if req_switched || self.url_input.read(cx).value().as_str() != req.url {
                let tokenized_url = {
                    let ws = self.workspace_state.read(cx);
                    crate::components::env_autocomplete::build_tokenized_content(&req.url, ws)
                };
                self.url_input.update(cx, |inp, cx| {
                    inp.set_value(tokenized_url, window, cx);
                });
            }

            if req_switched || body_ver_changed {
                self.last_synced_body_version = ed_body_ver;
                match &req.body {
                    RequestBody::Json(json_str) => {
                        self.json_editor.update(cx, |txt, cx| {
                            txt.set_value(json_str.clone(), window, cx);
                        });
                    }
                    RequestBody::Raw { content, .. } => {
                        self.raw_textarea.update(cx, |txt, cx| {
                            txt.set_value(content.clone(), window, cx);
                        });
                    }
                    _ => {}
                }
            }

            if req_switched {
                match &req.auth {
                    Auth::Bearer { token } => {
                        self.bearer_token_input.update(cx, |inp, cx| {
                            inp.set_value(token.clone(), window, cx);
                        });
                    }
                    Auth::Basic { username, password } => {
                        self.basic_user_input.update(cx, |inp, cx| {
                            inp.set_value(username.clone(), window, cx);
                        });
                        self.basic_pass_input.update(cx, |inp, cx| {
                            inp.set_value(password.clone(), window, cx);
                        });
                    }
                    Auth::ApiKey { key, value, .. } => {
                        self.apikey_key_input.update(cx, |inp, cx| {
                            inp.set_value(key.clone(), window, cx);
                        });
                        self.apikey_val_input.update(cx, |inp, cx| {
                            inp.set_value(value.clone(), window, cx);
                        });
                    }
                    Auth::None => {}
                }

                let pre_script = req.script.as_ref().and_then(|s| s.pre_request.clone()).unwrap_or_default();
                if self.pre_script_editor.read(cx).value().as_str() != pre_script {
                    self.pre_script_editor.update(cx, |txt, cx| {
                        txt.set_value(pre_script, window, cx);
                    });
                }
                let test_script = req.script.as_ref().and_then(|s| s.post_response.clone()).unwrap_or_default();
                if self.test_script_editor.read(cx).value().as_str() != test_script {
                    self.test_script_editor.update(cx, |txt, cx| {
                        txt.set_value(test_script, window, cx);
                    });
                }
            }
        }

        let ws_entity = self.workspace_state.clone();
        let ed_entity = self.editor_state.clone();
        let resp_entity = self.response_state.clone();
        let toast_entity = self.toast_state.clone();

        let app_view_entity = cx.entity().clone();
        let req_id_str = active_req.as_ref().map(|r| r.id.clone()).unwrap_or_else(|| "scratch".to_string());
        let ws_read = ws_entity.read(cx);
        let active_conflict = ws_read.conflicts.get(&req_id_str).cloned();
        let active_parse_err = ws_read.parse_errors.get(&req_id_str).cloned();
        let corrupt_sec_err = ws_read.corrupt_secrets_warning.clone();

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
            .on_action({
                let ws_c = ws_entity.clone();
                move |_: &ToggleSidebar, _, cx| {
                    ws_c.update(cx, |state, cx| {
                        state.toggle_sidebar(cx);
                    });
                }
            })
            .on_action({
                let ws_c = ws_entity.clone();
                move |_: &ToggleSettings, _, cx| {
                    ws_c.update(cx, |state, cx| {
                        state.toggle_settings_modal(cx);
                    });
                }
            })
            .on_action({
                let ws_c = ws_entity.clone();
                let ed_c = ed_entity.clone();
                move |_: &NavigateBack, _, cx| {
                    if let Some(req) = ws_c.update(cx, |ws, cx| ws.navigate_back(cx)) {
                        ed_c.update(cx, |ed, cx| {
                            ed.set_active_request(req, false, cx);
                        });
                    }
                }
            })
            .on_action({
                let ws_c = ws_entity.clone();
                let ed_c = ed_entity.clone();
                move |_: &NavigateForward, _, cx| {
                    if let Some(req) = ws_c.update(cx, |ws, cx| ws.navigate_forward(cx)) {
                        ed_c.update(cx, |ed, cx| {
                            ed.set_active_request(req, false, cx);
                        });
                    }
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
                    .child({
                        let main_content = div().size_full().child({
                            // Editor & Response split: Horizontal or Vertical
                            let editor_pane = div()
                                            .size_full()
                                            .bg(t.bg_editor)
                                            .relative()
                                            .flex()
                                            .flex_col()
                                            // Conflict banner
                                            .when_some(active_conflict, |this, msg| {
                                                let ws_rel = ws_entity.clone();
                                                let ws_keep = ws_entity.clone();
                                                let cid = req_id_str.clone();
                                                let cid_k = req_id_str.clone();
                                                this.child(
                                                    div()
                                                        .px(px(12.))
                                                        .py(px(6.))
                                                        .bg(gpui::Hsla { a: 0.15, ..t.status_4xx })
                                                        .border_b_1()
                                                        .border_color(t.status_4xx)
                                                        .flex()
                                                        .items_center()
                                                        .justify_between()
                                                        .text_xs()
                                                        .child(
                                                            div()
                                                                .flex()
                                                                .items_center()
                                                                .gap(px(8.))
                                                                .child(
                                                                    div()
                                                                        .font_weight(gpui::FontWeight::BOLD)
                                                                        .text_color(t.status_4xx)
                                                                        .child("⚠️ Conflict:")
                                                                )
                                                                .child(
                                                                    div()
                                                                        .text_color(t.text_primary)
                                                                        .child(msg)
                                                                )
                                                        )
                                                        .child(
                                                            div()
                                                                .flex()
                                                                .items_center()
                                                                .gap(px(6.))
                                                                .child(
                                                                    div()
                                                                        .id("btn-conflict-reload")
                                                                        .cursor_pointer()
                                                                        .px(px(8.))
                                                                        .py(px(2.))
                                                                        .rounded(px(4.))
                                                                        .bg(t.primary)
                                                                        .text_color(t.primary_foreground)
                                                                        .on_click(move |_, _, cx| {
                                                                            ws_rel.update(cx, |ws, cx| {
                                                                                ws.resolve_conflict_reload(&cid, cx);
                                                                            });
                                                                        })
                                                                        .child("Reload")
                                                                )
                                                                .child(
                                                                    div()
                                                                        .id("btn-conflict-keep")
                                                                        .cursor_pointer()
                                                                        .px(px(8.))
                                                                        .py(px(2.))
                                                                        .rounded(px(4.))
                                                                        .bg(t.bg_card)
                                                                        .border_1()
                                                                        .border_color(t.border)
                                                                        .text_color(t.text_primary)
                                                                        .on_click(move |_, _, cx| {
                                                                            ws_keep.update(cx, |ws, cx| {
                                                                                ws.resolve_conflict_keep(&cid_k, cx);
                                                                            });
                                                                        })
                                                                        .child("Keep mine")
                                                                )
                                                        )
                                                )
                                            })
                                            // Parse error banner
                                            .when_some(active_parse_err, |this, err| {
                                                this.child(
                                                    div()
                                                        .px(px(12.))
                                                        .py(px(6.))
                                                        .bg(gpui::Hsla { a: 0.12, ..t.status_5xx })
                                                        .border_b_1()
                                                        .border_color(t.status_5xx)
                                                        .flex()
                                                        .items_center()
                                                        .gap(px(8.))
                                                        .text_xs()
                                                        .text_color(t.status_5xx)
                                                        .child("⚠️ Parse error on disk (showing last good in-memory version):")
                                                        .child(
                                                            div()
                                                                .font_family("monospace")
                                                                .text_color(t.text_primary)
                                                                .child(err)
                                                        )
                                                )
                                            })
                                            // Corrupt secrets banner
                                            .when_some(corrupt_sec_err, |this, msg| {
                                                this.child(
                                                    div()
                                                        .px(px(12.))
                                                        .py(px(6.))
                                                        .bg(gpui::Hsla { a: 0.15, ..t.status_4xx })
                                                        .border_b_1()
                                                        .border_color(t.status_4xx)
                                                        .flex()
                                                        .items_center()
                                                        .gap(px(8.))
                                                        .text_xs()
                                                        .text_color(t.status_4xx)
                                                        .child(format!("⚠️ Sensitive Data Warning: {}", msg))
                                                )
                                            })
                                            // Top Request Bar: Method + URL + Send
                                            .child({
                                                let view_handle = app_view_entity.clone();
                                                RequestBar::new(
                                                    ws_entity.clone(),
                                                    ed_entity.clone(),
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
                                                        let has_pre = active_req
                                                            .as_ref()
                                                            .and_then(|r| r.script.as_ref())
                                                            .and_then(|s| s.pre_request.as_ref())
                                                            .map(|s| !s.trim().is_empty())
                                                            .unwrap_or(false);
                                                        TabItem::new("tab-pre-script", "Pre-request")
                                                            .active(active_tab_kind == RequestTabKind::PreRequestScript)
                                                            .when(has_pre, |tab| tab.badge("●"))
                                                            .on_click(move |_, _, cx| {
                                                                ed_c.update(cx, |state, cx| {
                                                                    state.set_active_tab_kind(RequestTabKind::PreRequestScript, cx);
                                                                });
                                                            })
                                                    })
                                                    .child({
                                                        let ed_c = ed_entity.clone();
                                                        let has_test = active_req
                                                            .as_ref()
                                                            .and_then(|r| r.script.as_ref())
                                                            .and_then(|s| s.post_response.as_ref())
                                                            .map(|s| !s.trim().is_empty())
                                                            .unwrap_or(false);
                                                        TabItem::new("tab-tests", "Tests")
                                                            .active(active_tab_kind == RequestTabKind::Tests)
                                                            .when(has_test, |tab| tab.badge("●"))
                                                            .on_click(move |_, _, cx| {
                                                                ed_c.update(cx, |state, cx| {
                                                                    state.set_active_tab_kind(RequestTabKind::Tests, cx);
                                                                });
                                                            })
                                                    })
                                                    .child(div().flex_1())
                                                    .child({
                                                        let ws_s = ws_entity.clone();
                                                        div()
                                                            .id("btn-open-request-settings")
                                                            .cursor_pointer()
                                                            .px(px(6.))
                                                            .py(px(3.))
                                                            .rounded(px(4.))
                                                            .text_color(t.text_muted)
                                                            .hover(|s| s.bg(t.bg_hover).text_color(t.text_primary))
                                                            .flex()
                                                            .items_center()
                                                            .gap(px(4.))
                                                            .text_xs()
                                                            .on_click(move |_, _, cx| {
                                                                ws_s.update(cx, |state, cx| {
                                                                    state.toggle_settings_modal(cx);
                                                                });
                                                            })
                                                            .child("⚙")
                                                            .child("Settings")
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
                                                                        self.json_editor.clone(),
                                                                        self.raw_textarea.clone(),
                                                                    )
                                                                    .workspace_state(ws_entity.clone())
                                                                    .toast_state(self.toast_state.clone())
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
                                                                    .workspace_state(ws_entity.clone())
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
                                                                    .workspace_state(ws_entity.clone())
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
                                                            RequestTabKind::PreRequestScript => {
                                                                let pre_txt = self.pre_script_editor.clone();
                                                                let script_err = ed_entity.read(cx).script_error.clone();
                                                                div().size_full().child(
                                                                    ScriptEditor::new(
                                                                        self.pre_script_editor.clone(),
                                                                        ScriptEditorKind::PreRequest,
                                                                        script_err,
                                                                        move |snippet, window, cx| {
                                                                            pre_txt.update(cx, |t, cx| {
                                                                                let cur = t.value().to_string();
                                                                                let new_val = if cur.is_empty() {
                                                                                    snippet.to_string()
                                                                                } else if cur.ends_with('\n') {
                                                                                    format!("{}{}", cur, snippet)
                                                                                } else {
                                                                                    format!("{}\n{}", cur, snippet)
                                                                                };
                                                                                t.set_value(new_val, window, cx);
                                                                            });
                                                                        },
                                                                    )
                                                                )
                                                            }
                                                            RequestTabKind::Tests => {
                                                                let test_txt = self.test_script_editor.clone();
                                                                let script_err = ed_entity.read(cx).script_error.clone();
                                                                div().size_full().child(
                                                                    ScriptEditor::new(
                                                                        self.test_script_editor.clone(),
                                                                        ScriptEditorKind::Tests,
                                                                        script_err,
                                                                        move |snippet, window, cx| {
                                                                            test_txt.update(cx, |t, cx| {
                                                                                let cur = t.value().to_string();
                                                                                let new_val = if cur.is_empty() {
                                                                                    snippet.to_string()
                                                                                } else if cur.ends_with('\n') {
                                                                                    format!("{}{}", cur, snippet)
                                                                                } else {
                                                                                    format!("{}\n{}", cur, snippet)
                                                                                };
                                                                                t.set_value(new_val, window, cx);
                                                                            });
                                                                        },
                                                                    )
                                                                )
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

                                        let response_pane = ResponseViewer::new(ws_entity.clone(), ed_entity.clone(), resp_entity.clone());

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
                                    });

                        if sidebar_collapsed {
                            main_content.into_any_element()
                        } else {
                            h_resizable("workspace-sidebar-split")
                                .child(
                                    resizable_panel()
                                        .size(sidebar_width)
                                        .size_range(px(200.)..px(480.))
                                        .child(SidebarTree::new(ws_entity.clone(), ed_entity.clone())),
                                )
                                .child(resizable_panel().child(main_content))
                                .on_resize({
                                    let ws_entity = ws_entity.clone();
                                    move |state, _window, cx| {
                                        let new_size = state.read(cx).sizes().first().copied();
                                        if let Some(size) = new_size {
                                            ws_entity.update(cx, |ws, cx| {
                                                ws.set_sidebar_width(size, cx);
                                            });
                                        }
                                    }
                                })
                                .into_any_element()
                        }
                    }),
            )
            // Modals & Overlays
            .when(show_env_modal, |this| {
                this.child(EnvironmentModal::new(ws_entity.clone()))
            })
            .when(show_cookies_modal, |this| {
                this.child(CookiesModal::new(ws_entity.clone()))
            })
            .when(show_settings_modal, |this| {
                this.child(SettingsModal::new(ws_entity.clone(), ed_entity.clone()))
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
            KeyBinding::new("ctrl-b", ToggleSidebar, None),
            KeyBinding::new("cmd-b", ToggleSidebar, None),
            KeyBinding::new("ctrl-,", ToggleSettings, None),
            KeyBinding::new("cmd-,", ToggleSettings, None),
            KeyBinding::new("ctrl-[", NavigateBack, None),
            KeyBinding::new("cmd-[", NavigateBack, None),
            KeyBinding::new("ctrl-]", NavigateForward, None),
            KeyBinding::new("cmd-]", NavigateForward, None),
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

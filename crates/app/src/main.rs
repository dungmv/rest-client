#![allow(dead_code, clippy::type_complexity)]
#![recursion_limit = "512"]
#![windows_subsystem = "windows"]

mod components;
mod platform;
pub mod settings;
mod state;
mod theme;

use components::*;
use gpui::prelude::FluentBuilder as _;
use gpui::*;
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::menu::{DropdownMenu as _, PopupMenuItem};
use gpui_component::input::{
    EditorState as CodeEditorState, Input, InputEvent, InputState, TextareaState,
};
use gpui_component::resizable::{h_resizable, resizable_panel, v_resizable};
use gpui_component::scroll::ScrollableElement;
use gpui_component::Icon;
use gpui_kit::assets::{AllAssets, IconName};
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

actions!(restclient, [SendRequest, ToggleCommandPalette, ToggleSidebar, ToggleSettings, ToggleGitModal, OpenImportModal, NavigateBack, NavigateForward, DismissModal]);

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
    xml_editor: Entity<CodeEditorState>,
    raw_textarea: Entity<TextareaState>,
    response_editor: Entity<CodeEditorState>,
    request_url_textarea: Entity<TextareaState>,
    request_body_editor: Entity<CodeEditorState>,
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
    last_synced_request_version: usize,
    last_synced_body_version: usize,
    last_synced_response_id: Option<String>,
    last_synced_response_view_mode: Option<ResponseViewMode>,
    last_synced_example_name: Option<String>,
    _subscriptions: Vec<Subscription>,
}

fn save_active_request_without_redraw(
    editor: &Entity<EditorState>,
    workspace: &Entity<WorkspaceState>,
    cx: &mut App,
) {
    let is_scratch_or_none = editor.read(cx).active_request.as_ref().map(|r| r.workspace_id == "scratch" || r.id.is_empty()).unwrap_or(true);
    if !is_scratch_or_none {
        if let Some(req_id) = editor.read(cx).active_request.as_ref().map(|r| r.id.clone()) {
            let ed = editor.clone();
            workspace.update(cx, |ws, cx| ws.schedule_save_active_request(req_id, ed, cx));
        }
    }
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

        let highlighter_factory: std::rc::Rc<dyn Fn(&str) -> Option<Box<dyn gpui_component::input::InputHighlighter>>> =
            std::rc::Rc::new(|lang: &str| match lang {
                "json" => Some(Box::new(crate::components::body_editor::JsonHighlighter::new())),
                "xml" => Some(Box::new(crate::components::body_editor::XmlHighlighter::new())),
                "javascript" | "js" => Some(Box::new(JavaScriptHighlighter::new())),
                _ => None,
            });

        let json_editor = cx.new(|cx| {
            let mut state = CodeEditorState::new(window, cx);
            state.set_highlighter_factory(highlighter_factory.clone(), cx);
            state.lsp_mut().completion_provider = Some(std::rc::Rc::new(
                crate::components::env_autocomplete::EnvCompletionProvider::new(workspace_state.clone()),
            ));
            state.language("json").line_number(true).folding(true)
        });
        let xml_editor = cx.new(|cx| {
            let mut state = CodeEditorState::new(window, cx);
            state.set_highlighter_factory(highlighter_factory.clone(), cx);
            state.lsp_mut().completion_provider = Some(std::rc::Rc::new(
                crate::components::env_autocomplete::EnvCompletionProvider::new(workspace_state.clone()),
            ));
            state.language("xml").line_number(true).folding(true)
        });
        let raw_textarea = cx.new(|cx| TextareaState::new(window, cx));
        let response_editor = cx.new(|cx| {
            let mut state = CodeEditorState::new(window, cx);
            state.set_highlighter_factory(highlighter_factory.clone(), cx);
            state.language("json").line_number(true).folding(true).soft_wrap(false)
        });
        let request_url_textarea = cx.new(|cx| TextareaState::new(window, cx).auto_grow(1, 8));
        let request_body_editor = cx.new(|cx| {
            let mut state = CodeEditorState::new(window, cx);
            state.set_highlighter_factory(highlighter_factory.clone(), cx);
            state.language("json").line_number(true).folding(false)
        });
        let pre_script_editor = cx.new(|cx| {
            let mut state = CodeEditorState::new(window, cx);
            state.set_highlighter_factory(highlighter_factory.clone(), cx);
            state.lsp_mut().completion_provider = Some(std::rc::Rc::new(
                ScriptCompletionProvider::with_workspace(ScriptEditorKind::PreRequest, workspace_state.clone()),
            ));
            state.language("javascript").line_number(true).folding(true)
        });
        let test_script_editor = cx.new(|cx| {
            let mut state = CodeEditorState::new(window, cx);
            state.set_highlighter_factory(highlighter_factory.clone(), cx);
            state.lsp_mut().completion_provider = Some(std::rc::Rc::new(
                ScriptCompletionProvider::with_workspace(ScriptEditorKind::Tests, workspace_state.clone()),
            ));
            state.language("javascript").line_number(true).folding(true)
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
        let (initial_req, ws_store) = {
            let ws_read = workspace_state.read(cx);
            let req = ws_read
                .local_state
                .active_request
                .as_ref()
                .and_then(|act_id| ws_read.requests.iter().find(|r| &r.id == act_id).cloned())
                .or_else(|| ws_read.requests.first().cloned());
            (req, ws_read.store.clone())
        };
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
            } else if let RequestBody::Raw { content, content_type } = &req.body {
                if content_type == "application/xml" || content_type == "text/xml" {
                    xml_editor.update(cx, |txt, cx| {
                        txt.set_value(content.clone(), window, cx);
                    });
                }
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

            let ws_for_auth = workspace_state.clone();
            match &req.auth {
                Auth::Bearer { token } => {
                    let ws_m = ws_for_auth.clone();
                    bearer_token_input.update(cx, |inp, cx| {
                        let ws = ws_m.read(cx);
                        inp.set_value(crate::components::env_autocomplete::build_tokenized_content(token, ws), window, cx);
                    });
                }
                Auth::Basic { username, password } => {
                    let ws_u = ws_for_auth.clone();
                    let ws_p = ws_for_auth.clone();
                    basic_user_input.update(cx, |inp, cx| {
                        let ws = ws_u.read(cx);
                        inp.set_value(crate::components::env_autocomplete::build_tokenized_content(username, ws), window, cx);
                    });
                    basic_pass_input.update(cx, |inp, cx| {
                        let ws = ws_p.read(cx);
                        inp.set_value(crate::components::env_autocomplete::build_tokenized_content(password, ws), window, cx);
                    });
                }
                Auth::ApiKey { key, value, .. } => {
                    let ws_k = ws_for_auth.clone();
                    let ws_v = ws_for_auth.clone();
                    apikey_key_input.update(cx, |inp, cx| {
                        let ws = ws_k.read(cx);
                        inp.set_value(crate::components::env_autocomplete::build_tokenized_content(key, ws), window, cx);
                    });
                    apikey_val_input.update(cx, |inp, cx| {
                        let ws = ws_v.read(cx);
                        inp.set_value(crate::components::env_autocomplete::build_tokenized_content(value, ws), window, cx);
                    });
                }
                Auth::Inherit | Auth::None => {}
            }

            let req_id = req.id.clone();
            response_state.update(cx, |resp, cx| {
                resp.load_for_request(&req_id, ws_store.as_ref(), cx);
            });
        }

        // Set up subscriptions:
        let mut _subscriptions = Vec::new();

        // 1. Subscribe to URL input change
        let ed_url = editor_state.clone();
        let ws_url = workspace_state.clone();
        _subscriptions.push(cx.subscribe(&url_input, move |this, input, event, cx| {
            match event {
                InputEvent::Change => {
                    let val = input.read(cx).value().to_string();
                    if restclient_core::curl::is_curl_command(&val) {
                        let ws_id = ed_url
                            .read(cx)
                            .active_request
                            .as_ref()
                            .map(|r| r.workspace_id.clone())
                            .unwrap_or_else(|| "scratch".to_string());
                        match restclient_core::curl::curl_to_request(&ws_id, &val) {
                            Ok(parsed) => {
                                let method_name = parsed.method.as_str().to_string();
                                let target_url = parsed.url.clone();
                                ed_url.update(cx, |ed, cx| {
                                    ed.apply_imported_curl(parsed, cx);
                                });
                                save_active_request_without_redraw(&ed_url, &ws_url, cx);
                                this.toast_state.update(cx, |t, cx| {
                                    t.push_success(
                                        "cURL Imported",
                                        format!("Parsed {} request to {}", method_name, target_url),
                                        cx,
                                    );
                                });
                                return;
                            }
                            Err(e) => {
                                if val.lines().count() > 1 || val.contains(" -") || val.contains(" --") {
                                    this.toast_state.update(cx, |t, cx| {
                                        t.push_error(
                                            "cURL Parse Error",
                                            format!("Failed to parse cURL: {}", e),
                                            cx,
                                        );
                                    });
                                }
                            }
                        }
                    }
                    ed_url.update(cx, |ed, cx| {
                        ed.update_url_from_input(val, cx);
                    });
                    save_active_request_without_redraw(&ed_url, &ws_url, cx);
                }
                InputEvent::PressEnter { .. } => {
                    this.send_current_request(cx);
                }
                _ => {}
            }
        }));

        // 2. Subscribe to JSON editor change
        let ed_json = editor_state.clone();
        let ws_json = workspace_state.clone();
        _subscriptions.push(cx.subscribe(&json_editor, move |_, txt, event, cx| {
            if let InputEvent::Change = event {
                let val = txt.read(cx).value().to_string();
                ed_json.update(cx, |ed, _cx| {
                    ed.apply_set_body_json(val);
                });
                save_active_request_without_redraw(&ed_json, &ws_json, cx);
            }
        }));

        // 2b. Subscribe to XML editor change
        let ed_xml = editor_state.clone();
        let ws_xml = workspace_state.clone();
        _subscriptions.push(cx.subscribe(&xml_editor, move |_, txt, event, cx| {
            if let InputEvent::Change = event {
                let val = txt.read(cx).value().to_string();
                ed_xml.update(cx, |ed, _cx| {
                    ed.apply_set_raw_body(val, "application/xml".to_string());
                });
                save_active_request_without_redraw(&ed_xml, &ws_xml, cx);
            }
        }));

        // 3. Subscribe to Raw textarea change
        let ed_raw = editor_state.clone();
        let ws_raw = workspace_state.clone();
        _subscriptions.push(cx.subscribe(&raw_textarea, move |_, txt, event, cx| {
            if let InputEvent::Change = event {
                let val = txt.read(cx).value().to_string();
                ed_raw.update(cx, |ed, _cx| {
                    let content_type = match ed.active_request.as_ref().map(|r| &r.body) {
                        Some(RequestBody::Raw { content_type, .. }) => content_type.clone(),
                        _ => "text/plain".to_string(),
                    };
                    ed.apply_set_raw_body(val, content_type);
                });
                save_active_request_without_redraw(&ed_raw, &ws_raw, cx);
            }
        }));

        // 3b. Subscribe to Auth inputs
        let ed_bearer = editor_state.clone();
        let ws_bearer = workspace_state.clone();
        _subscriptions.push(cx.subscribe(&bearer_token_input, move |_, inp, event, cx| {
            if let InputEvent::Change = event {
                let val = inp.read(cx).value().to_string();
                ed_bearer.update(cx, |ed, cx| {
                    ed.update_bearer_token_from_input(val, cx);
                });
                save_active_request_without_redraw(&ed_bearer, &ws_bearer, cx);
            }
        }));

        let ed_basic_u = editor_state.clone();
        let ws_basic_u = workspace_state.clone();
        let pass_ref = basic_pass_input.clone();
        _subscriptions.push(cx.subscribe(&basic_user_input, move |_, inp, event, cx| {
            if let InputEvent::Change = event {
                let u = inp.read(cx).value().to_string();
                let p = pass_ref.read(cx).value().to_string();
                ed_basic_u.update(cx, |ed, cx| {
                    ed.update_basic_auth_from_input(u, p, cx);
                });
                save_active_request_without_redraw(&ed_basic_u, &ws_basic_u, cx);
            }
        }));

        let ed_basic_p = editor_state.clone();
        let ws_basic_p = workspace_state.clone();
        let user_ref = basic_user_input.clone();
        _subscriptions.push(cx.subscribe(&basic_pass_input, move |_, inp, event, cx| {
            if let InputEvent::Change = event {
                let u = user_ref.read(cx).value().to_string();
                let p = inp.read(cx).value().to_string();
                ed_basic_p.update(cx, |ed, cx| {
                    ed.update_basic_auth_from_input(u, p, cx);
                });
                save_active_request_without_redraw(&ed_basic_p, &ws_basic_p, cx);
            }
        }));

        let ed_ak_k = editor_state.clone();
        let ws_ak_k = workspace_state.clone();
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
                    ed.update_api_key_from_input(k, v, in_header, cx);
                });
                save_active_request_without_redraw(&ed_ak_k, &ws_ak_k, cx);
            }
        }));

        let ed_ak_v = editor_state.clone();
        let ws_ak_v = workspace_state.clone();
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
                    ed.update_api_key_from_input(k, v, in_header, cx);
                });
                save_active_request_without_redraw(&ed_ak_v, &ws_ak_v, cx);
            }
        }));

        // 3c. Subscribe to Script editors
        let ed_pre = editor_state.clone();
        let ws_pre = workspace_state.clone();
        _subscriptions.push(cx.subscribe(&pre_script_editor, move |_, txt, event, cx| {
            if let InputEvent::Change = event {
                let val = txt.read(cx).value().to_string();
                ed_pre.update(cx, |ed, _cx| {
                    ed.apply_set_pre_script(val);
                });
                save_active_request_without_redraw(&ed_pre, &ws_pre, cx);
            }
        }));

        let ed_test = editor_state.clone();
        let ws_test = workspace_state.clone();
        _subscriptions.push(cx.subscribe(&test_script_editor, move |_, txt, event, cx| {
            if let InputEvent::Change = event {
                let val = txt.read(cx).value().to_string();
                ed_test.update(cx, |ed, _cx| {
                    ed.apply_set_test_script(val);
                });
                save_active_request_without_redraw(&ed_test, &ws_test, cx);
            }
        }));

        // 4. Observe editor_state: reload response history when active request switches & auto-save
        let ws_clone = workspace_state.clone();
        let resp_clone = response_state.clone();
        let mut last_req_id = initial_req.as_ref().map(|r| r.id.clone());
        _subscriptions.push(cx.observe(&editor_state, move |_, ed, cx| {
            let req_switch_info = {
                let ed_ref = ed.read(cx);
                ed_ref.active_request.as_ref().map(|r| (r.id.clone(), r.workspace_id == "scratch" || r.id.is_empty()))
            };
            if let Some((req_id, is_scratch)) = req_switch_info {
                if last_req_id.as_ref() != Some(&req_id) {
                    last_req_id = Some(req_id.clone());
                    let ws_store = ws_clone.read(cx).store.clone();
                    resp_clone.update(cx, |r_state, cx| {
                        r_state.load_for_request(&req_id, ws_store.as_ref(), cx);
                    });
                }

                // Debounced auto-save active request to workspace storage
                if !is_scratch {
                    let ed_ent = ed.clone();
                    ws_clone.update(cx, |ws, cx| {
                        ws.schedule_save_active_request(req_id, ed_ent, cx);
                    });
                }
            } else if last_req_id.is_some() {
                last_req_id = None;
                resp_clone.update(cx, |r_state, cx| {
                    r_state.set_response(None, cx);
                });
            }
        }));

        // 5. Observe workspace_state: synchronize workspace changes, renamed request to editor_state.active_request & response_state
        let ed_sync = editor_state.clone();
        let resp_sync = response_state.clone();
        let mut last_workspace_path = workspace_state.read(cx).workspace_path.clone();
        _subscriptions.push(cx.observe(&workspace_state, move |_, ws, cx| {
            let current_ws_path = ws.read(cx).workspace_path.clone();
            if current_ws_path != last_workspace_path {
                last_workspace_path = current_ws_path;
                let (next_req, ws_store) = {
                    let ws_ref = ws.read(cx);
                    let req = ws_ref
                        .local_state
                        .active_request
                        .as_ref()
                        .and_then(|act_id| ws_ref.requests.iter().find(|r| &r.id == act_id).cloned())
                        .or_else(|| ws_ref.requests.first().cloned());
                    (req, ws_ref.store.clone())
                };
                if let Some(req) = next_req {
                    ed_sync.update(cx, |ed, cx| {
                        ed.set_active_request(req.clone(), true, cx);
                    });
                    let req_id = req.id.clone();
                    resp_sync.update(cx, |resp, cx| {
                        resp.load_for_request(&req_id, ws_store.as_ref(), cx);
                    });
                } else {
                    ed_sync.update(cx, |ed, cx| {
                        ed.active_request = None;
                        cx.notify();
                    });
                    resp_sync.update(cx, |resp, cx| {
                        resp.set_response(None, cx);
                    });
                }
                return;
            }

            let (last_renamed, matching_req, saved_name_for_active, saved_examples_for_active) = {
                let ws_ref = ws.read(cx);
                let last_renamed = ws_ref.last_renamed_request.clone();
                let matching_req = if let Some((_, new_id)) = &last_renamed {
                    ws_ref.requests.iter().find(|r| &r.id == new_id).cloned()
                } else {
                    None
                };
                let active_id = ed_sync.read(cx).active_request.as_ref().map(|r| r.id.clone());
                let (saved_name_for_active, saved_examples_for_active) = if let Some(ref a_id) = active_id {
                    let req = ws_ref.requests.iter().find(|r| &r.id == a_id);
                    (req.map(|r| r.name.clone()), req.map(|r| r.examples.clone()))
                } else {
                    (None, None)
                };
                (last_renamed, matching_req, saved_name_for_active, saved_examples_for_active)
            };

            if let Some((old_id, new_id)) = last_renamed {
                resp_sync.update(cx, |resp, _cx| {
                    resp.rename_request(&old_id, &new_id);
                });
                ed_sync.update(cx, |ed, cx| {
                    if let Some(curr_act) = &ed.active_request {
                        if curr_act.id == old_id {
                            if let Some(matching) = matching_req {
                                ed.set_active_request(matching, false, cx);
                            }
                        }
                    }
                });
            }

            if let Some(saved) = saved_name_for_active {
                ed_sync.update(cx, |ed, cx| {
                    if let Some(curr) = &ed.active_request {
                        if curr.name != saved {
                            ed.set_name(saved, cx);
                        }
                    }
                });
            }

            if let Some(examples) = saved_examples_for_active {
                ed_sync.update(cx, |ed, cx| {
                    if let Some(curr) = &mut ed.active_request {
                        if curr.examples != examples {
                            curr.examples = examples;
                            cx.notify();
                        }
                    }
                });
            }

            if ws.read(cx).last_renamed_example.is_some() {
                let last_renamed_ex = ws.update(cx, |ws, _cx| ws.last_renamed_example.take());
                if let Some((_req_id, old_ex, new_ex)) = last_renamed_ex {
                    resp_sync.update(cx, |resp, cx| {
                        resp.rename_example(&old_ex, &new_ex, cx);
                    });
                }
            }
        }));

        // 6. Observe response_state: re-render AppView on response/example changes
        _subscriptions.push(cx.observe(&response_state, |_, _, cx| {
            cx.notify();
        }));

        // 5. Observe OS appearance changes for System theme preference
        let ws_theme = workspace_state.clone();
        _subscriptions.push(window.observe_window_appearance(move |window, cx| {
            let app_appearance = window.appearance();
            ws_theme.update(cx, |ws, cx| {
                if ws.theme_preference == crate::theme::ThemePreference::System {
                    let mode = crate::theme::resolve_theme_mode(crate::theme::ThemePreference::System, app_appearance);
                    if crate::theme::current_theme_mode() != mode {
                        crate::theme::set_theme(crate::theme::ThemePreference::System, mode);
                        let gpui_mode = match mode {
                            crate::theme::ThemeMode::Dark => gpui_component::ThemeMode::Dark,
                            crate::theme::ThemeMode::Light => gpui_component::ThemeMode::Light,
                        };
                        gpui_component::Theme::change(gpui_mode, None, cx);
                        cx.refresh_windows();
                        cx.notify();
                    }
                }
            });
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
            xml_editor,
            raw_textarea,
            response_editor,
            request_url_textarea,
            request_body_editor,
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
            last_synced_request_version: 0,
            last_synced_body_version,
            last_synced_response_id: None,
            last_synced_response_view_mode: None,
            last_synced_example_name: None,
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

        // 0. Resolve Hierarchical Headers & Auth (Workspace -> Folder hierarchy -> Request)
        {
            let ws = self.workspace_state.read(cx);
            let (resolved_headers, resolved_auth) = ws.resolve_request_hierarchy(&req_item);
            req_item.headers = resolved_headers;
            req_item.auth = resolved_auth;
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

        match &mut req_item.auth {
            Auth::Bearer { token } => *token = resolve_fake_vars(token),
            Auth::Basic { username, password } => {
                *username = resolve_fake_vars(username);
                *password = resolve_fake_vars(password);
            }
            Auth::ApiKey { key, value, .. } => {
                *key = resolve_fake_vars(key);
                *value = resolve_fake_vars(value);
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
                        v_item.current_value = val.clone();
                    } else {
                        temp_env.variables.push(EnvironmentVariable {
                            key: k.clone(),
                            initial_value: "".to_string(),
                            current_value: val.clone(),
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
        ws_entity.update(cx, |ws, cx| {
            ws.running_request_id = Some(req_item.id.clone());
            cx.notify();
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
                                                        var.current_value = val.clone();
                                                    } else {
                                                        act_env.variables.push(EnvironmentVariable {
                                                            key: k.clone(),
                                                            initial_value: "".to_string(),
                                                            current_value: val.clone(),
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
                                                        var.current_value = val.clone();
                                                    } else {
                                                        ws.global_environment.variables.push(EnvironmentVariable {
                                                            key: k.clone(),
                                                            initial_value: "".to_string(),
                                                            current_value: val.clone(),
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
                                this.workspace_state.update(cx, |ws, cx| {
                                    ws.running_request_id = None;
                                    if let Some(r) = ws.requests.iter_mut().find(|r| r.id == req_id) {
                                        r.last_status_code = Some(status);
                                    }
                                    let _ = ws.store.save_response_cache(&req_id, &loc_file);
                                    cx.notify();
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
                                this.workspace_state.update(cx, |ws, cx| {
                                    ws.running_request_id = None;
                                    cx.notify();
                                });
                                this.editor_state.update(cx, |ed, cx| {
                                    ed.stop_sending(cx);
                                });
                            }
                            HttpSendEvent::Error(err_msg) => {
                                this.workspace_state.update(cx, |ws, cx| {
                                    ws.running_request_id = None;
                                    cx.notify();
                                });
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

    fn render_auth_input<F>(
        &self,
        input_id: &'static str,
        input_state: &Entity<InputState>,
        window: &mut Window,
        cx: &mut Context<Self>,
        on_update: F,
    ) -> impl IntoElement
    where
        F: 'static + Clone + Fn(String, &mut Window, &mut App),
    {
        let t = theme();
        let current_val = input_state.read(cx).value().to_string();

        let (completion_candidates, unclosed) = if current_val.contains("{{") {
            let ws = self.workspace_state.read(cx);
            let unclosed = detect_unclosed_env_var(&current_val);
            let candidates = unclosed
                .as_ref()
                .map(|(_, _, q)| get_env_completions(ws, q))
                .unwrap_or_default();
            (candidates, unclosed)
        } else {
            (Vec::new(), None)
        };
        let show_autocomplete = !completion_candidates.is_empty();

        let hover_token_state = window.use_keyed_state(
            gpui::SharedString::from(format!("hover-auth-{}", input_id)),
            cx,
            |_window, _cx| None::<(String, gpui::Bounds<gpui::Pixels>)>,
        );
        let hovered_token = hover_token_state.read(cx).clone();

        let ws_for_token = self.workspace_state.clone();
        let ws_for_click = self.workspace_state.clone();
        let ws_for_hover = self.workspace_state.clone();
        let h_token = hover_token_state.clone();

        div()
            .relative()
            .w_full()
            .child(
                div()
                    .h(px(28.))
                    .px(px(6.))
                    .rounded(px(4.))
                    .bg(t.bg_input)
                    .border_1()
                    .border_color(t.border)
                    .hover(|s| s.border_color(t.border_focus))
                    .flex()
                    .items_center()
                    .child(
                        Input::new(input_state)
                            .id(input_id)
                            .appearance(false)
                            .bordered(false)
                            .focus_bordered(false)
                            .cleanable(true)
                            .text_xs()
                            .font_family(".SystemMonoFont")
                            .token(move |token_ctx, _window, cx| {
                                let var_name = token_ctx.token().id().to_string();
                                let ws = ws_for_token.read(cx);
                                let (_val, scope, is_res) = get_variable_details(ws, &var_name);
                                let (_kind, color, bg_tint) = get_variable_kind_and_color(&scope, is_res);

                                div()
                                    .id(gpui::SharedString::from(format!("{}-tok-{}", input_id, var_name)))
                                    .flex()
                                    .items_center()
                                    .px(px(5.))
                                    .py(px(1.))
                                    .rounded(px(3.))
                                    .bg(bg_tint)
                                    .text_xs()
                                    .font_family(".SystemMonoFont")
                                    .text_color(color)
                                    .cursor_pointer()
                                    .child(var_name)
                            })
                            .on_token_hover(move |event, _window, cx| {
                                let is_h = event.is_hovered();
                                let name = event.token().id().to_string();
                                let bounds = event.bounds();
                                h_token.update(cx, |h, cx| {
                                    if is_h { *h = Some((name, bounds)); } else { *h = None; }
                                    cx.notify();
                                });
                            })
                            .on_token_click(move |_, _window, cx| {
                                ws_for_click.update(cx, |st, cx| {
                                    st.toggle_env_modal(cx);
                                });
                            }),
                    ),
            )
            .when_some(hovered_token, |this, (var_name, _bounds)| {
                let ws = ws_for_hover.read(cx);
                let (val, scope, is_res) = get_variable_details(ws, &var_name);
                let (_kind, color, bg_tint) = get_variable_kind_and_color(&scope, is_res);

                this.child(
                    deferred(
                        div()
                            .absolute()
                            .top(px(34.))
                            .left(px(0.))
                            .w(px(320.))
                            .p(px(8.))
                            .rounded(px(6.))
                            .bg(t.bg_card)
                            .border_1()
                            .border_color(color)
                            .shadow_2xl()
                            .occlude()
                            .flex()
                            .flex_col()
                            .gap(px(4.))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .child(
                                        div()
                                            .text_xs()
                                            .font_weight(gpui::FontWeight::BOLD)
                                            .text_color(color)
                                            .child(format!("{{{{{}}}}}", var_name)),
                                    )
                                    .child(
                                        div()
                                            .px(px(5.))
                                            .py(px(1.))
                                            .rounded(px(3.))
                                            .bg(bg_tint)
                                            .border_1()
                                            .border_color(color)
                                            .text_xs()
                                            .font_weight(gpui::FontWeight::MEDIUM)
                                            .text_color(color)
                                            .child(scope),
                                    ),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .font_family(".SystemMonoFont")
                                    .text_color(if is_res { t.text_primary } else { color })
                                    .child(if is_res {
                                        format!("= \"{}\"", val)
                                    } else {
                                        "Variable not found in active or global environments".to_string()
                                    }),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(t.text_muted)
                                    .child("Click variable to manage environments"),
                            ),
                    )
                    .with_priority(90),
                )
            })
            .when(show_autocomplete, |this| {
                let (start_idx, end_idx) = unclosed.as_ref().map(|(pos, end, _)| (*pos, *end)).unwrap_or((0, 0));
                let current_text = current_val.clone();
                let inp_target = input_state.clone();
                let ws_target = self.workspace_state.clone();
                let on_up = on_update.clone();

                this.child(
                    deferred(
                        div()
                            .absolute()
                            .top(px(34.))
                            .left(px(0.))
                            .w(px(340.))
                            .max_h(px(200.))
                            .rounded(px(6.))
                            .bg(t.bg_card)
                            .border_1()
                            .border_color(t.primary)
                            .shadow_2xl()
                            .occlude()
                            .flex()
                            .flex_col()
                            .overflow_hidden()
                            .child(
                                div()
                                    .px(px(10.))
                                    .py(px(5.))
                                    .border_b_1()
                                    .border_color(t.border)
                                    .bg(t.bg_panel)
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .child(
                                        div()
                                            .text_xs()
                                            .font_weight(gpui::FontWeight::BOLD)
                                            .text_color(t.text_secondary)
                                            .child("ENVIRONMENT VARIABLES"),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(t.text_muted)
                                            .child("Click to insert"),
                                    ),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .p(px(4.))
                                    .overflow_y_scrollbar()
                                    .bg(t.bg_card)
                                    .flex()
                                    .flex_col()
                                    .gap(px(2.))
                                    .children(completion_candidates.into_iter().map(|cand| {
                                        let cand_name = cand.name.clone();
                                        let cand_val = cand.value.clone();
                                        let cand_scope = cand.scope.clone();
                                        let cur_text = current_text.clone();
                                        let inp_c = inp_target.clone();
                                        let ws_c = ws_target.clone();
                                        let on_up_c = on_up.clone();
                                        let (_kind, cand_color, cand_bg_tint) = get_variable_kind_and_color(&cand_scope, true);

                                        div()
                                            .id(gpui::SharedString::from(format!("{}-cand-{}", input_id, cand.name)))
                                            .cursor_pointer()
                                            .px(px(8.))
                                            .py(px(5.))
                                            .rounded(px(4.))
                                            .hover(|s| s.bg(t.bg_hover))
                                            .flex()
                                            .items_center()
                                            .justify_between()
                                            .on_click(move |_, window, cx| {
                                                let mut new_text = cur_text.clone();
                                                if start_idx <= new_text.len() && end_idx <= new_text.len() && start_idx <= end_idx {
                                                    new_text.replace_range(start_idx..end_idx, &format!("{{{{{}}}}}", cand_name));
                                                    let content = {
                                                        let ws = ws_c.read(cx);
                                                        build_tokenized_content(&new_text, ws)
                                                    };
                                                    inp_c.update(cx, |inp, cx| {
                                                        inp.set_value(content, window, cx);
                                                    });
                                                    on_up_c(new_text, window, cx);
                                                }
                                            })
                                            .child(
                                                div()
                                                    .flex()
                                                    .items_center()
                                                    .gap(px(8.))
                                                    .child(
                                                        div()
                                                            .text_xs()
                                                            .font_weight(gpui::FontWeight::BOLD)
                                                            .font_family(".SystemMonoFont")
                                                            .text_color(cand_color)
                                                            .child(format!("{{{{{}}}}}", cand.name)),
                                                    )
                                                    .child(
                                                        div()
                                                            .text_xs()
                                                            .font_family(".SystemMonoFont")
                                                            .text_color(t.text_muted)
                                                            .child(format!("= \"{}\"", cand_val)),
                                                    ),
                                            )
                                            .child(
                                                div()
                                                    .px(px(6.))
                                                    .py(px(1.))
                                                    .rounded(px(3.))
                                                    .bg(cand_bg_tint)
                                                    .border_1()
                                                    .border_color(cand_color)
                                                    .text_xs()
                                                    .font_weight(gpui::FontWeight::MEDIUM)
                                                    .text_color(cand_color)
                                                    .child(cand_scope),
                                            )
                                    })),
                            ),
                    )
                    .with_priority(100),
                )
            })
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
        let show_scope_settings_modal = self.workspace_state.read(cx).scope_settings_target.is_some();
        let show_git_modal = self.workspace_state.read(cx).show_git_modal;
        let show_import_modal = self.workspace_state.read(cx).show_import_modal;
        let show_rename_workspace_modal = self.workspace_state.read(cx).show_rename_workspace_modal;
        let show_command_palette = self.workspace_state.read(cx).show_command_palette;

        let active_tab_kind = self.editor_state.read(cx).active_tab_kind;
        let method_dropdown_open = self.editor_state.read(cx).method_dropdown_open;
        let (active_req_id, active_req_info) = {
            let ed = self.editor_state.read(cx);
            let id = ed.active_request.as_ref().map(|r| r.id.clone());
            let info = ed.active_request.as_ref().map(|r| (
                r.params.len(),
                r.headers.len(),
                r.script.as_ref().and_then(|s| s.pre_request.as_ref()).map(|s| !s.trim().is_empty()).unwrap_or(false),
                r.script.as_ref().and_then(|s| s.post_response.as_ref()).map(|s| !s.trim().is_empty()).unwrap_or(false),
            ));
            (id, info)
        };

        // Synchronize active request fields into InputState / TextareaState when switched or body version changed
        {
            let (req_switched, req_ver_changed, body_ver_changed, ed_body_ver, ed_req_ver) = {
                let ed = self.editor_state.read(cx);
                let switched = match (&self.last_synced_request_id, &ed.active_request) {
                    (Some(last_id), Some(req)) => last_id != &req.id,
                    (None, Some(_)) => true,
                    (Some(_), None) => true,
                    (None, None) => false,
                };
                let req_changed = self.last_synced_request_version != ed.request_version;
                let body_changed = self.last_synced_body_version != ed.body_version;
                (switched, req_changed, body_changed, ed.body_version, ed.request_version)
            };

            if req_switched || req_ver_changed || body_ver_changed {
                let req_opt = self.editor_state.read(cx).active_request.clone();
                if let Some(req) = req_opt {
                    if req_switched || req_ver_changed {
                        self.last_synced_request_id = Some(req.id.clone());
                        self.last_synced_request_version = ed_req_ver;
                        self.last_synced_body_version = ed_body_ver;

                        let tokenized_url = {
                            let ws = self.workspace_state.read(cx);
                            crate::components::env_autocomplete::build_tokenized_content(&req.url, ws)
                        };
                        self.url_input.update(cx, |inp, cx| {
                            inp.set_value(tokenized_url, window, cx);
                        });

                        let ws_holder = self.workspace_state.clone();
                        match &req.auth {
                            Auth::Bearer { token } => {
                                let ws_m = ws_holder.clone();
                                self.bearer_token_input.update(cx, |inp, cx| {
                                    let ws = ws_m.read(cx);
                                    inp.set_value(crate::components::env_autocomplete::build_tokenized_content(token, ws), window, cx);
                                });
                            }
                            Auth::Basic { username, password } => {
                                let ws_u = ws_holder.clone();
                                let ws_p = ws_holder.clone();
                                self.basic_user_input.update(cx, |inp, cx| {
                                    let ws = ws_u.read(cx);
                                    inp.set_value(crate::components::env_autocomplete::build_tokenized_content(username, ws), window, cx);
                                });
                                self.basic_pass_input.update(cx, |inp, cx| {
                                    let ws = ws_p.read(cx);
                                    inp.set_value(crate::components::env_autocomplete::build_tokenized_content(password, ws), window, cx);
                                });
                            }
                            Auth::ApiKey { key, value, .. } => {
                                let ws_k = ws_holder.clone();
                                let ws_v = ws_holder.clone();
                                self.apikey_key_input.update(cx, |inp, cx| {
                                    let ws = ws_k.read(cx);
                                    inp.set_value(crate::components::env_autocomplete::build_tokenized_content(key, ws), window, cx);
                                });
                                self.apikey_val_input.update(cx, |inp, cx| {
                                    let ws = ws_v.read(cx);
                                    inp.set_value(crate::components::env_autocomplete::build_tokenized_content(value, ws), window, cx);
                                });
                            }
                            Auth::Inherit | Auth::None => {
                                self.bearer_token_input.update(cx, |inp, cx| {
                                    inp.set_value(String::new(), window, cx);
                                });
                                self.basic_user_input.update(cx, |inp, cx| {
                                    inp.set_value(String::new(), window, cx);
                                });
                                self.basic_pass_input.update(cx, |inp, cx| {
                                    inp.set_value(String::new(), window, cx);
                                });
                                self.apikey_key_input.update(cx, |inp, cx| {
                                    inp.set_value(String::new(), window, cx);
                                });
                                self.apikey_val_input.update(cx, |inp, cx| {
                                    inp.set_value(String::new(), window, cx);
                                });
                            }
                        }

                        let pre_script = req.script.as_ref().and_then(|s| s.pre_request.clone()).unwrap_or_default();
                        self.pre_script_editor.update(cx, |txt, cx| {
                            txt.set_value(pre_script, window, cx);
                        });
                        let test_script = req.script.as_ref().and_then(|s| s.post_response.clone()).unwrap_or_default();
                        self.test_script_editor.update(cx, |txt, cx| {
                            txt.set_value(test_script, window, cx);
                        });
                    }

                    if req_switched || req_ver_changed || body_ver_changed {
                        self.last_synced_body_version = ed_body_ver;
                        match &req.body {
                            RequestBody::Json(json_str) => {
                                self.json_editor.update(cx, |txt, cx| {
                                    txt.set_value(json_str.clone(), window, cx);
                                });
                            }
                            RequestBody::Raw { content, content_type } => {
                                if content_type == "application/xml" || content_type == "text/xml" {
                                    self.xml_editor.update(cx, |txt, cx| {
                                        txt.set_value(content.clone(), window, cx);
                                    });
                                }
                                self.raw_textarea.update(cx, |txt, cx| {
                                    txt.set_value(content.clone(), window, cx);
                                });
                            }
                            RequestBody::None | RequestBody::FormUrlEncoded(_) | RequestBody::Multipart(_) | RequestBody::Binary { .. } => {
                                self.json_editor.update(cx, |txt, cx| {
                                    txt.set_value(String::new(), window, cx);
                                });
                                self.raw_textarea.update(cx, |txt, cx| {
                                    txt.set_value(String::new(), window, cx);
                                });
                            }
                        }
                    }
                }
            }
        }

        // Synchronize response_editor with ResponseState's formatted_body
        {
            let resp_s = self.response_state.read(cx);
            let current_resp_id = resp_s.current_response.as_ref().map(|r| r.id.clone());
            let current_mode = resp_s.view_mode;
            let current_ex = resp_s.active_example_name.clone();

            let resp_changed = current_resp_id != self.last_synced_response_id
                || Some(current_mode) != self.last_synced_response_view_mode
                || current_ex != self.last_synced_example_name;

            if resp_changed {
                self.last_synced_response_id = current_resp_id;
                self.last_synced_response_view_mode = Some(current_mode);
                self.last_synced_example_name = current_ex;

                let (display_text, is_json, is_xml) = if let Some(fmt) = &resp_s.formatted_body {
                    (fmt.display_text.to_string(), fmt.is_json, fmt.is_xml)
                } else if let Some(resp) = &resp_s.current_response {
                    let text = String::from_utf8_lossy(&resp.body).to_string();
                    let is_j = resp.is_json() || text.trim_start().starts_with('{') || text.trim_start().starts_with('[');
                    let is_x = !is_j && (resp.is_xml() || text.trim_start().starts_with("<?xml") || (text.trim_start().starts_with('<') && text.trim_end().ends_with('>')));
                    (text, is_j, is_x)
                } else {
                    (String::new(), false, false)
                };

                let lang = if is_json {
                    "json"
                } else if is_xml {
                    "xml"
                } else {
                    "plaintext"
                };

                self.response_editor.update(cx, |txt, cx| {
                    txt.set_value(display_text, window, cx);
                    txt.set_highlighter(lang, cx);
                    txt.set_readonly(true, cx);
                });
            }
        }

        // Synchronize request_url_textarea and request_body_editor with RequestSent
        {
            let (sent_url, body_text, lang) = {
                let resp_s = self.response_state.read(cx);
                if let Some(resp) = &resp_s.current_response {
                    if let Some(sent) = &resp.request_sent {
                        let (body_text, lang) = if let Some(preview) = &sent.body_preview {
                            let is_j = preview.trim_start().starts_with('{') || preview.trim_start().starts_with('[');
                            let text = if is_j {
                                serde_json::from_str::<serde_json::Value>(preview)
                                    .map(|v| serde_json::to_string_pretty(&v).unwrap_or(preview.clone()))
                                    .unwrap_or_else(|_| preview.clone())
                            } else {
                                preview.clone()
                            };
                            (text, if is_j { "json" } else { "plaintext" })
                        } else {
                            (String::new(), "plaintext")
                        };
                        (Some(sent.url.clone()), Some(body_text), Some(lang))
                    } else {
                        (None, None, None)
                    }
                } else {
                    (None, None, None)
                }
            };

            if let Some(sent_url) = sent_url {
                self.request_url_textarea.update(cx, |txt, cx| {
                    if txt.value() != sent_url {
                        txt.set_value(sent_url, window, cx);
                        txt.set_readonly(true, cx);
                    }
                });
            }
            if let (Some(body_text), Some(lang)) = (body_text, lang) {
                self.request_body_editor.update(cx, |txt, cx| {
                    if txt.value() != body_text {
                        txt.set_value(body_text, window, cx);
                        txt.set_highlighter(lang, cx);
                        txt.set_readonly(true, cx);
                    }
                });
            }
        }

        let ws_entity = self.workspace_state.clone();
        let ed_entity = self.editor_state.clone();
        let resp_entity = self.response_state.clone();
        let toast_entity = self.toast_state.clone();

        let app_view_entity = cx.entity().clone();
        let req_id_str = active_req_id.clone().unwrap_or_else(|| "scratch".to_string());
        let ws_read = ws_entity.read(cx);
        let active_conflict = ws_read.conflicts.get(&req_id_str).cloned();
        let active_parse_err = ws_read.parse_errors.get(&req_id_str).cloned();
        let corrupt_sec_err = ws_read.corrupt_secrets_warning.clone();

        let res = div()
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
                move |_: &ToggleGitModal, _, cx| {
                    ws_c.update(cx, |state, cx| {
                        state.toggle_git_modal(cx);
                    });
                }
            })
            .on_action({
                let ws_c = ws_entity.clone();
                move |_: &OpenImportModal, _, cx| {
                    ws_c.update(cx, |state, cx| {
                        state.open_import_modal(None, cx);
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
            .on_action({
                let ws_c = ws_entity.clone();
                move |_: &DismissModal, _, cx| {
                    ws_c.update(cx, |state, cx| {
                        state.dismiss_top_modal(cx);
                    });
                }
            })
            .on_key_down({
                let ws_c = ws_entity.clone();
                move |event, _, cx| {
                    if event.keystroke.key.as_str() == "escape" {
                        ws_c.update(cx, |state, cx| {
                            state.dismiss_top_modal(cx);
                        });
                    }
                }
            })
            // Custom Title Bar
            .child(AppTitleBar::new(ws_entity.clone(), ed_entity.clone(), resp_entity.clone()))
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
                                                            .on_click(move |_, _, cx| {
                                                                ed_c.update(cx, |state, cx| {
                                                                    state.set_active_tab_kind(RequestTabKind::Body, cx);
                                                                });
                                                            })
                                                    })
                                                    .child({
                                                        let ed_c = ed_entity.clone();
                                                        let param_count = active_req_info.as_ref().map(|info| info.0).unwrap_or(0);
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
                                                        let header_count = active_req_info.as_ref().map(|info| info.1).unwrap_or(0);
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
                                                        TabItem::new("tab-auth", "Auth")
                                                            .active(active_tab_kind == RequestTabKind::Auth)
                                                            .on_click(move |_, _, cx| {
                                                                ed_c.update(cx, |state, cx| {
                                                                    state.set_active_tab_kind(RequestTabKind::Auth, cx);
                                                                });
                                                            })
                                                    })
                                                    .child({
                                                        let ed_c = ed_entity.clone();
                                                        let has_pre = active_req_info.as_ref().map(|info| info.2).unwrap_or(false);
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
                                                        let has_test = active_req_info.as_ref().map(|info| info.3).unwrap_or(false);
                                                        TabItem::new("tab-tests", "Post-response")
                                                            .active(active_tab_kind == RequestTabKind::Tests)
                                                            .when(has_test, |tab| tab.badge("●"))
                                                            .on_click(move |_, _, cx| {
                                                                ed_c.update(cx, |state, cx| {
                                                                    state.set_active_tab_kind(RequestTabKind::Tests, cx);
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
                                                                        self.json_editor.clone(),
                                                                        self.xml_editor.clone(),
                                                                        self.raw_textarea.clone(),
                                                                    )
                                                                    .workspace_state(ws_entity.clone())
                                                                    .toast_state(self.toast_state.clone())
                                                                )
                                                            }
                                                            RequestTabKind::Params => {
                                                                let params = ed_entity.read(cx).active_request.as_ref().map(|r| r.params.clone()).unwrap_or_default();
                                                                let ed_up = ed_entity.clone();
                                                                let ws_up = ws_entity.clone();
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
                                                                            save_active_request_without_redraw(&ed_up, &ws_up, cx);
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
                                                                let headers = ed_entity.read(cx).active_request.as_ref().map(|r| r.headers.clone()).unwrap_or_default();
                                                                let inherited_count = {
                                                                    let active_r = ed_entity.read(cx).active_request.clone();
                                                                    if let Some(r) = active_r {
                                                                        let ws_r = ws_entity.read(cx);
                                                                        let ws_h = ws_r.active_workspace.as_ref().map(|w| w.headers.len()).unwrap_or(0);
                                                                        let folder_h: usize = ws_r.get_folder_hierarchy(r.folder_id.as_deref())
                                                                            .iter()
                                                                            .map(|f| f.headers.len())
                                                                            .sum();
                                                                        ws_h + folder_h
                                                                    } else {
                                                                        0
                                                                    }
                                                                };
                                                                let ed_up = ed_entity.clone();
                                                                let ws_up = ws_entity.clone();
                                                                let ed_add = ed_entity.clone();
                                                                let ed_del = ed_entity.clone();
                                                                div()
                                                                    .size_full()
                                                                    .flex()
                                                                    .flex_col()
                                                                    .when(inherited_count > 0, |this| {
                                                                        this.child(
                                                                            div()
                                                                                .px(px(12.))
                                                                                .py(px(6.))
                                                                                .bg(t.bg_app)
                                                                                .border_b_1()
                                                                                .border_color(t.border)
                                                                                .flex()
                                                                                .items_center()
                                                                                .gap(px(6.))
                                                                                .child(Icon::new(IconName::Info).size(px(14.)))
                                                                                .child(
                                                                                    div()
                                                                                        .text_xs()
                                                                                        .text_color(t.text_secondary)
                                                                                        .child(format!("{} inherited header(s) from parent folder/workspace will be merged on send", inherited_count)),
                                                                                ),
                                                                        )
                                                                    })
                                                                    .child(
                                                                        div().flex_1().child(
                                                                            KvTable::new(
                                                                                format!("headers-kv-{}", req_id_str),
                                                                                headers,
                                                                                move |idx, k, v, enabled, _, cx| {
                                                                                    ed_up.update(cx, |state, cx| {
                                                                                        state.update_header(idx, k, v, enabled, cx);
                                                                                    });
                                                                                    save_active_request_without_redraw(&ed_up, &ws_up, cx);
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
                                                                    )
                                                            }
                                                            RequestTabKind::Auth => {
                                                                let auth = ed_entity.read(cx).active_request.as_ref().map(|r| r.auth.clone()).unwrap_or(Auth::Inherit);
                                                                let ed_c = ed_entity.clone();
                                                                let auth_label = match &auth {
                                                                    Auth::Inherit => "Inherit",
                                                                    Auth::None => "None",
                                                                    Auth::Bearer { .. } => "Bearer Token",
                                                                    Auth::Basic { .. } => "Basic Auth",
                                                                    Auth::ApiKey { .. } => "API Key",
                                                                };
                                                                let is_inherit = matches!(auth, Auth::Inherit);
                                                                let is_none = matches!(auth, Auth::None);
                                                                let is_bearer = matches!(auth, Auth::Bearer { .. });
                                                                let is_basic = matches!(auth, Auth::Basic { .. });
                                                                let is_apikey = matches!(auth, Auth::ApiKey { .. });

                                                                div()
                                                                    .p(px(16.))
                                                                    .flex()
                                                                    .flex_col()
                                                                    .gap(px(10.))
                                                                    .child(
                                                                        div()
                                                                            .flex()
                                                                            .items_center()
                                                                            .gap(px(10.))
                                                                            .child(
                                                                                div()
                                                                                    .w(px(70.))
                                                                                    .text_xs()
                                                                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                                                                    .text_color(t.text_secondary)
                                                                                    .child("Type:")
                                                                            )
                                                                            .child({
                                                                                let ed_m = ed_c.clone();
                                                                                Button::new("btn-auth-type-dropdown")
                                                                                    .ghost()
                                                                                    .compact()
                                                                                    .px(px(8.))
                                                                                    .py(px(4.))
                                                                                    .h(px(28.))
                                                                                    .rounded(px(4.))
                                                                                    .bg(t.bg_card)
                                                                                    .border_1()
                                                                                    .border_color(t.border)
                                                                                    .child(
                                                                                        div()
                                                                                            .flex()
                                                                                            .items_center()
                                                                                            .gap(px(6.))
                                                                                            .child(
                                                                                                div()
                                                                                                    .text_xs()
                                                                                                    .font_weight(gpui::FontWeight::MEDIUM)
                                                                                                    .text_color(t.text_primary)
                                                                                                    .child(auth_label),
                                                                                            )
                                                                                            .child(
                                                                                                div()
                                                                                                    .text_xs()
                                                                                                    .text_color(t.text_muted)
                                                                                                    .child("▾"),
                                                                                            ),
                                                                                    )
                                                                                    .dropdown_menu(move |menu, _window, _cx| {
                                                                                        let ed_inh = ed_m.clone();
                                                                                        let ed_none = ed_m.clone();
                                                                                        let ed_bearer = ed_m.clone();
                                                                                        let ed_basic = ed_m.clone();
                                                                                        let ed_apikey = ed_m.clone();

                                                                                        menu.item(
                                                                                            PopupMenuItem::new("Inherit")
                                                                                                .checked(is_inherit)
                                                                                                .on_click(move |_, _, cx| {
                                                                                                    ed_inh.update(cx, |state, cx| {
                                                                                                        if !matches!(state.active_request.as_ref().map(|r| &r.auth), Some(Auth::Inherit)) {
                                                                                                            state.set_auth(Auth::Inherit, cx);
                                                                                                        }
                                                                                                    });
                                                                                                }),
                                                                                        )
                                                                                        .item(
                                                                                            PopupMenuItem::new("None")
                                                                                                .checked(is_none)
                                                                                                .on_click(move |_, _, cx| {
                                                                                                    ed_none.update(cx, |state, cx| {
                                                                                                        if !matches!(state.active_request.as_ref().map(|r| &r.auth), Some(Auth::None)) {
                                                                                                            state.set_auth(Auth::None, cx);
                                                                                                        }
                                                                                                    });
                                                                                                }),
                                                                                        )
                                                                                        .item(
                                                                                            PopupMenuItem::new("Bearer Token")
                                                                                                .checked(is_bearer)
                                                                                                .on_click(move |_, _, cx| {
                                                                                                    ed_bearer.update(cx, |state, cx| {
                                                                                                        if !matches!(state.active_request.as_ref().map(|r| &r.auth), Some(Auth::Bearer { .. })) {
                                                                                                            state.set_auth(Auth::Bearer { token: "".to_string() }, cx);
                                                                                                        }
                                                                                                    });
                                                                                                }),
                                                                                        )
                                                                                        .item(
                                                                                            PopupMenuItem::new("Basic Auth")
                                                                                                .checked(is_basic)
                                                                                                .on_click(move |_, _, cx| {
                                                                                                    ed_basic.update(cx, |state, cx| {
                                                                                                        if !matches!(state.active_request.as_ref().map(|r| &r.auth), Some(Auth::Basic { .. })) {
                                                                                                            state.set_auth(Auth::Basic { username: "".to_string(), password: "".to_string() }, cx);
                                                                                                        }
                                                                                                    });
                                                                                                }),
                                                                                        )
                                                                                        .item(
                                                                                            PopupMenuItem::new("API Key")
                                                                                                .checked(is_apikey)
                                                                                                .on_click(move |_, _, cx| {
                                                                                                    ed_apikey.update(cx, |state, cx| {
                                                                                                        if !matches!(state.active_request.as_ref().map(|r| &r.auth), Some(Auth::ApiKey { .. })) {
                                                                                                            state.set_auth(Auth::ApiKey { key: "X-API-Key".to_string(), value: "".to_string(), in_header: true }, cx);
                                                                                                        }
                                                                                                    });
                                                                                                }),
                                                                                        )
                                                                                    })
                                                                            })
                                                                    )
                                                                    .child({
                                                                        match auth {
                                                                            Auth::Inherit => {
                                                                                let folder_id = ed_c.read(cx).active_request.as_ref().and_then(|r| r.folder_id.clone());
                                                                                let desc = ws_entity.read(cx).get_inherited_auth_description(folder_id.as_deref());
                                                                                div()
                                                                                    .p(px(10.))
                                                                                    .rounded(px(6.))
                                                                                    .bg(t.bg_app)
                                                                                    .border_1()
                                                                                    .border_color(t.border)
                                                                                    .flex()
                                                                                    .items_center()
                                                                                    .gap(px(8.))
                                                                                    .child(Icon::new(IconName::Info).size(px(16.)))
                                                                                    .child(
                                                                                        div()
                                                                                            .text_xs()
                                                                                            .text_color(t.text_secondary)
                                                                                            .child(desc),
                                                                                    )
                                                                            }
                                                                            Auth::None => {
                                                                                div()
                                                                                    .pt(px(4.))
                                                                                    .text_xs()
                                                                                    .italic()
                                                                                    .text_color(t.text_muted)
                                                                                    .child("No authentication will be added to this request.")
                                                                            }
                                                                            Auth::Bearer { .. } => {
                                                                                let ed_b = ed_c.clone();
                                                                                div()
                                                                                    .flex()
                                                                                    .items_center()
                                                                                    .gap(px(10.))
                                                                                    .child(
                                                                                        div()
                                                                                            .w(px(70.))
                                                                                            .text_xs()
                                                                                            .font_weight(gpui::FontWeight::SEMIBOLD)
                                                                                            .text_color(t.text_secondary)
                                                                                            .child("Token:")
                                                                                    )
                                                                                    .child(
                                                                                        div()
                                                                                            .w_full()
                                                                                            .max_w(px(400.))
                                                                                            .child(
                                                                                                self.render_auth_input(
                                                                                                    "auth-bearer-token-input",
                                                                                                    &self.bearer_token_input,
                                                                                                    window,
                                                                                                    cx,
                                                                                                    move |new_token, _window, cx| {
                                                                                                        ed_b.update(cx, |ed, cx| {
                                                                                                            ed.set_bearer_token(new_token, cx);
                                                                                                        });
                                                                                                    },
                                                                                                ),
                                                                                            ),
                                                                                    )
                                                                            }
                                                                            Auth::Basic { .. } => {
                                                                                let ed_u = ed_c.clone();
                                                                                let ed_p = ed_c.clone();
                                                                                let p_in = self.basic_pass_input.clone();
                                                                                let u_in = self.basic_user_input.clone();
                                                                                div()
                                                                                    .flex()
                                                                                    .flex_col()
                                                                                    .gap(px(10.))
                                                                                    .child(
                                                                                        div()
                                                                                            .flex()
                                                                                            .items_center()
                                                                                            .gap(px(10.))
                                                                                            .child(
                                                                                                div()
                                                                                                    .w(px(70.))
                                                                                                    .text_xs()
                                                                                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                                                                                    .text_color(t.text_secondary)
                                                                                                    .child("Username:")
                                                                                            )
                                                                                            .child(
                                                                                                div()
                                                                                                    .w_full()
                                                                                                    .max_w(px(400.))
                                                                                                    .child(
                                                                                                        self.render_auth_input(
                                                                                                            "auth-basic-user-input",
                                                                                                            &self.basic_user_input,
                                                                                                            window,
                                                                                                            cx,
                                                                                                            move |new_user, _window, cx| {
                                                                                                                let pass = p_in.read(cx).value().to_string();
                                                                                                                ed_u.update(cx, |ed, cx| {
                                                                                                                    ed.set_basic_auth(new_user, pass, cx);
                                                                                                                });
                                                                                                            },
                                                                                                        ),
                                                                                                    ),
                                                                                            ),
                                                                                    )
                                                                                    .child(
                                                                                        div()
                                                                                            .flex()
                                                                                            .items_center()
                                                                                            .gap(px(10.))
                                                                                            .child(
                                                                                                div()
                                                                                                    .w(px(70.))
                                                                                                    .text_xs()
                                                                                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                                                                                    .text_color(t.text_secondary)
                                                                                                    .child("Password:")
                                                                                            )
                                                                                            .child(
                                                                                                div()
                                                                                                    .w_full()
                                                                                                    .max_w(px(400.))
                                                                                                    .child(
                                                                                                        self.render_auth_input(
                                                                                                            "auth-basic-pass-input",
                                                                                                            &self.basic_pass_input,
                                                                                                            window,
                                                                                                            cx,
                                                                                                            move |new_pass, _window, cx| {
                                                                                                                let user = u_in.read(cx).value().to_string();
                                                                                                                ed_p.update(cx, |ed, cx| {
                                                                                                                    ed.set_basic_auth(user, new_pass, cx);
                                                                                                                });
                                                                                                            },
                                                                                                        ),
                                                                                                    ),
                                                                                            ),
                                                                                    )
                                                                            }
                                                                            Auth::ApiKey { in_header, .. } => {
                                                                                let ed_loc = ed_c.clone();
                                                                                let ed_k = ed_c.clone();
                                                                                let ed_v = ed_c.clone();
                                                                                let v_in = self.apikey_val_input.clone();
                                                                                let k_in = self.apikey_key_input.clone();
                                                                                let loc_label = if in_header { "Header" } else { "Query Params" };

                                                                                div()
                                                                                    .flex()
                                                                                    .flex_col()
                                                                                    .gap(px(10.))
                                                                                    .child(
                                                                                        div()
                                                                                            .flex()
                                                                                            .items_center()
                                                                                            .gap(px(10.))
                                                                                            .child(
                                                                                                div()
                                                                                                    .w(px(70.))
                                                                                                    .text_xs()
                                                                                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                                                                                    .text_color(t.text_secondary)
                                                                                                    .child("Key:")
                                                                                            )
                                                                                            .child(
                                                                                                div()
                                                                                                    .w_full()
                                                                                                    .max_w(px(400.))
                                                                                                    .child(
                                                                                                        self.render_auth_input(
                                                                                                            "auth-apikey-key-input",
                                                                                                            &self.apikey_key_input,
                                                                                                            window,
                                                                                                            cx,
                                                                                                            move |new_key, _window, cx| {
                                                                                                                let val = v_in.read(cx).value().to_string();
                                                                                                                ed_k.update(cx, |ed, cx| {
                                                                                                                    ed.set_api_key(new_key, val, in_header, cx);
                                                                                                                });
                                                                                                            },
                                                                                                        ),
                                                                                                    ),
                                                                                            ),
                                                                                    )
                                                                                    .child(
                                                                                        div()
                                                                                            .flex()
                                                                                            .items_center()
                                                                                            .gap(px(10.))
                                                                                            .child(
                                                                                                div()
                                                                                                    .w(px(70.))
                                                                                                    .text_xs()
                                                                                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                                                                                    .text_color(t.text_secondary)
                                                                                                    .child("Value:")
                                                                                            )
                                                                                            .child(
                                                                                                div()
                                                                                                    .w_full()
                                                                                                    .max_w(px(400.))
                                                                                                    .child(
                                                                                                        self.render_auth_input(
                                                                                                            "auth-apikey-val-input",
                                                                                                            &self.apikey_val_input,
                                                                                                            window,
                                                                                                            cx,
                                                                                                            move |new_val, _window, cx| {
                                                                                                                let key = k_in.read(cx).value().to_string();
                                                                                                                ed_v.update(cx, |ed, cx| {
                                                                                                                    ed.set_api_key(key, new_val, in_header, cx);
                                                                                                                });
                                                                                                            },
                                                                                                        ),
                                                                                                    ),
                                                                                            ),
                                                                                    )
                                                                                    .child(
                                                                                        div()
                                                                                            .flex()
                                                                                            .items_center()
                                                                                            .gap(px(10.))
                                                                                            .child(
                                                                                                div()
                                                                                                    .w(px(70.))
                                                                                                    .text_xs()
                                                                                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                                                                                    .text_color(t.text_secondary)
                                                                                                    .child("Add to:")
                                                                                            )
                                                                                            .child({
                                                                                                let ed_l = ed_loc.clone();
                                                                                                let k_input = self.apikey_key_input.clone();
                                                                                                let v_input = self.apikey_val_input.clone();
                                                                                                Button::new("btn-auth-apikey-location-dropdown")
                                                                                                    .ghost()
                                                                                                    .compact()
                                                                                                    .px(px(8.))
                                                                                                    .py(px(4.))
                                                                                                    .h(px(28.))
                                                                                                    .rounded(px(4.))
                                                                                                    .bg(t.bg_card)
                                                                                                    .border_1()
                                                                                                    .border_color(t.border)
                                                                                                    .child(
                                                                                                        div()
                                                                                                            .flex()
                                                                                                            .items_center()
                                                                                                            .gap(px(6.))
                                                                                                            .child(
                                                                                                                div()
                                                                                                                    .text_xs()
                                                                                                                    .font_weight(gpui::FontWeight::MEDIUM)
                                                                                                                    .text_color(t.text_primary)
                                                                                                                    .child(loc_label),
                                                                                                            )
                                                                                                            .child(
                                                                                                                div()
                                                                                                                    .text_xs()
                                                                                                                    .text_color(t.text_muted)
                                                                                                                    .child("▾"),
                                                                                                            ),
                                                                                                    )
                                                                                                    .dropdown_menu(move |menu, _window, _cx| {
                                                                                                        let ed_h = ed_l.clone();
                                                                                                        let ed_q = ed_l.clone();
                                                                                                        let k_h = k_input.clone();
                                                                                                        let v_h = v_input.clone();
                                                                                                        let k_q = k_input.clone();
                                                                                                        let v_q = v_input.clone();

                                                                                                        menu.item(
                                                                                                            PopupMenuItem::new("Header")
                                                                                                                .checked(in_header)
                                                                                                                .on_click(move |_, _, cx| {
                                                                                                                    let key = k_h.read(cx).value().to_string();
                                                                                                                    let val = v_h.read(cx).value().to_string();
                                                                                                                    ed_h.update(cx, |ed, cx| {
                                                                                                                        ed.set_api_key(key, val, true, cx);
                                                                                                                    });
                                                                                                                }),
                                                                                                        )
                                                                                                        .item(
                                                                                                            PopupMenuItem::new("Query Params")
                                                                                                                .checked(!in_header)
                                                                                                                .on_click(move |_, _, cx| {
                                                                                                                    let key = k_q.read(cx).value().to_string();
                                                                                                                    let val = v_q.read(cx).value().to_string();
                                                                                                                    ed_q.update(cx, |ed, cx| {
                                                                                                                        ed.set_api_key(key, val, false, cx);
                                                                                                                    });
                                                                                                                }),
                                                                                                        )
                                                                                                    })
                                                                                            }),
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

                                        let response_pane = ResponseViewer::new(
                                            ws_entity.clone(),
                                            ed_entity.clone(),
                                            resp_entity.clone(),
                                            self.response_editor.clone(),
                                            self.request_url_textarea.clone(),
                                            self.request_body_editor.clone(),
                                        );

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
                                        .child(SidebarTree::new(ws_entity.clone(), ed_entity.clone(), resp_entity.clone(), toast_entity.clone())),
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
            .when(show_scope_settings_modal, |this| {
                this.child(ScopeSettingsModal::new(ws_entity.clone()))
            })
            .when(show_git_modal, |this| {
                this.child(GitModal::new(ws_entity.clone()))
            })
            .when(show_import_modal, |this| {
                this.child(ImportModal::new(ws_entity.clone(), toast_entity.clone()))
            })
            .when(show_rename_workspace_modal, |this| {
                this.child(RenameWorkspaceModal::new(ws_entity.clone()))
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
            .child(ToastContainer::new(toast_entity));

        res
    }
}

fn main() {
    #[cfg(target_os = "macos")]
    {
        // Suppress benign internal Apple InputMethodKit / TSM logs
        // (such as "ERROR: getApplicationProperty: called with invalid property")
        // emitted when typing with macOS Vietnamese Telex / CJK input methods.
        if std::env::var("OS_ACTIVITY_MODE").is_err() {
            std::env::set_var("OS_ACTIVITY_MODE", "disable");
        }
    }

    let app = gpui_kit::application().with_assets(AllAssets);

    app.run(move |cx| {
        platform::set_dock_icon();
        gpui_kit::init(cx);

        let settings = settings::AppSettings::load();
        let pref = theme::ThemePreference::from_str_opt(settings.theme.as_deref());
        let mode = theme::resolve_theme_mode(pref, cx.window_appearance());
        theme::set_theme(pref, mode);

        let gpui_mode = match mode {
            theme::ThemeMode::Dark => gpui_component::ThemeMode::Dark,
            theme::ThemeMode::Light => gpui_component::ThemeMode::Light,
        };
        gpui_component::Theme::change(gpui_mode, None, cx);

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
            KeyBinding::new("ctrl-shift-g", ToggleGitModal, None),
            KeyBinding::new("cmd-shift-g", ToggleGitModal, None),
            KeyBinding::new("ctrl-i", OpenImportModal, None),
            KeyBinding::new("cmd-i", OpenImportModal, None),
            KeyBinding::new("ctrl-[", NavigateBack, None),
            KeyBinding::new("cmd-[", NavigateBack, None),
            KeyBinding::new("ctrl-]", NavigateForward, None),
            KeyBinding::new("cmd-]", NavigateForward, None),
            KeyBinding::new("escape", DismissModal, None),
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



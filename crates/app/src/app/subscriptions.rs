use super::sync::save_active_request_without_redraw;
use super::AppView;
use crate::state::{
    EditorState, GitState, ModalState, ResponseState, SidebarState, ToastState, WorkspaceDataState,
};
use gpui::*;
use gpui_component::input::{
    EditorState as CodeEditorState, InputEvent, InputState, TextareaState,
};
use restclient_core::models::{Auth, HttpRequestItem, RequestBody};

pub struct SubscriptionTargets<'a> {
    pub workspace_state: &'a Entity<WorkspaceDataState>,
    pub modal_state: &'a Entity<ModalState>,
    pub sidebar_state: &'a Entity<SidebarState>,
    pub git_state: &'a Entity<GitState>,
    pub editor_state: &'a Entity<EditorState>,
    pub response_state: &'a Entity<ResponseState>,
    pub toast_state: &'a Entity<ToastState>,
    pub url_input: &'a Entity<InputState>,
    pub json_editor: &'a Entity<CodeEditorState>,
    pub xml_editor: &'a Entity<CodeEditorState>,
    pub raw_textarea: &'a Entity<TextareaState>,
    pub bearer_token_input: &'a Entity<InputState>,
    pub basic_user_input: &'a Entity<InputState>,
    pub basic_pass_input: &'a Entity<InputState>,
    pub apikey_key_input: &'a Entity<InputState>,
    pub apikey_val_input: &'a Entity<InputState>,
    pub pre_script_editor: &'a Entity<CodeEditorState>,
    pub test_script_editor: &'a Entity<CodeEditorState>,
}

pub fn setup_app_subscriptions(
    targets: SubscriptionTargets,
    initial_req: Option<&HttpRequestItem>,
    window: &mut Window,
    cx: &mut Context<AppView>,
) -> Vec<Subscription> {
    let mut subscriptions = Vec::new();

    let editor_state = targets.editor_state;
    let workspace_state = targets.workspace_state;
    let url_input = targets.url_input;
    let json_editor = targets.json_editor;
    let xml_editor = targets.xml_editor;
    let raw_textarea = targets.raw_textarea;
    let bearer_token_input = targets.bearer_token_input;
    let basic_user_input = targets.basic_user_input;
    let basic_pass_input = targets.basic_pass_input;
    let apikey_key_input = targets.apikey_key_input;
    let apikey_val_input = targets.apikey_val_input;
    let pre_script_editor = targets.pre_script_editor;
    let test_script_editor = targets.test_script_editor;
    let response_state = targets.response_state;

    // 1. Subscribe to URL input change
    let ed_url = editor_state.clone();
    let ws_url = workspace_state.clone();
    subscriptions.push(cx.subscribe(url_input, move |this, input, event, cx| {
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
                            if val.lines().count() > 1 || val.contains(" -") || val.contains(" --")
                            {
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
    subscriptions.push(cx.subscribe(json_editor, move |_, txt, event, cx| {
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
    subscriptions.push(cx.subscribe(xml_editor, move |_, txt, event, cx| {
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
    subscriptions.push(cx.subscribe(raw_textarea, move |_, txt, event, cx| {
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
    subscriptions.push(cx.subscribe(bearer_token_input, move |_, inp, event, cx| {
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
    subscriptions.push(cx.subscribe(basic_user_input, move |_, inp, event, cx| {
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
    subscriptions.push(cx.subscribe(basic_pass_input, move |_, inp, event, cx| {
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
    subscriptions.push(cx.subscribe(apikey_key_input, move |_, inp, event, cx| {
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
    subscriptions.push(cx.subscribe(apikey_val_input, move |_, inp, event, cx| {
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
    subscriptions.push(cx.subscribe(pre_script_editor, move |_, txt, event, cx| {
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
    subscriptions.push(cx.subscribe(test_script_editor, move |_, txt, event, cx| {
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
    let mut last_req_id = initial_req.map(|r| r.id.clone());
    subscriptions.push(cx.observe(editor_state, move |_, ed, cx| {
        let req_switch_info = {
            let ed_ref = ed.read(cx);
            ed_ref
                .active_request
                .as_ref()
                .map(|r| (r.id.clone(), r.workspace_id == "scratch" || r.id.is_empty()))
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
    subscriptions.push(cx.observe(workspace_state, move |_, ws, cx| {
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
            let active_id = ed_sync
                .read(cx)
                .active_request
                .as_ref()
                .map(|r| r.id.clone());
            let (saved_name_for_active, saved_examples_for_active) =
                if let Some(ref a_id) = active_id {
                    let req = ws_ref.requests.iter().find(|r| &r.id == a_id);
                    (
                        req.map(|r| r.name.clone()),
                        req.map(|r| r.examples.clone()),
                    )
                } else {
                    (None, None)
                };
            (
                last_renamed,
                matching_req,
                saved_name_for_active,
                saved_examples_for_active,
            )
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
    subscriptions.push(cx.observe(response_state, |_, _, cx| {
        cx.notify();
    }));

    // 7. Observe sidebar_state, modal_state, git_state: re-render AppView on UI changes
    subscriptions.push(cx.observe(targets.sidebar_state, |_, _, cx| {
        cx.notify();
    }));
    subscriptions.push(cx.observe(targets.modal_state, |_, _, cx| {
        cx.notify();
    }));
    subscriptions.push(cx.observe(targets.git_state, |_, _, cx| {
        cx.notify();
    }));

    // 8. Observe OS appearance changes for System theme preference
    let modal_theme = targets.modal_state.clone();
    subscriptions.push(window.observe_window_appearance(move |window, cx| {
        let app_appearance = window.appearance();
        modal_theme.update(cx, |modal, cx| {
            if modal.theme_preference == crate::theme::ThemePreference::System {
                let mode = crate::theme::resolve_theme_mode(
                    crate::theme::ThemePreference::System,
                    app_appearance,
                );
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

    subscriptions
}

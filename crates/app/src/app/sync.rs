use super::AppView;
use crate::state::{EditorState, PersistenceState, WorkspaceDataState};
use gpui::{App, Entity, Window};
use restclient_core::models::{Auth, RequestBody};

pub fn save_active_request_without_redraw(
    editor: &Entity<EditorState>,
    persistence: &Entity<PersistenceState>,
    workspace: &Entity<WorkspaceDataState>,
    cx: &mut App,
) {
    let is_scratch_or_none = editor
        .read(cx)
        .active_request
        .as_ref()
        .map(|r| r.workspace_id == "scratch" || r.id.is_empty())
        .unwrap_or(true);
    if !is_scratch_or_none {
        if let Some(req_id) = editor.read(cx).active_request.as_ref().map(|r| r.id.clone()) {
            let ed = editor.clone();
            let ws = workspace.clone();
            persistence.update(cx, |per, cx| per.schedule_save_active_request(req_id, ed, ws, cx));
        }
    }
}

impl AppView {
    pub(crate) fn sync_active_request_editors(&mut self, window: &mut Window, cx: &mut App) {
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
            (
                switched,
                req_changed,
                body_changed,
                ed.body_version,
                ed.request_version,
            )
        };

        if !req_switched && !req_ver_changed && !body_ver_changed {
            return;
        }

        let req_opt = self.editor_state.read(cx).active_request.clone();
        let Some(req) = req_opt else {
            return;
        };

        if req_switched || req_ver_changed {
            self.last_synced_request_id = Some(req.id.clone());
            self.last_synced_request_version = ed_req_ver;
            self.last_synced_body_version = ed_body_ver;

            let tokenized_url = {
                let env = self.env_state.read(cx);
                crate::components::env_autocomplete::build_tokenized_content(&req.url, env)
            };
            self.url_input.update(cx, |inp, cx| {
                inp.set_value(tokenized_url, window, cx);
            });

            let env_holder = self.env_state.clone();
            match &req.auth {
                Auth::Bearer { token } => {
                    let env_m = env_holder.clone();
                    self.bearer_token_input.update(cx, |inp, cx| {
                        let env = env_m.read(cx);
                        inp.set_value(
                            crate::components::env_autocomplete::build_tokenized_content(token, env),
                            window,
                            cx,
                        );
                    });
                }
                Auth::Basic { username, password } => {
                    let env_u = env_holder.clone();
                    let env_p = env_holder.clone();
                    self.basic_user_input.update(cx, |inp, cx| {
                        let env = env_u.read(cx);
                        inp.set_value(
                            crate::components::env_autocomplete::build_tokenized_content(
                                username, env,
                            ),
                            window,
                            cx,
                        );
                    });
                    self.basic_pass_input.update(cx, |inp, cx| {
                        let env = env_p.read(cx);
                        inp.set_value(
                            crate::components::env_autocomplete::build_tokenized_content(
                                password, env,
                            ),
                            window,
                            cx,
                        );
                    });
                }
                Auth::ApiKey { key, value, .. } => {
                    let env_k = env_holder.clone();
                    let env_v = env_holder.clone();
                    self.apikey_key_input.update(cx, |inp, cx| {
                        let env = env_k.read(cx);
                        inp.set_value(
                            crate::components::env_autocomplete::build_tokenized_content(key, env),
                            window,
                            cx,
                        );
                    });
                    self.apikey_val_input.update(cx, |inp, cx| {
                        let env = env_v.read(cx);
                        inp.set_value(
                            crate::components::env_autocomplete::build_tokenized_content(value, env),
                            window,
                            cx,
                        );
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

            let pre_script = req
                .script
                .as_ref()
                .and_then(|s| s.pre_request.clone())
                .unwrap_or_default();
            self.pre_script_editor.update(cx, |txt, cx| {
                txt.set_value(pre_script, window, cx);
            });
            let test_script = req
                .script
                .as_ref()
                .and_then(|s| s.post_response.clone())
                .unwrap_or_default();
            self.test_script_editor.update(cx, |txt, cx| {
                txt.set_value(test_script, window, cx);
            });
        }

        if req_switched || req_ver_changed || body_ver_changed {
            self.last_synced_body_version = ed_body_ver;
            match &req.body {
                RequestBody::Json { content: json_str } => {
                    self.json_editor.update(cx, |txt, cx| {
                        txt.set_value(json_str.clone(), window, cx);
                    });
                }
                RequestBody::Raw {
                    content,
                    content_type,
                } => {
                    if content_type == "application/xml" || content_type == "text/xml" {
                        self.xml_editor.update(cx, |txt, cx| {
                            txt.set_value(content.clone(), window, cx);
                        });
                    }
                    self.raw_textarea.update(cx, |txt, cx| {
                        txt.set_value(content.clone(), window, cx);
                    });
                }
                RequestBody::None
                | RequestBody::FormUrlEncoded { .. }
                | RequestBody::Multipart { .. }
                | RequestBody::Binary { .. } => {
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

    pub(crate) fn sync_response_editor(&mut self, window: &mut Window, cx: &mut App) {
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
                let is_j = resp.is_json()
                    || text.trim_start().starts_with('{')
                    || text.trim_start().starts_with('[');
                let is_x = !is_j
                    && (resp.is_xml()
                        || text.trim_start().starts_with("<?xml")
                        || (text.trim_start().starts_with('<') && text.trim_end().ends_with('>')));
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

    pub(crate) fn sync_sent_request_preview(&mut self, window: &mut Window, cx: &mut App) {
        let (sent_url, body_text, lang) = {
            let resp_s = self.response_state.read(cx);
            if let Some(resp) = &resp_s.current_response {
                if let Some(sent) = &resp.request_sent {
                    let (body_text, lang) = if let Some(preview) = &sent.body_preview {
                        let is_j = preview.trim_start().starts_with('{')
                            || preview.trim_start().starts_with('[');
                        let text = if is_j {
                            serde_json::from_str::<serde_json::Value>(preview)
                                .map(|v| {
                                    serde_json::to_string_pretty(&v).unwrap_or(preview.clone())
                                })
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
}

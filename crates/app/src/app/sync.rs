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
    let req_id = editor.read(cx).active_request_id().to_string();
    if req_id != "scratch" && !req_id.is_empty() {
        let ed = editor.clone();
        let ws = workspace.clone();
        persistence.update(cx, |per, cx| per.schedule_save_active_request(req_id, ed, ws, cx));
    }
}

impl AppView {
    pub(crate) fn sync_active_request_editors(&mut self, window: &mut Window, cx: &mut App) {
        let (req_switched, req_ver_changed, body_ver_changed, ed_body_ver, ed_req_ver, active_id) = {
            let ed = self.editor_state.read(cx);
            let active_id = ed.active_request_id().to_string();
            let switched = match &self.last_synced_request_id {
                Some(last_id) => last_id != &active_id,
                None => true,
            };
            let req_changed = self.last_synced_request_version != ed.request_version;
            let body_changed = self.last_synced_body_version != ed.body_version;
            (
                switched,
                req_changed,
                body_changed,
                ed.body_version,
                ed.request_version,
                active_id,
            )
        };

        if !req_switched && !req_ver_changed && !body_ver_changed {
            return;
        }

        let Some(req) = self.workspace_state.read(cx).find_request(&active_id).cloned() else {
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
}

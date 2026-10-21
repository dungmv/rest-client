use gpui::*;
use restclient_core::http::{HttpError, HttpExecutor, HttpProgress, ResolvedRequest};
use restclient_core::models::{
    EnvironmentVariable, HttpResponseRecord,
};
use restclient_core::scripting::{
    resolve_fake_vars_in_request, PostResponseContext, PostResponseOutput, PreRequestContext,
    PreRequestOutput, ScriptSandbox,
};

use crate::app::AppView;

#[derive(Debug)]
pub enum HttpSendEvent {
    Progress(String),
    Success(Box<HttpResponseRecord>, PostResponseOutput),
    Cancelled,
    Error(String),
}

impl AppView {
    pub fn send_current_request(&self, cx: &mut Context<Self>) {
        let ed_entity = self.editor_state.clone();
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
        resolve_fake_vars_in_request(&mut req_item);

        let env_entity = self.env_state.clone();
        let cached_env = env_entity.read(cx).effective_environment();

        // 2. Pre-request Script execution
        let pre_script_code = req_item.script.as_ref().and_then(|s| s.pre_request.clone());
        let post_script_code = req_item.script.as_ref().and_then(|s| s.post_response.clone());

        let pre_output = if let Some(code) = pre_script_code {
            let pre_ctx = PreRequestContext::from_request_and_env(&req_item, Some(&cached_env));
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
        let mut mutated_env = None;
        let effective_env_ref = if !pre_output.env_mutations.is_empty() {
            let mut temp_env = (*cached_env).clone();
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
            mutated_env = Some(temp_env);
            mutated_env.as_ref()
        } else {
            Some(&*cached_env)
        };

        // 4. Resolve Request with effective environment & apply pre-script mutations
        let mut resolved = match ResolvedRequest::from_item(&req_item, effective_env_ref) {
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
            state.start_sending(Some(req_item.id.clone()), cancel_tx, cx);
        });
        let initial_logs = pre_output.console_entries.clone();
        let r_id = req_item.id.clone();
        self.response_state.update(cx, |resp, cx| {
            resp.start_sending(Some(&r_id), cx);
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

        let eff_env_for_post = mutated_env.map(std::sync::Arc::new).or(Some(cached_env));
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
                                let post_ctx = PostResponseContext::from_response_and_env(&record, eff_env_for_post.as_deref());
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
                                    this.env_state.update(cx, |env, cx| {
                                        if let Some(act_env) = env.active_environment_mut() {
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
                                            let envs = env.environments.clone();
                                            env.save_environments(envs, cx);
                                        } else {
                                            for (k, v_opt) in &all_mutations {
                                                if let Some(val) = v_opt {
                                                    if let Some(var) = env.global_environment.variables.iter_mut().find(|v| v.key == *k) {
                                                        var.current_value = val.clone();
                                                    } else {
                                                        env.global_environment.variables.push(EnvironmentVariable {
                                                            key: k.clone(),
                                                            initial_value: "".to_string(),
                                                            current_value: val.clone(),
                                                            secret: false,
                                                        });
                                                    }
                                                } else {
                                                    env.global_environment.variables.retain(|v| v.key != *k);
                                                }
                                            }
                                            let global = env.global_environment.clone();
                                            env.save_global_environment(global, cx);
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

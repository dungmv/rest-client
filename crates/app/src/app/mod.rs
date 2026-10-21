pub mod actions;
pub mod auth_input;
pub mod auth_tab;
pub mod editor_pane;
pub mod modals;
pub mod subscriptions;
pub mod sync;

pub use actions::*;
pub use modals::*;
pub use subscriptions::*;

use crate::components::*;
use crate::state::*;
use crate::theme::theme;
use gpui::*;
use gpui_component::input::{EditorState as CodeEditorState, InputState, TextareaState};
use gpui_component::resizable::{h_resizable, resizable_panel, v_resizable};
use restclient_core::http::ReqwestExecutor;
use restclient_core::models::{Auth, RequestBody};
use std::sync::Arc;

pub struct AppView {
    pub(crate) workspace_state: Entity<WorkspaceDataState>,
    pub(crate) env_state: Entity<EnvironmentState>,
    pub(crate) persistence_state: Entity<PersistenceState>,
    pub(crate) sidebar_state: Entity<SidebarState>,
    pub(crate) modal_state: Entity<ModalState>,
    pub(crate) git_state: Entity<GitState>,
    pub(crate) editor_state: Entity<EditorState>,
    pub(crate) response_state: Entity<ResponseState>,
    pub(crate) toast_state: Entity<ToastState>,
    pub(crate) url_input: Entity<InputState>,
    pub(crate) json_editor: Entity<CodeEditorState>,
    pub(crate) xml_editor: Entity<CodeEditorState>,
    pub(crate) raw_textarea: Entity<TextareaState>,
    pub(crate) response_editor: Entity<CodeEditorState>,
    pub(crate) request_url_textarea: Entity<TextareaState>,
    pub(crate) request_body_editor: Entity<CodeEditorState>,
    pub(crate) pre_script_editor: Entity<CodeEditorState>,
    pub(crate) test_script_editor: Entity<CodeEditorState>,
    pub(crate) bearer_token_input: Entity<InputState>,
    pub(crate) basic_user_input: Entity<InputState>,
    pub(crate) basic_pass_input: Entity<InputState>,
    pub(crate) apikey_key_input: Entity<InputState>,
    pub(crate) apikey_val_input: Entity<InputState>,
    pub(crate) executor: Arc<ReqwestExecutor>,
    pub(crate) tokio_rt: Arc<tokio::runtime::Runtime>,
    pub(crate) last_synced_request_id: Option<String>,
    pub(crate) last_synced_request_version: usize,
    pub(crate) last_synced_body_version: usize,
    pub(crate) last_synced_response_id: Option<String>,
    pub(crate) last_synced_response_view_mode: Option<ResponseViewMode>,
    pub(crate) last_synced_example_name: Option<String>,
    pub(crate) _subscriptions: Vec<Subscription>,
}

impl AppView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let workspace_state = cx.new(|_| WorkspaceDataState::new());
        let (ws_dir, ws_store, ws_name) = {
            let ws = workspace_state.read(cx);
            (
                ws.workspace_path.clone(),
                ws.store.clone(),
                ws.active_workspace.as_ref().map(|w| w.name.clone()).unwrap_or_else(|| "Workspace".to_string()),
            )
        };
        let persistence_state = cx.new(|_| PersistenceState::new(ws_store.clone(), ws_dir.clone()));
        let initial_active_env = persistence_state.read(cx).local_state.active_environment.clone();
        let env_state = cx.new(|_| EnvironmentState::new(ws_store.clone(), ws_name, initial_active_env));

        persistence_state.update(cx, |p, cx| {
            p.start_watcher(workspace_state.clone(), env_state.clone(), cx);
        });

        let sidebar_state = cx.new(|_| SidebarState::new());
        let modal_state = cx.new(|_| ModalState::new());
        let git_state = cx.new(|_| GitState::new(ws_dir));
        let editor_state = cx.new(|_| EditorState::new());
        let response_state = cx.new(|_| ResponseState::new());
        let toast_state = cx.new(|_| ToastState::new());

        let url_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Enter request URL (e.g. https://httpbin.org/get)")
        });

        let highlighter_factory: std::rc::Rc<
            dyn Fn(&str) -> Option<Box<dyn gpui_component::input::InputHighlighter>>,
        > = std::rc::Rc::new(|lang: &str| match lang {
            "json" => Some(Box::new(
                crate::components::body_editor::JsonHighlighter::new(),
            )),
            "xml" => Some(Box::new(
                crate::components::body_editor::XmlHighlighter::new(),
            )),
            "javascript" | "js" => Some(Box::new(JavaScriptHighlighter::new())),
            _ => None,
        });

        let json_editor = cx.new(|cx| {
            let mut state = CodeEditorState::new(window, cx);
            state.set_highlighter_factory(highlighter_factory.clone(), cx);
            state.lsp_mut().completion_provider = Some(std::rc::Rc::new(
                crate::components::env_autocomplete::EnvCompletionProvider::new(
                    env_state.clone(),
                ),
            ));
            state.language("json").line_number(true).folding(true)
        });
        let xml_editor = cx.new(|cx| {
            let mut state = CodeEditorState::new(window, cx);
            state.set_highlighter_factory(highlighter_factory.clone(), cx);
            state.lsp_mut().completion_provider = Some(std::rc::Rc::new(
                crate::components::env_autocomplete::EnvCompletionProvider::new(
                    env_state.clone(),
                ),
            ));
            state.language("xml").line_number(true).folding(true)
        });
        let raw_textarea = cx.new(|cx| TextareaState::new(window, cx));
        let response_editor = cx.new(|cx| {
            let mut state = CodeEditorState::new(window, cx);
            state.set_highlighter_factory(highlighter_factory.clone(), cx);
            state
                .language("json")
                .line_number(true)
                .folding(true)
                .soft_wrap(false)
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
                ScriptCompletionProvider::with_environment(
                    ScriptEditorKind::PreRequest,
                    env_state.clone(),
                ),
            ));
            state
                .language("javascript")
                .line_number(true)
                .folding(true)
        });
        let test_script_editor = cx.new(|cx| {
            let mut state = CodeEditorState::new(window, cx);
            state.set_highlighter_factory(highlighter_factory.clone(), cx);
            state.lsp_mut().completion_provider = Some(std::rc::Rc::new(
                ScriptCompletionProvider::with_environment(
                    ScriptEditorKind::Tests,
                    env_state.clone(),
                ),
            ));
            state
                .language("javascript")
                .line_number(true)
                .folding(true)
        });

        let bearer_token_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("Enter Bearer token..."));
        let basic_user_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("Username..."));
        let basic_pass_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("Password..."));
        let apikey_key_input = cx.new(|cx| InputState::new(window, cx).placeholder("X-API-Key"));
        let apikey_val_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("API key value..."));

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
            let per_read = persistence_state.read(cx);
            let req = per_read
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
                let env = env_state.read(cx);
                crate::components::env_autocomplete::build_tokenized_content(&req_url, env)
            };
            url_input.update(cx, |inp, cx| {
                inp.set_value(tokenized_url, window, cx);
            });

            if let RequestBody::Json(json_content) = &req.body {
                json_editor.update(cx, |txt, cx| {
                    txt.set_value(json_content.clone(), window, cx);
                });
            } else if let RequestBody::Raw {
                content,
                content_type,
            } = &req.body
            {
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

            let env_for_auth = env_state.clone();
            match &req.auth {
                Auth::Bearer { token } => {
                    let env_m = env_for_auth.clone();
                    bearer_token_input.update(cx, |inp, cx| {
                        let env = env_m.read(cx);
                        inp.set_value(
                            crate::components::env_autocomplete::build_tokenized_content(token, env),
                            window,
                            cx,
                        );
                    });
                }
                Auth::Basic { username, password } => {
                    let env_u = env_for_auth.clone();
                    let env_p = env_for_auth.clone();
                    basic_user_input.update(cx, |inp, cx| {
                        let env = env_u.read(cx);
                        inp.set_value(
                            crate::components::env_autocomplete::build_tokenized_content(
                                username, env,
                            ),
                            window,
                            cx,
                        );
                    });
                    basic_pass_input.update(cx, |inp, cx| {
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
                    let env_k = env_for_auth.clone();
                    let env_v = env_for_auth.clone();
                    apikey_key_input.update(cx, |inp, cx| {
                        let env = env_k.read(cx);
                        inp.set_value(
                            crate::components::env_autocomplete::build_tokenized_content(key, env),
                            window,
                            cx,
                        );
                    });
                    apikey_val_input.update(cx, |inp, cx| {
                        let env = env_v.read(cx);
                        inp.set_value(
                            crate::components::env_autocomplete::build_tokenized_content(value, env),
                            window,
                            cx,
                        );
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
        let _subscriptions = setup_app_subscriptions(
            SubscriptionTargets {
                workspace_state: &workspace_state,
                env_state: &env_state,
                persistence_state: &persistence_state,
                modal_state: &modal_state,
                sidebar_state: &sidebar_state,
                git_state: &git_state,
                editor_state: &editor_state,
                response_state: &response_state,
                toast_state: &toast_state,
                url_input: &url_input,
                json_editor: &json_editor,
                xml_editor: &xml_editor,
                raw_textarea: &raw_textarea,
                bearer_token_input: &bearer_token_input,
                basic_user_input: &basic_user_input,
                basic_pass_input: &basic_pass_input,
                apikey_key_input: &apikey_key_input,
                apikey_val_input: &apikey_val_input,
                pre_script_editor: &pre_script_editor,
                test_script_editor: &test_script_editor,
            },
            initial_req.as_ref(),
            window,
            cx,
        );

        let last_synced_request_id = initial_req.as_ref().map(|r| r.id.clone());
        let last_synced_body_version = editor_state.read(cx).body_version;

        if git_state.read(cx).tracked_warning {
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
            env_state,
            persistence_state,
            sidebar_state,
            modal_state,
            git_state,
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
}

impl Render for AppView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = theme();

        let sidebar_collapsed = self.sidebar_state.read(cx).sidebar_collapsed;
        let sidebar_width = self.sidebar_state.read(cx).sidebar_width;
        let is_vertical_split = self.modal_state.read(cx).split_layout_vertical;

        // Synchronize active request fields into InputState / TextareaState when switched or body version changed
        self.sync_active_request_editors(window, cx);

        // Synchronize response_editor with ResponseState's formatted_body
        self.sync_response_editor(window, cx);

        // Synchronize request_url_textarea and request_body_editor with RequestSent
        self.sync_sent_request_preview(window, cx);

        let ws_entity = self.workspace_state.clone();
        let sidebar_entity = self.sidebar_state.clone();
        let modal_entity = self.modal_state.clone();
        let git_entity = self.git_state.clone();
        let ed_entity = self.editor_state.clone();
        let resp_entity = self.response_state.clone();
        let toast_entity = self.toast_state.clone();
        let app_view_entity = cx.entity().clone();

        let editor_pane = self.render_editor_pane(window, cx);

        let response_pane = ResponseViewer::new(
            ws_entity.clone(),
            ed_entity.clone(),
            resp_entity.clone(),
            self.response_editor.clone(),
            self.request_url_textarea.clone(),
            self.request_body_editor.clone(),
        );

        let split_pane = if is_vertical_split {
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
        };

        let main_content = div().size_full().child(split_pane);

        let workspace_content = if sidebar_collapsed {
            main_content.into_any_element()
        } else {
            h_resizable("workspace-sidebar-split")
                .child(
                    resizable_panel()
                        .size(sidebar_width)
                        .size_range(px(200.)..px(480.))
                        .child(SidebarTree::new(
                            ws_entity.clone(),
                            self.persistence_state.clone(),
                            sidebar_entity.clone(),
                            modal_entity.clone(),
                            git_entity.clone(),
                            ed_entity.clone(),
                            resp_entity.clone(),
                            toast_entity.clone(),
                        )),
                )
                .child(resizable_panel().child(main_content))
                .on_resize({
                    let sb_entity = sidebar_entity.clone();
                    move |state, _window, cx| {
                        let new_size = state.read(cx).sizes().first().copied();
                        if let Some(size) = new_size {
                            sb_entity.update(cx, |sb, cx| {
                                sb.set_sidebar_width(size, cx);
                            });
                        }
                    }
                })
                .into_any_element()
        };

        let root = div()
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
                let modal_c = modal_entity.clone();
                move |_: &ToggleCommandPalette, _, cx| {
                    modal_c.update(cx, |state, cx| {
                        state.toggle_command_palette(cx);
                    });
                }
            })
            .on_action({
                let sb_c = sidebar_entity.clone();
                move |_: &ToggleSidebar, _, cx| {
                    sb_c.update(cx, |state, cx| {
                        state.toggle_sidebar(cx);
                    });
                }
            })
            .on_action({
                let modal_c = modal_entity.clone();
                move |_: &ToggleSettings, _, cx| {
                    modal_c.update(cx, |state, cx| {
                        state.toggle_settings_modal(cx);
                    });
                }
            })
            .on_action({
                let git_c = git_entity.clone();
                move |_: &ToggleGitModal, _, cx| {
                    git_c.update(cx, |state, cx| {
                        state.toggle_modal(cx);
                    });
                }
            })
            .on_action({
                let modal_c = modal_entity.clone();
                move |_: &OpenImportModal, _, cx| {
                    modal_c.update(cx, |state, cx| {
                        state.open_import_modal(None, cx);
                    });
                }
            })
            .on_action({
                let sb_c = sidebar_entity.clone();
                let ws_c = ws_entity.clone();
                let ed_c = ed_entity.clone();
                move |_: &NavigateBack, _, cx| {
                    if let Some(req_id) = sb_c.update(cx, |sb, _| sb.navigate_back()) {
                        let maybe_req = ws_c.read(cx).find_request(&req_id).cloned();
                        if let Some(req) = maybe_req {
                            ed_c.update(cx, |ed, cx| {
                                ed.set_active_request(req, false, cx);
                            });
                        }
                    }
                }
            })
            .on_action({
                let sb_c = sidebar_entity.clone();
                let ws_c = ws_entity.clone();
                let ed_c = ed_entity.clone();
                move |_: &NavigateForward, _, cx| {
                    if let Some(req_id) = sb_c.update(cx, |sb, _| sb.navigate_forward()) {
                        let maybe_req = ws_c.read(cx).find_request(&req_id).cloned();
                        if let Some(req) = maybe_req {
                            ed_c.update(cx, |ed, cx| {
                                ed.set_active_request(req, false, cx);
                            });
                        }
                    }
                }
            })
            .on_action({
                let modal_c = modal_entity.clone();
                let git_c = git_entity.clone();
                move |_: &DismissModal, _, cx| {
                    let git_open = git_c.read(cx).show_modal;
                    if git_open {
                        git_c.update(cx, |g, cx| g.close_modal(cx));
                    } else {
                        modal_c.update(cx, |m, cx| {
                            m.dismiss_top_modal(cx);
                        });
                    }
                }
            })
            .on_key_down({
                let modal_c = modal_entity.clone();
                let git_c = git_entity.clone();
                move |event, _, cx| {
                    if event.keystroke.key.as_str() == "escape" {
                        let git_open = git_c.read(cx).show_modal;
                        if git_open {
                            git_c.update(cx, |g, cx| g.close_modal(cx));
                        } else {
                            modal_c.update(cx, |m, cx| {
                                m.dismiss_top_modal(cx);
                            });
                        }
                    }
                }
            })
            // Custom Title Bar
            .child(AppTitleBar::new(
                ws_entity.clone(),
                self.env_state.clone(),
                modal_entity.clone(),
                sidebar_entity.clone(),
                ed_entity.clone(),
                resp_entity.clone(),
            ))
            // Main workspace content area
            .child(
                div()
                    .flex_1()
                    .w_full()
                    .overflow_hidden()
                    .child(workspace_content),
            );

        let send_view = app_view_entity.clone();
        attach_modal_overlays(
            root,
            &ws_entity,
            &self.env_state,
            &self.persistence_state,
            &modal_entity,
            &sidebar_entity,
            &git_entity,
            &ed_entity,
            &toast_entity,
            move |_, cx| {
                send_view.update(cx, |view, cx| {
                    view.send_current_request(cx);
                });
            },
            cx,
        )
    }
}

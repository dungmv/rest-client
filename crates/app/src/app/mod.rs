pub mod actions;
pub mod auth_input;
pub mod auth_tab;
pub mod composer_inputs;
pub mod inspector_displays;
pub mod modals;
pub mod request_composer;
pub mod subscriptions;
pub mod sync;

pub use actions::*;
pub use composer_inputs::*;
pub use inspector_displays::*;
pub use modals::*;
pub use subscriptions::*;

use crate::components::*;
use crate::state::*;
use crate::theme::theme;
use gpui::*;
use gpui_component::resizable::{h_resizable, resizable_panel, v_resizable};
use restclient_core::http::ReqwestExecutor;
use std::sync::Arc;

pub struct AppView {
    pub(crate) workspace_state: Entity<WorkspaceDataState>,
    pub(crate) env_state: Entity<EnvironmentState>,
    pub(crate) persistence_state: Entity<PersistenceState>,
    pub(crate) sidebar_state: Entity<SidebarState>,
    pub(crate) modal_state: Entity<ModalState>,
    pub(crate) git_state: Entity<GitState>,
    pub(crate) session_state: Entity<RequestSessionState>,
    pub(crate) response_state: Entity<ResponseState>,
    pub(crate) toast_state: Entity<ToastState>,
    pub(crate) composer_inputs: ComposerInputs,
    pub(crate) inspector_displays: InspectorDisplays,
    pub(crate) executor: Arc<ReqwestExecutor>,
    pub(crate) tokio_rt: Arc<tokio::runtime::Runtime>,
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
        let session_state = cx.new(|_| RequestSessionState::new());
        let response_state = cx.new(|_| ResponseState::new());
        let toast_state = cx.new(|_| ToastState::new());

        type HighlighterFactory = std::rc::Rc<
            dyn Fn(&str) -> Option<Box<dyn gpui_component::input::InputHighlighter>>,
        >;

        let highlighter_factory: HighlighterFactory = std::rc::Rc::new(|lang: &str| match lang {
            "json" => Some(Box::new(
                crate::components::body_editor::JsonHighlighter::new(),
            )),
            "xml" => Some(Box::new(
                crate::components::body_editor::XmlHighlighter::new(),
            )),
            "javascript" | "js" => Some(Box::new(JavaScriptHighlighter::new())),
            _ => None,
        });

        let mut composer_inputs = ComposerInputs::new(
            window,
            env_state.clone(),
            highlighter_factory.clone(),
            cx,
        );
        let mut inspector_displays = InspectorDisplays::new(window, highlighter_factory, cx);

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
            session_state.update(cx, |ed, cx| {
                ed.set_active_request_id(&req.id, cx);
            });
            composer_inputs.sync_active_request_editors(
                &session_state,
                &workspace_state,
                &env_state,
                window,
                cx,
            );
            let req_id = req.id.clone();
            response_state.update(cx, |resp, cx| {
                resp.load_for_request(&req_id, ws_store.as_ref(), cx);
            });
            inspector_displays.sync_response_editor(&response_state, window, cx);
            inspector_displays.sync_sent_request_preview(&response_state, window, cx);
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
                editor_state: &session_state,
                response_state: &response_state,
                toast_state: &toast_state,
                composer_inputs: &composer_inputs,
            },
            initial_req.as_ref(),
            window,
            cx,
        );

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
            session_state,
            response_state,
            toast_state,
            composer_inputs,
            inspector_displays,
            executor,
            tokio_rt,
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

        let ws_entity = self.workspace_state.clone();
        let sidebar_entity = self.sidebar_state.clone();
        let modal_entity = self.modal_state.clone();
        let git_entity = self.git_state.clone();
        let session_entity = self.session_state.clone();
        let resp_entity = self.response_state.clone();
        let toast_entity = self.toast_state.clone();
        let app_view_entity = cx.entity().clone();

        let composer_pane = self.render_request_composer(window, cx);

        let response_pane = ResponseInspector::new(
            ws_entity.clone(),
            session_entity.clone(),
            resp_entity.clone(),
            self.inspector_displays.response_editor.clone(),
            self.inspector_displays.request_url_textarea.clone(),
            self.inspector_displays.request_body_editor.clone(),
        );

        let split_pane = if is_vertical_split {
            div().size_full().child(
                v_resizable("inner-vertical-split")
                    .child(resizable_panel().child(composer_pane))
                    .child(resizable_panel().child(response_pane)),
            )
        } else {
            div().size_full().child(
                h_resizable("inner-horizontal-split")
                    .child(resizable_panel().child(composer_pane))
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
                            session_entity.clone(),
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
                let session_c = session_entity.clone();
                move |_: &NavigateBack, _, cx| {
                    if let Some(req_id) = sb_c.update(cx, |sb, _| sb.navigate_back()) {
                        session_c.update(cx, |ed, cx| {
                            ed.set_active_request_id(&req_id, cx);
                        });
                    }
                }
            })
            .on_action({
                let sb_c = sidebar_entity.clone();
                let session_c = session_entity.clone();
                move |_: &NavigateForward, _, cx| {
                    if let Some(req_id) = sb_c.update(cx, |sb, _| sb.navigate_forward()) {
                        session_c.update(cx, |ed, cx| {
                            ed.set_active_request_id(&req_id, cx);
                        });
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
                session_entity.clone(),
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
            &session_entity,
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

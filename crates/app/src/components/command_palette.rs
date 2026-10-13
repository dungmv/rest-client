use crate::components::status_badge::MethodBadge;
use crate::state::{EditorState, WorkspaceState};
use crate::theme::theme;
use gpui::{
    div, px, App, AppContext as _, Entity, InteractiveElement as _, IntoElement, ParentElement,
    RenderOnce, StatefulInteractiveElement as _, Styled, Window,
};
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::scroll::ScrollableElement as _;

#[derive(IntoElement)]
pub struct CommandPalette {
    workspace_state: Entity<WorkspaceState>,
    editor_state: Entity<EditorState>,
    on_send: std::rc::Rc<dyn Fn(&mut Window, &mut App)>,
}

impl CommandPalette {
    pub fn new(
        workspace_state: Entity<WorkspaceState>,
        editor_state: Entity<EditorState>,
        on_send: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            workspace_state,
            editor_state,
            on_send: std::rc::Rc::new(on_send),
        }
    }
}

impl RenderOnce for CommandPalette {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = theme();
        let ws = self.workspace_state.read(cx);
        let requests = ws.requests.clone();
        let envs = ws.environments.clone();

        let ws_clone = self.workspace_state.clone();
        let ed_clone = self.editor_state.clone();
        let on_send = self.on_send.clone();

        let cmd_holder = window.use_keyed_state(
            gpui::SharedString::from("cmd-palette-search-holder"),
            cx,
            |window, cx| {
                let inp = cx.new(|cx| {
                    InputState::new(window, cx)
                        .placeholder("Type a command, request, or environment...")
                });
                let sub = cx.subscribe_in(&inp, window, move |_, _, event, _, cx| {
                    if let InputEvent::Change = event {
                        cx.refresh_windows();
                    }
                });
                (inp, sub)
            },
        );

        let search_input = cmd_holder.read(cx).0.clone();
        let q = search_input.read(cx).value().to_lowercase();

        div()
            .absolute()
            .inset_0()
            .bg(gpui::Hsla { a: 0.6, ..t.bg_app })
            .flex()
            .items_start()
            .justify_center()
            .pt(px(100.))
            .child(
                div()
                    .w(px(560.))
                    .max_h(px(400.))
                    .rounded(px(8.))
                    .bg(t.bg_card)
                    .border_1()
                    .border_color(t.border)
                    .shadow_2xl()
                    .flex()
                    .flex_col()
                    // Search bar
                    .child(
                        div()
                            .px(px(12.))
                            .py(px(8.))
                            .border_b_1()
                            .border_color(t.border)
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .child("🔍")
                            .child(
                                div()
                                    .flex_1()
                                    .child(
                                        Input::new(&search_input)
                                            .id("cmd-palette-input")
                                            .appearance(false)
                                            .bordered(false)
                                            .focus_bordered(false)
                                            .cleanable(true),
                                    ),
                            )
                            .child({
                                let ws_s = ws_clone.clone();
                                div()
                                    .id("close-cmd-pal")
                                    .cursor_pointer()
                                    .text_xs()
                                    .text_color(t.text_muted)
                                    .hover(|s| s.text_color(t.text_primary))
                                    .on_click(move |_, _, cx| {
                                        ws_s.update(cx, |state, cx| {
                                            state.toggle_command_palette(cx);
                                        });
                                    })
                                    .child("ESC")
                            }),
                    )
                    // Actions & Items List
                    .child(
                        div()
                            .flex_1()
                            .overflow_y_scrollbar()
                            .p(px(6.))
                            .flex()
                            .flex_col()
                            .gap(px(4.))
                            // Action: Send active request
                            .child({
                                let ws_s = ws_clone.clone();
                                let on_send_cb = on_send.clone();
                                div()
                                    .id("cmd-send-req")
                                    .px(px(8.))
                                    .py(px(6.))
                                    .rounded(px(4.))
                                    .hover(|s| s.bg(t.bg_hover))
                                    .cursor_pointer()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .on_click(move |_, window, cx| {
                                        ws_s.update(cx, |state, cx| {
                                            state.toggle_command_palette(cx);
                                        });
                                        on_send_cb(window, cx);
                                    })
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(8.))
                                            .child("✈")
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                                    .text_color(t.text_primary)
                                                    .child("Send Active Request"),
                                            ),
                                    )
                                    .child(div().text_xs().text_color(t.text_muted).child("Ctrl+Enter"))
                            })
                            // List Requests
                            .child(
                                div()
                                    .px(px(8.))
                                    .pt(px(4.))
                                    .text_xs()
                                    .font_weight(gpui::FontWeight::BOLD)
                                    .text_color(t.text_muted)
                                    .child("SWITCH REQUEST"),
                            )
                            .children(
                                requests
                                    .into_iter()
                                    .filter(|r| {
                                        q.is_empty()
                                            || r.name.to_lowercase().contains(&q)
                                            || r.url.to_lowercase().contains(&q)
                                    })
                                    .map(|req| {
                                        let ws_s = ws_clone.clone();
                                        let ed_s = ed_clone.clone();
                                        let r = req.clone();

                                        div()
                                            .id(format!("cmd-req-{}", req.id))
                                            .px(px(8.))
                                            .py(px(6.))
                                            .rounded(px(4.))
                                            .hover(|s| s.bg(t.bg_hover))
                                            .cursor_pointer()
                                            .flex()
                                            .items_center()
                                            .gap(px(8.))
                                            .on_click(move |_, _, cx| {
                                                ed_s.update(cx, |state, cx| {
                                                    state.set_active_request(r.clone(), true, cx);
                                                });
                                                ws_s.update(cx, |state, cx| {
                                                    state.toggle_command_palette(cx);
                                                });
                                            })
                                            .child(MethodBadge::new(req.method))
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .font_family(".SystemMonoFont")
                                                    .text_color(t.text_primary)
                                                    .child(req.name.clone()),
                                            )
                                    }),
                            )
                            // Switch Environment
                            .child(
                                div()
                                    .px(px(8.))
                                    .pt(px(6.))
                                    .text_xs()
                                    .font_weight(gpui::FontWeight::BOLD)
                                    .text_color(t.text_muted)
                                    .child("SWITCH ENVIRONMENT"),
                            )
                            .children(
                                envs.into_iter()
                                    .filter(|e| q.is_empty() || e.name.to_lowercase().contains(&q))
                                    .map(|env| {
                                        let ws_s = ws_clone.clone();
                                        let env_id = env.id.clone();

                                        div()
                                            .id(format!("cmd-env-{}", env.id))
                                            .px(px(8.))
                                            .py(px(6.))
                                            .rounded(px(4.))
                                            .hover(|s| s.bg(t.bg_hover))
                                            .cursor_pointer()
                                            .flex()
                                            .items_center()
                                            .gap(px(8.))
                                            .on_click(move |_, _, cx| {
                                                ws_s.update(cx, |state, cx| {
                                                    state.set_active_environment(
                                                        Some(env_id.clone()),
                                                        cx,
                                                    );
                                                    state.toggle_command_palette(cx);
                                                });
                                            })
                                            .child("●")
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(t.text_primary)
                                                    .child(format!("Switch to: {}", env.name)),
                                            )
                                    }),
                            ),
                    ),
            )
    }
}

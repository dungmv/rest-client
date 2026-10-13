use crate::components::status_badge::MethodBadge;
use crate::state::{EditorState, WorkspaceState};
use crate::theme::theme;
use gpui::prelude::FluentBuilder as _;
use gpui::{
    div, px, AnyElement, App, Entity, InteractiveElement as _, IntoElement, ParentElement,
    RenderOnce, StatefulInteractiveElement as _, Styled, Window,
};
use gpui_component::input::{Input, InputState};
use restclient_core::interpolation::scan_variables;
use restclient_core::models::HttpMethod;

#[derive(IntoElement)]
pub struct RequestBar {
    workspace_state: Entity<WorkspaceState>,
    editor_state: Entity<EditorState>,
    url_input: Entity<InputState>,
    on_send: std::rc::Rc<dyn Fn(&mut Window, &mut App)>,
}

impl RequestBar {
    pub fn new(
        workspace_state: Entity<WorkspaceState>,
        editor_state: Entity<EditorState>,
        url_input: Entity<InputState>,
        on_send: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            workspace_state,
            editor_state,
            url_input,
            on_send: std::rc::Rc::new(on_send),
        }
    }
}

impl RenderOnce for RequestBar {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = theme();
        let ws = self.workspace_state.read(cx);
        let ed = self.editor_state.read(cx);

        let active_req = ed.active_request.clone();
        let method = active_req.as_ref().map(|r| r.method).unwrap_or(HttpMethod::Get);
        let url = self.url_input.read(cx).value().to_string();
        let is_sending = ed.is_sending;

        let ed_clone = self.editor_state.clone();
        let on_send = self.on_send.clone();

        // Scan variables in URL for badges in suffix
        let active_env = ws.active_environment();
        let var_spans = scan_variables(&url, active_env);

        div()
            .h(px(40.))
            .w_full()
            .px(px(8.))
            .flex()
            .items_center()
            .gap(px(8.))
            .border_b_1()
            .border_color(t.border)
            // Method dropdown trigger
            .child({
                let ed_s = ed_clone.clone();
                div()
                    .id("method-dropdown-btn")
                    .px(px(8.))
                    .py(px(4.))
                    .rounded(px(4.))
                    .cursor_pointer()
                    .hover(|s| s.bg(t.bg_hover))
                    .flex()
                    .items_center()
                    .gap(px(4.))
                    .on_click(move |_, _, cx| {
                        ed_s.update(cx, |state, cx| {
                            state.toggle_method_dropdown(cx);
                        });
                    })
                    .child(MethodBadge::new(method))
                    .child(
                        div()
                            .text_xs()
                            .text_color(t.text_muted)
                            .child("▾"),
                    )
            })
            // URL Input Box with live editing and variable status badges
            .child(
                div()
                    .flex_1()
                    .h(px(32.))
                    .flex()
                    .items_center()
                    .child({
                        let input_el = Input::new(&self.url_input)
                            .id("url-input-field")
                            .cleanable(true);

                        if var_spans.is_empty() {
                            input_el.into_any_element()
                        } else {
                            input_el
                                .suffix(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap(px(4.))
                                        .px(px(4.))
                                        .children(var_spans.into_iter().map(|span| {
                                            let color = if span.is_resolved {
                                                t.var_resolved
                                            } else {
                                                t.var_unresolved
                                            };
                                            div()
                                                .px(px(4.))
                                                .py(px(1.))
                                                .rounded(px(3.))
                                                .bg(t.bg_card)
                                                .border_1()
                                                .border_color(color)
                                                .text_xs()
                                                .font_family(".SystemMonoFont")
                                                .text_color(color)
                                                .child(format!("{{{{{}}}}}", span.name))
                                        })),
                                )
                                .into_any_element()
                        }
                    }),
            )
            // Send / Cancel Button with keyboard shortcut tooltip
            .child({
                let ed_s = ed_clone.clone();
                let on_send_cb = on_send.clone();

                div()
                    .id("btn-send-or-cancel")
                    .px(px(14.))
                    .h(px(32.))
                    .rounded(px(6.))
                    .bg(if is_sending { t.status_5xx } else { t.primary })
                    .hover(|s| {
                        if is_sending {
                            s.bg(t.status_5xx)
                        } else {
                            s.bg(t.primary_hover)
                        }
                    })
                    .cursor_pointer()
                    .flex()
                    .items_center()
                    .justify_center()
                    .gap(px(6.))
                    .text_xs()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(t.primary_foreground)
                    .on_click(move |_, window, cx| {
                        if is_sending {
                            ed_s.update(cx, |state, cx| {
                                if let Some(tx) = state.cancel_tx.take() {
                                    let _ = tx.send(());
                                }
                                state.stop_sending(cx);
                            });
                        } else {
                            on_send_cb(window, cx);
                        }
                    })
                    .child(if is_sending { "✕ Cancel" } else { "Send ✈" })
            })
    }
}

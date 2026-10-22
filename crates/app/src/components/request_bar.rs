use crate::components::env_autocomplete::{
    build_tokenized_content, detect_unclosed_env_var, get_env_completions, get_variable_details,
    get_variable_kind_and_color,
};
use crate::components::status_badge::MethodBadge;
use crate::state::{EnvironmentState, ModalState, RequestSessionState};
use crate::theme::theme;
use gpui::prelude::FluentBuilder as _;
use gpui::{
    deferred, div, px, App, AppContext as _, Entity, FontWeight,
    InteractiveElement as _, IntoElement, ParentElement, RenderOnce,
    StatefulInteractiveElement as _, Styled, Window,
};
use gpui_component::input::{Input, InputState};
use gpui_component::scroll::ScrollableElement as _;
use restclient_core::interpolation::scan_variables;
use restclient_core::models::HttpMethod;

#[allow(clippy::type_complexity)]
#[derive(IntoElement)]
pub struct RequestBar {
    workspace_state: Entity<crate::state::WorkspaceDataState>,
    env_state: Entity<EnvironmentState>,
    modal_state: Entity<ModalState>,
    editor_state: Entity<RequestSessionState>,
    url_input: Entity<InputState>,
    on_send: std::rc::Rc<dyn Fn(&mut Window, &mut App)>,
}

impl RequestBar {
    pub fn new(
        workspace_state: Entity<crate::state::WorkspaceDataState>,
        env_state: Entity<EnvironmentState>,
        modal_state: Entity<ModalState>,
        editor_state: Entity<RequestSessionState>,
        url_input: Entity<InputState>,
        on_send: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            workspace_state,
            env_state,
            modal_state,
            editor_state,
            url_input,
            on_send: std::rc::Rc::new(on_send),
        }
    }
}

impl RenderOnce for RequestBar {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = theme();
        let (method, is_sending) = {
            let ed = self.editor_state.read(cx);
            let active_id = ed.active_request_id();
            let method = self
                .workspace_state
                .read(cx)
                .find_request(active_id)
                .map(|r| r.method)
                .unwrap_or(HttpMethod::Get);
            (method, ed.is_sending)
        };
        let url = self.url_input.read(cx).value().to_string();

        let ws_clone = self.env_state.clone();
        let modal_clone = self.modal_state.clone();
        let ed_clone = self.editor_state.clone();
        let url_input_clone = self.url_input.clone();
        let on_send = self.on_send.clone();

        // Scan variables in URL and build completions only if URL contains {{
        let (var_map, completion_candidates, unclosed) = if url.contains("{{") {
            let ws = self.env_state.read(cx);
            let effective_env = ws.effective_environment();
            let var_spans = scan_variables(&url, Some(&effective_env));

            let map: std::collections::HashMap<String, (String, String, bool)> = var_spans
                .iter()
                .map(|s| {
                    let (val, scope, is_res) = get_variable_details(ws, &s.name);
                    (s.name.clone(), (val, scope, is_res))
                })
                .collect();

            let unclosed = detect_unclosed_env_var(&url);
            let candidates = unclosed
                .as_ref()
                .map(|(_, _, q)| get_env_completions(ws, q))
                .unwrap_or_default();

            (map, candidates, unclosed)
        } else {
            (std::collections::HashMap::new(), Vec::new(), None)
        };

        let hover_token_state = window.use_keyed_state(
            gpui::SharedString::from("url-inline-token-hover"),
            cx,
            |_window, _cx| None::<(String, gpui::Bounds<gpui::Pixels>)>,
        );
        let hovered_token = hover_token_state.read(cx).clone();

        let show_autocomplete = !completion_candidates.is_empty();

        div()
            .relative()
            .h(px(44.))
            .w_full()
            .px(px(8.))
            .py(px(4.))
            .flex()
            .items_center()
            .gap(px(8.))
            .bg(t.bg_editor)
            .border_b_1()
            .border_color(t.border)
            // Unified Request Input Bar
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.))
                    .h(px(34.))
                    .px(px(4.))
                    .rounded(px(6.))
                    .bg(t.bg_input)
                    .border_1()
                    .border_color(t.border)
                    .hover(|s| s.border_color(t.border_focus))
                    .flex()
                    .items_center()
                    .gap(px(2.))
                    // Method dropdown trigger
                    .child({
                        let ed_s = ed_clone.clone();
                        div()
                            .id("method-dropdown-btn")
                            .pl(px(6.))
                            .pr(px(3.))
                            .py(px(3.))
                            .rounded(px(4.))
                            .cursor_pointer()
                            .hover(|s| s.bg(t.bg_hover))
                            .flex()
                            .items_center()
                            .gap(px(3.))
                            .on_click(move |_, _, cx| {
                                ed_s.update(cx, |state, cx| {
                                    state.toggle_method_dropdown(cx);
                                });
                            })
                            .child(MethodBadge::new(method).font_weight(gpui::FontWeight::SEMIBOLD))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(t.text_muted)
                                    .child("▾"),
                            )
                    })
                    // Subtle vertical separator
                    .child(
                        div()
                            .w(px(1.))
                            .h(px(16.))
                            .bg(t.border)
                            .mx(px(2.)),
                    )
                    // URL Input Box - with inline tokens for environment variables
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.))
                            .h_full()
                            .relative()
                            .flex()
                            .items_center()
                            .pl(px(2.))
                            .child({
                                let ws_env = ws_clone.clone();
                                let modal_click = modal_clone.clone();
                                let h_token = hover_token_state.clone();

                                Input::new(&self.url_input)
                                    .id("url-input-field")
                                    .appearance(false)
                                    .bordered(false)
                                    .focus_bordered(false)
                                    .cleanable(false)
                                    .px(px(2.))
                                    .font_family(".SystemMonoFont")
                                    .text_xs()
                                    .token(move |token_ctx, _window, cx| {
                                        let var_name = token_ctx.token().id().to_string();
                                        let ws = ws_env.read(cx);
                                        let (_val, scope, is_res) = get_variable_details(ws, &var_name);
                                        let (_kind, color, bg_tint) = get_variable_kind_and_color(&scope, is_res);

                                        div()
                                            .id(gpui::SharedString::from(format!("inline-token-{}", var_name)))
                                            .flex()
                                            .items_center()
                                            .px(px(6.))
                                            .py(px(1.))
                                            .rounded(px(4.))
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
                                            if is_h {
                                                *h = Some((name, bounds));
                                            } else {
                                                *h = None;
                                            }
                                            cx.notify();
                                        });
                                    })
                                    .on_token_click(move |_, _window, cx| {
                                        modal_click.update(cx, |state, cx| {
                                            state.toggle_env_modal(cx);
                                        });
                                    })
                            }),
                    ),
            )
            // Tooltip card shown ON HOVER over inline variable token
            .when_some(hovered_token, |this, (var_name, _bounds)| {
                let (val, scope, is_res) = var_map
                    .get(&var_name)
                    .cloned()
                    .unwrap_or_else(|| ("".to_string(), "".to_string(), false));
                let (_kind, color, bg_tint) = get_variable_kind_and_color(&scope, is_res);

                this.child(
                    deferred(
                        div()
                            .absolute()
                            .top(px(42.))
                            .left(px(80.))
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
                                            .px(px(6.))
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
            // Autocomplete Dropdown popup when user types {{
            .when(show_autocomplete, |this| {
                let (start_idx, end_idx) = unclosed.as_ref().map(|(pos, end, _)| (*pos, *end)).unwrap_or((0, 0));
                let current_url = url.clone();
                let inp_target = url_input_clone.clone();
                let ed_target = ed_clone.clone();
                let ws_data_target = self.workspace_state.clone();
                let req_id_str = ed_clone.read(cx).active_request_id().to_string();

                this.child(
                    deferred(
                        div()
                            .absolute()
                            .top(px(42.))
                            .left(px(80.))
                            .w(px(380.))
                            .max_h(px(240.))
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
                                    .py(px(6.))
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
                                        let url_c = current_url.clone();
                                        let inp_c = inp_target.clone();
                                        let ed_c = ed_target.clone();
                                        let ws_c = ws_clone.clone();
                                        let ws_data_c = ws_data_target.clone();
                                        let req_id_c = req_id_str.clone();
                                        let (_kind, cand_color, cand_bg_tint) = get_variable_kind_and_color(&cand_scope, true);

                                        div()
                                            .id(gpui::SharedString::from(format!("cand-{}", cand.name)))
                                            .cursor_pointer()
                                            .px(px(8.))
                                            .py(px(5.))
                                            .rounded(px(4.))
                                            .hover(|s| s.bg(t.bg_hover))
                                            .flex()
                                            .items_center()
                                            .justify_between()
                                            .on_click(move |_, window, cx| {
                                                let mut new_u = url_c.clone();
                                                if start_idx <= new_u.len() && end_idx <= new_u.len() && start_idx <= end_idx {
                                                    new_u.replace_range(start_idx..end_idx, &format!("{{{{{}}}}}", cand_name));
                                                    let content = {
                                                        let ws = ws_c.read(cx);
                                                        build_tokenized_content(&new_u, ws)
                                                    };
                                                    inp_c.update(cx, |inp, cx| {
                                                        inp.set_value(content, window, cx);
                                                    });
                                                    ws_data_c.update(cx, |ws, _| {
                                                        ws.update_active_request(&req_id_c, |r| {
                                                            r.apply_url(&new_u);
                                                        });
                                                    });
                                                    ed_c.update(cx, |ed, cx| {
                                                        ed.mark_dirty(cx);
                                                    });
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
                                                            .truncate()
                                                            .max_w(px(150.))
                                                            .text_xs()
                                                            .font_family(".SystemMonoFont")
                                                            .text_color(t.text_muted)
                                                            .child(cand_val),
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
                                                    .text_color(cand_color)
                                                    .child(cand_scope),
                                            )
                                    })),
                            ),
                    )
                    .with_priority(100),
                )
            })
            // Send / Cancel Button
            .child({
                let ed_s = ed_clone.clone();
                let on_send_cb = on_send.clone();

                div()
                    .id("btn-send-or-cancel")
                    .px(px(16.))
                    .h(px(34.))
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
                    .child(if is_sending { "Cancel" } else { "Send" })
            })
    }
}

use crate::components::env_autocomplete::{
    build_tokenized_content, detect_unclosed_env_var, get_env_completions, get_variable_details,
    get_variable_kind_and_color,
};
use crate::state::{EnvironmentState, ModalState};
use crate::theme::theme;
use gpui::prelude::FluentBuilder as _;
use gpui::{
    deferred, div, px, App, Entity, InteractiveElement as _, IntoElement, ParentElement,
    SharedString, StatefulInteractiveElement as _, Styled, Window,
};
use gpui_component::input::{Input, InputState};
use gpui_component::scroll::ScrollableElement as _;

pub fn render_env_input<F>(
    input_id: impl Into<SharedString>,
    input_state: &Entity<InputState>,
    env_state: Option<&Entity<EnvironmentState>>,
    modal_state: Option<&Entity<ModalState>>,
    window: &mut Window,
    cx: &mut App,
    on_update: F,
) -> impl IntoElement
where
    F: 'static + Clone + Fn(String, &mut Window, &mut App),
{
    let input_id: SharedString = input_id.into();
    let t = theme();
    let current_val = input_state.read(cx).value().to_string();

    let (completion_candidates, unclosed) = if current_val.contains("{{") {
        if let Some(env_ent) = env_state {
            let env = env_ent.read(cx);
            let unclosed = detect_unclosed_env_var(&current_val);
            let candidates = unclosed
                .as_ref()
                .map(|(_, _, q)| get_env_completions(env, q))
                .unwrap_or_default();
            (candidates, unclosed)
        } else {
            (Vec::new(), None)
        }
    } else {
        (Vec::new(), None)
    };
    let show_autocomplete = !completion_candidates.is_empty();

    let hover_token_state = window.use_keyed_state(
        SharedString::from(format!("hover-env-{}", input_id)),
        cx,
        |_window, _cx| None::<(String, gpui::Bounds<gpui::Pixels>)>,
    );
    let hovered_token = hover_token_state.read(cx).clone();

    let env_for_token = env_state.cloned();
    let modal_for_click = modal_state.cloned();
    let env_for_hover = env_state.cloned();
    let h_token = hover_token_state.clone();
    let inp_id_for_tok = input_id.clone();

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
                        .id(input_id.clone())
                        .appearance(false)
                        .bordered(false)
                        .focus_bordered(false)
                        .cleanable(true)
                        .text_xs()
                        .font_family(".SystemMonoFont")
                        .token(move |token_ctx, _window, cx| {
                            let var_name = token_ctx.token().id().to_string();
                            let (scope, is_res) = if let Some(env_ent) = &env_for_token {
                                let env = env_ent.read(cx);
                                let (_val, sc, res) = get_variable_details(env, &var_name);
                                (sc, res)
                            } else {
                                ("Missing".to_string(), false)
                            };
                            let (_kind, color, bg_tint) =
                                get_variable_kind_and_color(&scope, is_res);

                            div()
                                .id(SharedString::from(format!(
                                    "{}-tok-{}",
                                    inp_id_for_tok, var_name
                                )))
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
                                if is_h {
                                    *h = Some((name, bounds));
                                } else {
                                    *h = None;
                                }
                                cx.notify();
                            });
                        })
                        .on_token_click(move |_, _window, cx| {
                            if let Some(modal) = &modal_for_click {
                                modal.update(cx, |st, cx| {
                                    st.toggle_env_modal(cx);
                                });
                            }
                        }),
                ),
        )
        .when_some(hovered_token, |this, (var_name, _bounds)| {
            let (val, scope, is_res) = if let Some(env_ent) = &env_for_hover {
                let env = env_ent.read(cx);
                get_variable_details(env, &var_name)
            } else {
                ("Missing variable".to_string(), "Missing".to_string(), false)
            };
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
                                    "Variable not found in active or global environments"
                                        .to_string()
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
            let (start_idx, end_idx) = unclosed
                .as_ref()
                .map(|(pos, end, _)| (*pos, *end))
                .unwrap_or((0, 0));
            let current_text = current_val.clone();
            let inp_target = input_state.clone();
            let env_target = env_state.cloned();
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
                                    let env_c = env_target.clone();
                                    let on_up_c = on_up.clone();
                                    let (_kind, cand_color, cand_bg_tint) =
                                        get_variable_kind_and_color(&cand_scope, true);

                                    div()
                                        .id(SharedString::from(format!(
                                            "{}-cand-{}",
                                            input_id, cand.name
                                        )))
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
                                            if start_idx <= new_text.len()
                                                && end_idx <= new_text.len()
                                                && start_idx <= end_idx
                                            {
                                                new_text.replace_range(
                                                    start_idx..end_idx,
                                                    &format!("{{{{{}}}}}", cand_name),
                                                );
                                                let content = if let Some(env_ent) = &env_c {
                                                    let env = env_ent.read(cx);
                                                    build_tokenized_content(&new_text, env)
                                                } else {
                                                    gpui_component::input::InputContent::new(
                                                        &new_text,
                                                    )
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

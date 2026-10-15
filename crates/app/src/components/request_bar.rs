use crate::components::env_autocomplete::{
    build_tokenized_content, detect_unclosed_env_var, get_env_completions, get_variable_details,
    get_variable_kind_and_color,
};
use crate::components::status_badge::MethodBadge;
use crate::state::{EditorState, WorkspaceState};
use crate::theme::theme;
use gpui::prelude::FluentBuilder as _;
use gpui::{
    deferred, div, px, App, AppContext as _, Entity, FocusHandle, Focusable as _, FontWeight,
    InteractiveElement as _, IntoElement, ParentElement, RenderOnce,
    StatefulInteractiveElement as _, Styled, Window,
};
use gpui_component::input::{Input, InputState};
use gpui_component::scroll::ScrollableElement as _;
use restclient_core::interpolation::scan_variables;
use restclient_core::models::HttpMethod;

struct UrlParts<'a> {
    protocol: Option<&'a str>,
    domain: &'a str,
    path: Option<&'a str>,
}

fn parse_url_parts(url: &str) -> UrlParts<'_> {
    let (protocol, rest) = if let Some(idx) = url.find("://") {
        (Some(&url[..idx + 3]), &url[idx + 3..])
    } else {
        (None, url)
    };

    let mut split_idx = None;
    let mut depth: usize = 0;
    for (i, c) in rest.char_indices() {
        if c == '{' {
            depth += 1;
        } else if c == '}' {
            depth = depth.saturating_sub(1);
        } else if depth == 0 && (c == '/' || c == '?' || c == '#') {
            split_idx = Some(i);
            break;
        }
    }

    if let Some(idx) = split_idx {
        UrlParts {
            protocol,
            domain: &rest[..idx],
            path: Some(&rest[idx..]),
        }
    } else {
        UrlParts {
            protocol,
            domain: rest,
            path: None,
        }
    }
}

fn render_url_part_with_vars(
    text: &str,
    is_domain: bool,
    text_color: gpui::Hsla,
    t: &crate::theme::AppTheme,
    var_map: &std::collections::HashMap<String, (String, String, bool)>,
    ws_modal: Entity<WorkspaceState>,
    hover_token_state: Entity<Option<(String, gpui::Bounds<gpui::Pixels>)>>,
) -> gpui::Div {
    let mut container = div().flex().items_center();
    let mut last_idx = 0;
    let bytes = text.as_bytes();
    let mut i = 0;

    while i + 1 < text.len() {
        if bytes[i] == b'{' && bytes[i + 1] == b'{' {
            if i > last_idx {
                let plain = &text[last_idx..i];
                container = container.child(
                    div()
                        .text_xs()
                        .font_family(".SystemMonoFont")
                        .when(is_domain, |s| s.font_weight(FontWeight::BOLD).text_color(t.text_primary))
                        .when(!is_domain, |s| s.text_color(text_color))
                        .child(plain.to_string()),
                );
            }
            if let Some(close_rel) = text[i + 2..].find("}}") {
                let close_idx = i + 2 + close_rel;
                let var_name = text[i + 2..close_idx].trim();
                let (_val, scope, is_res) = var_map
                    .get(var_name)
                    .cloned()
                    .unwrap_or_else(|| ("".to_string(), "".to_string(), false));
                let (_kind, color, bg_tint) = get_variable_kind_and_color(&scope, is_res);

                let ws_m = ws_modal.clone();
                let h_t = hover_token_state.clone();
                let v_name = var_name.to_string();
                container = container.child(
                    div()
                        .id(gpui::SharedString::from(format!("preview-token-{}", var_name)))
                        .flex()
                        .items_center()
                        .px(px(5.))
                        .py(px(1.))
                        .rounded(px(3.))
                        .bg(bg_tint)
                        .border_1()
                        .border_color(color)
                        .text_xs()
                        .font_family(".SystemMonoFont")
                        .text_color(color)
                        .cursor_pointer()
                        .on_hover(move |is_h, _, cx| {
                            let name = v_name.clone();
                            h_t.update(cx, |h, cx| {
                                if *is_h {
                                    *h = Some((name, gpui::Bounds::default()));
                                } else {
                                    *h = None;
                                }
                                cx.notify();
                            });
                        })
                        .on_click(move |_, _, cx| {
                            cx.stop_propagation();
                            ws_m.update(cx, |state, cx| {
                                state.toggle_env_modal(cx);
                            });
                        })
                        .child(var_name.to_string()),
                );
                i = close_idx + 2;
                last_idx = i;
                continue;
            }
        }
        i += 1;
    }

    if last_idx < text.len() {
        let plain = &text[last_idx..];
        container = container.child(
            div()
                .text_xs()
                .font_family(".SystemMonoFont")
                .when(is_domain, |s| s.font_weight(FontWeight::BOLD).text_color(t.text_primary))
                .when(!is_domain, |s| s.text_color(text_color))
                .child(plain.to_string()),
        );
    }

    container
}

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
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = theme();
        let is_focused = self.url_input.read(cx).focus_handle(cx).is_focused(window);
        let (active_req, is_sending) = {
            let ed = self.editor_state.read(cx);
            (ed.active_request.clone(), ed.is_sending)
        };
        let method = active_req.as_ref().map(|r| r.method).unwrap_or(HttpMethod::Get);
        let url = self.url_input.read(cx).value().to_string();

        let ws_clone = self.workspace_state.clone();
        let ed_clone = self.editor_state.clone();
        let url_input_clone = self.url_input.clone();
        let on_send = self.on_send.clone();

        // Scan variables in URL and build completions while reading ws
        let (var_map, completion_candidates, tokenized_content) = {
            let ws = self.workspace_state.read(cx);
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
                .map(|(_, q)| get_env_completions(ws, q))
                .unwrap_or_default();

            let tokenized = build_tokenized_content(&url, ws);
            (map, candidates, tokenized)
        };

        // If inline tokens count does not match scanned variables count, sync inline tokens
        let current_tokens_count = self.url_input.read(cx).tokens().len();
        let desired_tokens_count = tokenized_content.tokens().len();
        if current_tokens_count != desired_tokens_count {
            self.url_input.update(cx, |inp, cx| {
                inp.set_value(tokenized_content, window, cx);
            });
        }

        let hover_token_state = window.use_keyed_state(
            gpui::SharedString::from("url-inline-token-hover"),
            cx,
            |_window, _cx| None::<(String, gpui::Bounds<gpui::Pixels>)>,
        );
        let hovered_token = hover_token_state.read(cx).clone();

        let unclosed = detect_unclosed_env_var(&url);
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
                            .child(MethodBadge::new(method))
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
                            .child(
                                div()
                                    .size_full()
                                    .flex()
                                    .items_center()
                                    .when(!is_focused && !url.is_empty(), |this| this.opacity(0.))
                                    .child({
                                        let ws_env = ws_clone.clone();
                                        let ws_modal = ws_clone.clone();
                                        let h_token = hover_token_state.clone();

                                        Input::new(&self.url_input)
                                            .id("url-input-field")
                                            .appearance(false)
                                            .bordered(false)
                                            .focus_bordered(false)
                                            .cleanable(true)
                                            .px(px(2.))
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
                                                    .border_1()
                                                    .border_color(color)
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
                                                ws_modal.update(cx, |state, cx| {
                                                    state.toggle_env_modal(cx);
                                                });
                                            })
                                    }),
                            )
                            .when(!is_focused && !url.is_empty(), |this| {
                                let url_parts = parse_url_parts(&url);
                                let focus_h = self.url_input.read(cx).focus_handle(cx);
                                let ws_modal_c = ws_clone.clone();
                                let h_token_c = hover_token_state.clone();

                                this.child(
                                    div()
                                        .id("url-formatted-preview")
                                        .absolute()
                                        .inset_0()
                                        .pl(px(2.))
                                        .flex()
                                        .items_center()
                                        .cursor_text()
                                        .overflow_hidden()
                                        .on_click(move |_, window, cx| {
                                            focus_h.focus(window, cx);
                                        })
                                        .when_some(url_parts.protocol, |this, proto| {
                                            this.child(
                                                div()
                                                    .text_xs()
                                                    .font_family(".SystemMonoFont")
                                                    .text_color(gpui::Hsla { a: 0.45, ..t.text_primary })
                                                    .child(proto.to_string()),
                                            )
                                        })
                                        .child(render_url_part_with_vars(
                                            url_parts.domain,
                                            true,
                                            t.text_primary,
                                            &t,
                                            &var_map,
                                            ws_modal_c.clone(),
                                            h_token_c.clone(),
                                        ))
                                        .when_some(url_parts.path, |this, path| {
                                            this.child(render_url_part_with_vars(
                                                path,
                                                false,
                                                gpui::Hsla { a: 0.55, ..t.text_primary },
                                                &t,
                                                &var_map,
                                                ws_modal_c.clone(),
                                                h_token_c.clone(),
                                            ))
                                        }),
                                )
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
                            .bg(crate::theme::hex_to_hsla(0x161822))
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
                let start_idx = unclosed.as_ref().map(|(pos, _)| *pos).unwrap_or(0);
                let current_url = url.clone();
                let inp_target = url_input_clone.clone();
                let ed_target = ed_clone.clone();

                this.child(
                    deferred(
                        div()
                            .absolute()
                            .top(px(42.))
                            .left(px(80.))
                            .w(px(380.))
                            .max_h(px(240.))
                            .rounded(px(6.))
                            .bg(crate::theme::hex_to_hsla(0x161822))
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
                                    .bg(crate::theme::hex_to_hsla(0x1c1e2b))
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
                                    .bg(crate::theme::hex_to_hsla(0x161822))
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
                                                if start_idx <= new_u.len() {
                                                    new_u.replace_range(start_idx.., &format!("{{{{{}}}}}", cand_name));
                                                    let content = {
                                                        let ws = ws_c.read(cx);
                                                        build_tokenized_content(&new_u, ws)
                                                    };
                                                    inp_c.update(cx, |inp, cx| {
                                                        inp.set_value(content, window, cx);
                                                    });
                                                    ed_c.update(cx, |ed, cx| {
                                                        ed.set_url(new_u, cx);
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

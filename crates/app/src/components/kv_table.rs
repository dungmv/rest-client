use crate::components::env_autocomplete::{
    build_tokenized_content, detect_unclosed_env_var, get_env_completions, get_variable_details,
    get_variable_kind_and_color, EnvVarCandidate,
};
use crate::state::{EnvironmentState, ModalState};
use crate::theme::theme;
use gpui::prelude::FluentBuilder as _;
use gpui::{
    deferred, div, px, App, AppContext as _, Entity, InteractiveElement as _, IntoElement,
    ParentElement, RenderOnce, StatefulInteractiveElement as _, Styled, Subscription, Window,
};
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::scroll::ScrollableElement as _;
use restclient_core::interpolation::scan_variables;
use restclient_core::models::KeyValuePair;
use std::rc::Rc;

struct KvRowInputHolder {
    key_input: Entity<InputState>,
    val_input: Entity<InputState>,
    _subscriptions: Vec<Subscription>,
}

#[derive(Clone)]
struct RowEnvData {
    key_vars: Vec<(String, String, String, bool)>,
    val_vars: Vec<(String, String, String, bool)>,
    key_completions: Option<(usize, usize, Vec<EnvVarCandidate>)>,
    val_completions: Option<(usize, usize, Vec<EnvVarCandidate>)>,
}

#[allow(clippy::type_complexity)]
#[derive(IntoElement)]
pub struct KvTable {
    id_prefix: String,
    pairs: Vec<KeyValuePair>,
    on_update: Rc<dyn Fn(usize, String, String, bool, &mut Window, &mut App)>,
    on_add: Rc<dyn Fn(&mut Window, &mut App)>,
    on_remove: Rc<dyn Fn(usize, &mut Window, &mut App)>,
    env_state: Option<Entity<EnvironmentState>>,
    modal_state: Option<Entity<ModalState>>,
}

impl KvTable {
    pub fn new(
        id_prefix: impl Into<String>,
        pairs: Vec<KeyValuePair>,
        on_update: impl Fn(usize, String, String, bool, &mut Window, &mut App) + 'static,
        on_add: impl Fn(&mut Window, &mut App) + 'static,
        on_remove: impl Fn(usize, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id_prefix: id_prefix.into(),
            pairs,
            on_update: Rc::new(on_update),
            on_add: Rc::new(on_add),
            on_remove: Rc::new(on_remove),
            env_state: None,
            modal_state: None,
        }
    }

    pub fn env_state(mut self, env: Entity<EnvironmentState>) -> Self {
        self.env_state = Some(env);
        self
    }

    pub fn optional_env_state(mut self, env: Option<Entity<EnvironmentState>>) -> Self {
        self.env_state = env;
        self
    }

    pub fn modal_state(mut self, modal: Entity<ModalState>) -> Self {
        self.modal_state = Some(modal);
        self
    }
}

impl RenderOnce for KvTable {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = theme();
        let on_add_cb = self.on_add.clone();
        let id_prefix = self.id_prefix.clone();
        let env_clone = self.env_state.clone();
        let modal_clone = self.modal_state.clone();

        // Precompute variable information for all rows before running use_keyed_state
        let row_env_info: Vec<RowEnvData> = if let Some(env_entity) = &self.env_state {
            let env = env_entity.read(cx);
            let effective_env = env.effective_environment();
            self.pairs
                .iter()
                .map(|pair| {
                    let key_vars = if pair.key.contains("{{") {
                        scan_variables(&pair.key, Some(&effective_env))
                            .into_iter()
                            .map(|v| {
                                let (val, scope, is_res) = get_variable_details(env, &v.name);
                                (v.name, val, scope, is_res)
                            })
                            .collect()
                    } else {
                        vec![]
                    };

                    let val_vars = if pair.value.contains("{{") {
                        scan_variables(&pair.value, Some(&effective_env))
                            .into_iter()
                            .map(|v| {
                                let (val, scope, is_res) = get_variable_details(env, &v.name);
                                (v.name, val, scope, is_res)
                            })
                            .collect()
                    } else {
                        vec![]
                    };

                    let key_completions = if pair.key.contains("{{") {
                        detect_unclosed_env_var(&pair.key)
                            .map(|(pos, end, q)| (pos, end, get_env_completions(env, &q)))
                    } else {
                        None
                    };

                    let val_completions = if pair.value.contains("{{") {
                        detect_unclosed_env_var(&pair.value)
                            .map(|(pos, end, q)| (pos, end, get_env_completions(env, &q)))
                    } else {
                        None
                    };

                    RowEnvData {
                        key_vars,
                        val_vars,
                        key_completions,
                        val_completions,
                    }
                })
                .collect()
        } else {
            (0..self.pairs.len())
                .map(|_| RowEnvData {
                    key_vars: vec![],
                    val_vars: vec![],
                    key_completions: None,
                    val_completions: None,
                })
                .collect()
        };

        div()
            .w_full()
            .flex()
            .flex_col()
            .gap(px(4.))
            .p(px(8.))
            // Table Header
            .child(
                div()
                    .flex()
                    .items_center()
                    .px(px(6.))
                    .py(px(4.))
                    .text_xs()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(t.text_muted)
                    .border_b_1()
                    .border_color(t.border)
                    .child(div().w(px(28.)).child(""))
                    .child(div().w(px(220.)).child("Key"))
                    .child(div().flex_1().child("Value"))
                    .child(div().w(px(28.)).child("")),
            )
            // Table Rows
            .children(self.pairs.into_iter().enumerate().map(|(idx, pair)| {
                let on_update = self.on_update.clone();
                let on_remove = self.on_remove.clone();
                let key_val = pair.key.clone();
                let val_val = pair.value.clone();
                let enabled = pair.enabled;
                let row_prefix = format!("{}-row-{}", id_prefix, idx);
                let row_env = row_env_info.get(idx).cloned().unwrap_or_else(|| RowEnvData {
                    key_vars: vec![],
                    val_vars: vec![],
                    key_completions: None,
                    val_completions: None,
                });

                let on_up_k = on_update.clone();
                let on_up_v = on_update.clone();
                let env_for_init = self.env_state.clone();
                let k_init_val = key_val.clone();
                let v_init_val = val_val.clone();
                let row_holder = window.use_keyed_state(
                    gpui::SharedString::from(format!("{}-holder", row_prefix)),
                    cx,
                    |window, cx| {
                        let init_k = if let Some(env_m) = &env_for_init {
                            build_tokenized_content(&k_init_val, env_m.read(cx))
                        } else {
                            k_init_val.into()
                        };
                        let init_v = if let Some(env_m) = &env_for_init {
                            build_tokenized_content(&v_init_val, env_m.read(cx))
                        } else {
                            v_init_val.into()
                        };
                        let key_input = cx.new(|cx| {
                            let mut s = InputState::new(window, cx).placeholder("Key...");
                            s.set_value(init_k, window, cx);
                            s
                        });
                        let val_input = cx.new(|cx| {
                            let mut s = InputState::new(window, cx).placeholder("Value...");
                            s.set_value(init_v, window, cx);
                            s
                        });

                        let k_in_1 = key_input.clone();
                        let v_in_1 = val_input.clone();
                        let sub1 = cx.subscribe_in(&key_input, window, move |_, _, event, window, cx| {
                            if let InputEvent::Change = event {
                                let k = k_in_1.read(cx).value().to_string();
                                let v = v_in_1.read(cx).value().to_string();
                                on_up_k(idx, k, v, true, window, cx);
                            }
                        });

                        let k_in_2 = key_input.clone();
                        let v_in_2 = val_input.clone();
                        let sub2 = cx.subscribe_in(&val_input, window, move |_, _, event, window, cx| {
                            if let InputEvent::Change = event {
                                let k = k_in_2.read(cx).value().to_string();
                                let v = v_in_2.read(cx).value().to_string();
                                on_up_v(idx, k, v, true, window, cx);
                            }
                        });

                        KvRowInputHolder {
                            key_input,
                            val_input,
                            _subscriptions: vec![sub1, sub2],
                        }
                    },
                );

                let (key_input, val_input) = {
                    let h = row_holder.read(cx);
                    (h.key_input.clone(), h.val_input.clone())
                };

                let hover_k_state = window.use_keyed_state(
                    gpui::SharedString::from(format!("hover-ktoken-{}", row_prefix)),
                    cx,
                    |_window, _cx| None::<(String, gpui::Bounds<gpui::Pixels>)>,
                );
                let hover_v_state = window.use_keyed_state(
                    gpui::SharedString::from(format!("hover-vtoken-{}", row_prefix)),
                    cx,
                    |_window, _cx| None::<(String, gpui::Bounds<gpui::Pixels>)>,
                );
                let hovered_k = hover_k_state.read(cx).clone();
                let hovered_v = hover_v_state.read(cx).clone();

                div()
                    .flex()
                    .items_center()
                    .px(px(6.))
                    .py(px(2.))
                    .gap(px(8.))
                    .rounded(px(4.))
                    .hover(|s| s.bg(t.bg_hover))
                    // Checkbox
                    .child({
                        let on_update_c = on_update.clone();
                        let k = key_val.clone();
                        let v = val_val.clone();
                        div()
                            .id(gpui::SharedString::from(format!("{}-check", row_prefix)))
                            .w(px(20.))
                            .h(px(20.))
                            .rounded(px(4.))
                            .border_1()
                            .border_color(if enabled { t.primary } else { t.border })
                            .bg(if enabled { t.primary } else { t.bg_input })
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .text_xs()
                            .text_color(t.primary_foreground)
                            .on_click(move |_, window, cx| {
                                on_update_c(idx, k.clone(), v.clone(), !enabled, window, cx);
                            })
                            .child(if enabled { "✓" } else { "" })
                    })
                    // Key Input with hover badge and autocomplete
                    .child(
                        div()
                            .relative()
                            .w(px(220.))
                            .h(px(28.))
                            .px(px(4.))
                            .rounded(px(4.))
                            .bg(t.bg_input)
                            .border_1()
                            .border_color(t.border)
                            .flex()
                            .items_center()
                            .child(
                                div()
                                    .flex_1()
                                    .child({
                                        let env_k = env_clone.clone();
                                        let modal_k = modal_clone.clone();
                                        let h_k = hover_k_state.clone();
                                        Input::new(&key_input)
                                            .id(gpui::SharedString::from(format!("{}-key", row_prefix)))
                                            .appearance(false)
                                            .bordered(false)
                                            .focus_bordered(false)
                                            .token(move |token_ctx, _window, cx| {
                                                let var_name = token_ctx.token().id().to_string();
                                                let (color, bg_tint) = if let Some(env) = &env_k {
                                                    let (_val, scope, is_res) = get_variable_details(env.read(cx), &var_name);
                                                    let (_kind, c, bg) = get_variable_kind_and_color(&scope, is_res);
                                                    (c, bg)
                                                } else {
                                                    let (_kind, c, bg) = get_variable_kind_and_color("Missing", false);
                                                    (c, bg)
                                                };
                                                div()
                                                    .id(gpui::SharedString::from(format!("k-tok-{}", var_name)))
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
                                                h_k.update(cx, |h, cx| {
                                                    if is_h { *h = Some((name, bounds)); } else { *h = None; }
                                                    cx.notify();
                                                });
                                            })
                                            .when_some(modal_k, |el, modal| {
                                                el.on_token_click(move |_, _window, cx| {
                                                    modal.update(cx, |state, cx| {
                                                        state.toggle_env_modal(cx);
                                                    });
                                                })
                                            })
                                    }),
                            )
                            // Tooltip card shown ON HOVER over inline key token
                            .when_some(hovered_k.clone(), |this, (var_name, _)| {
                                let (val, scope, is_res) = row_env.key_vars.iter()
                                    .find(|(n, _, _, _)| n == &var_name)
                                    .map(|(_, v, s, r)| (v.clone(), s.clone(), *r))
                                    .unwrap_or_else(|| ("".to_string(), "".to_string(), false));
                                let (_kind, color, bg_tint) = get_variable_kind_and_color(&scope, is_res);
                                this.child(
                                    deferred(
                                        div()
                                            .absolute()
                                            .top(px(30.))
                                            .left(px(0.))
                                            .w(px(280.))
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
                            // Floating Autocomplete Menu for Key
                            .when_some(row_env.key_completions.clone(), |this, (start_pos, end_pos, completions)| {
                                if completions.is_empty() {
                                    return this;
                                }
                                let key_input_c = key_input.clone();
                                let cur_k = key_val.clone();
                                let cur_v = val_val.clone();
                                let on_up_c = on_update.clone();
                                let env_k_c = env_clone.clone();

                                this.child(
                                    deferred(
                                        div()
                                            .absolute()
                                            .top(px(30.))
                                            .left(px(0.))
                                            .w(px(320.))
                                            .max_h(px(200.))
                                            .overflow_y_scrollbar()
                                            .p(px(4.))
                                            .rounded(px(6.))
                                            .bg(t.bg_card)
                                            .border_1()
                                            .border_color(t.primary)
                                            .shadow_2xl()
                                            .occlude()
                                            .flex()
                                            .flex_col()
                                            .gap(px(2.))
                                            .children(completions.into_iter().map(|item| {
                                                let item_name = item.name.clone();
                                                let item_val = item.value.clone();
                                                let item_scope = item.scope.clone();
                                                let k_in = key_input_c.clone();
                                                let cur_k_item = cur_k.clone();
                                                let cur_v_item = cur_v.clone();
                                                let on_up = on_up_c.clone();
                                                let env_k_inner = env_k_c.clone();
                                                let (_kind, item_color, item_bg_tint) = get_variable_kind_and_color(&item_scope, true);

                                                div()
                                                    .id(gpui::SharedString::from(format!("{}-key-ac-{}", row_prefix, item.name)))
                                                    .px(px(8.))
                                                    .py(px(5.))
                                                    .rounded(px(4.))
                                                    .cursor_pointer()
                                                    .hover(|s| s.bg(t.bg_hover))
                                                    .flex()
                                                    .items_center()
                                                    .justify_between()
                                                    .on_click(move |_, window, cx| {
                                                        let mut new_key = cur_k_item.clone();
                                                        if start_pos <= new_key.len() && end_pos <= new_key.len() && start_pos <= end_pos {
                                                            new_key.replace_range(start_pos..end_pos, &format!("{{{{{}}}}}", item_name));
                                                            let content = if let Some(env_m) = &env_k_inner {
                                                                build_tokenized_content(&new_key, env_m.read(cx))
                                                            } else {
                                                                new_key.clone().into()
                                                            };
                                                            k_in.update(cx, |inp, cx| {
                                                                inp.set_value(content, window, cx);
                                                            });
                                                            on_up(idx, new_key, cur_v_item.clone(), enabled, window, cx);
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
                                                                    .text_color(item_color)
                                                                    .child(format!("{{{{{}}}}}", item.name)),
                                                            )
                                                            .child(
                                                                div()
                                                                    .truncate()
                                                                    .max_w(px(120.))
                                                                    .text_xs()
                                                                    .font_family(".SystemMonoFont")
                                                                    .text_color(t.text_muted)
                                                                    .child(item_val),
                                                            ),
                                                    )
                                                    .child(
                                                        div()
                                                            .px(px(5.))
                                                            .py(px(1.))
                                                            .rounded(px(3.))
                                                            .bg(item_bg_tint)
                                                            .border_1()
                                                            .border_color(item_color)
                                                            .text_xs()
                                                            .text_color(item_color)
                                                            .child(item_scope),
                                                    )
                                            })),
                                    )
                                    .with_priority(100),
                                )
                            }),
                    )
                    // Value Input with hover badge and autocomplete
                    .child(
                        div()
                            .relative()
                            .flex_1()
                            .h(px(28.))
                            .px(px(4.))
                            .rounded(px(4.))
                            .bg(t.bg_input)
                            .border_1()
                            .border_color(t.border)
                            .flex()
                            .items_center()
                            .child(
                                div()
                                    .flex_1()
                                    .child({
                                        let env_v = env_clone.clone();
                                        let modal_v = modal_clone.clone();
                                        let h_v = hover_v_state.clone();
                                        Input::new(&val_input)
                                            .id(gpui::SharedString::from(format!("{}-val", row_prefix)))
                                            .appearance(false)
                                            .bordered(false)
                                            .focus_bordered(false)
                                            .token(move |token_ctx, _window, cx| {
                                                let var_name = token_ctx.token().id().to_string();
                                                let (color, bg_tint) = if let Some(env) = &env_v {
                                                    let (_val, scope, is_res) = get_variable_details(env.read(cx), &var_name);
                                                    let (_kind, c, bg) = get_variable_kind_and_color(&scope, is_res);
                                                    (c, bg)
                                                } else {
                                                    let (_kind, c, bg) = get_variable_kind_and_color("Missing", false);
                                                    (c, bg)
                                                };
                                                div()
                                                    .id(gpui::SharedString::from(format!("v-tok-{}", var_name)))
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
                                                h_v.update(cx, |h, cx| {
                                                    if is_h { *h = Some((name, bounds)); } else { *h = None; }
                                                    cx.notify();
                                                });
                                            })
                                            .when_some(modal_v, |el, modal| {
                                                el.on_token_click(move |_, _window, cx| {
                                                    modal.update(cx, |state, cx| {
                                                        state.toggle_env_modal(cx);
                                                    });
                                                })
                                            })
                                    }),
                            )
                            // Tooltip card shown ON HOVER over inline value token
                            .when_some(hovered_v.clone(), |this, (var_name, _)| {
                                let (val, scope, is_res) = row_env.val_vars.iter()
                                    .find(|(n, _, _, _)| n == &var_name)
                                    .map(|(_, v, s, r)| (v.clone(), s.clone(), *r))
                                    .unwrap_or_else(|| ("".to_string(), "".to_string(), false));
                                let (_kind, color, bg_tint) = get_variable_kind_and_color(&scope, is_res);
                                this.child(
                                    deferred(
                                        div()
                                            .absolute()
                                            .top(px(30.))
                                            .left(px(0.))
                                            .w(px(280.))
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
                            // Floating Autocomplete Menu for Value
                            .when_some(row_env.val_completions.clone(), |this, (start_pos, end_pos, completions)| {
                                if completions.is_empty() {
                                    return this;
                                }
                                let val_input_c = val_input.clone();
                                let cur_k = key_val.clone();
                                let cur_v = val_val.clone();
                                let on_up_c = on_update.clone();
                                let env_v_c = env_clone.clone();

                                this.child(
                                    deferred(
                                        div()
                                            .absolute()
                                            .top(px(30.))
                                            .left(px(0.))
                                            .w(px(320.))
                                            .max_h(px(200.))
                                            .overflow_y_scrollbar()
                                            .p(px(4.))
                                            .rounded(px(6.))
                                            .bg(t.bg_card)
                                            .border_1()
                                            .border_color(t.primary)
                                            .shadow_2xl()
                                            .occlude()
                                            .flex()
                                            .flex_col()
                                            .gap(px(2.))
                                            .children(completions.into_iter().map(|item| {
                                                let item_name = item.name.clone();
                                                let item_val = item.value.clone();
                                                let item_scope = item.scope.clone();
                                                let v_in = val_input_c.clone();
                                                let cur_val = cur_v.clone();
                                                let cur_k = cur_k.clone();
                                                let on_up = on_up_c.clone();
                                                let env_v_inner = env_v_c.clone();
                                                let (_kind, item_color, item_bg_tint) = get_variable_kind_and_color(&item_scope, true);

                                                div()
                                                    .id(gpui::SharedString::from(format!("{}-val-ac-{}", row_prefix, item.name)))
                                                    .px(px(8.))
                                                    .py(px(5.))
                                                    .rounded(px(4.))
                                                    .cursor_pointer()
                                                    .hover(|s| s.bg(t.bg_hover))
                                                    .flex()
                                                    .items_center()
                                                    .justify_between()
                                                    .on_click(move |_, window, cx| {
                                                        let mut new_val = cur_val.clone();
                                                        if start_pos <= new_val.len() && end_pos <= new_val.len() && start_pos <= end_pos {
                                                            new_val.replace_range(start_pos..end_pos, &format!("{{{{{}}}}}", item_name));
                                                            let content = if let Some(env_m) = &env_v_inner {
                                                                build_tokenized_content(&new_val, env_m.read(cx))
                                                            } else {
                                                                new_val.clone().into()
                                                            };
                                                            v_in.update(cx, |inp, cx| {
                                                                inp.set_value(content, window, cx);
                                                            });
                                                            on_up(idx, cur_k.clone(), new_val, enabled, window, cx);
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
                                                                    .text_color(item_color)
                                                                    .child(format!("{{{{{}}}}}", item.name)),
                                                            )
                                                            .child(
                                                                div()
                                                                    .truncate()
                                                                    .max_w(px(120.))
                                                                    .text_xs()
                                                                    .font_family(".SystemMonoFont")
                                                                    .text_color(t.text_muted)
                                                                    .child(item_val),
                                                            ),
                                                    )
                                                    .child(
                                                        div()
                                                            .px(px(5.))
                                                            .py(px(1.))
                                                            .rounded(px(3.))
                                                            .bg(item_bg_tint)
                                                            .border_1()
                                                            .border_color(item_color)
                                                            .text_xs()
                                                            .text_color(item_color)
                                                            .child(item_scope),
                                                    )
                                            })),
                                    )
                                    .with_priority(100),
                                )
                            }),
                    )
                    // Delete button
                    .child({
                        div()
                            .id(gpui::SharedString::from(format!("{}-del", row_prefix)))
                            .w(px(24.))
                            .h(px(24.))
                            .rounded(px(4.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .hover(|s| s.bg(t.bg_hover).text_color(t.status_5xx))
                            .text_color(t.text_muted)
                            .text_xs()
                            .on_click(move |_, window, cx| {
                                on_remove(idx, window, cx);
                            })
                            .child("×")
                    })
            }))
            // Add row button
            .child(
                div()
                    .pt(px(6.))
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(
                        div()
                            .id(gpui::SharedString::from(format!("{}-btn-add", id_prefix)))
                            .px(px(10.))
                            .py(px(4.))
                            .rounded(px(4.))
                            .bg(t.bg_card)
                            .border_1()
                            .border_color(t.border)
                            .cursor_pointer()
                            .hover(|s| s.bg(t.bg_hover).text_color(t.text_primary))
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_xs()
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .text_color(t.text_secondary)
                            .on_click(move |_, window, cx| {
                                on_add_cb(window, cx);
                            })
                            .child("+ Add Row"),
                    ),
            )
    }
}

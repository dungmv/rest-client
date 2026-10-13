use crate::state::WorkspaceState;
use crate::theme::theme;
use gpui::prelude::FluentBuilder as _;
use gpui::{
    div, px, App, AppContext as _, Entity, InteractiveElement as _, IntoElement, ParentElement,
    RenderOnce, StatefulInteractiveElement as _, Styled, Window,
};
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::scroll::ScrollableElement as _;
use restclient_core::models::{Environment, EnvironmentVariable};

#[derive(IntoElement)]
pub struct EnvironmentModal {
    workspace_state: Entity<WorkspaceState>,
}

impl EnvironmentModal {
    pub fn new(workspace_state: Entity<WorkspaceState>) -> Self {
        Self { workspace_state }
    }
}

impl RenderOnce for EnvironmentModal {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = theme();
        let ws = self.workspace_state.read(cx);
        let envs = ws.environments.clone();
        let active_env_id = ws.active_environment_id.clone();
        let ws_clone = self.workspace_state.clone();

        div()
            .absolute()
            .inset_0()
            .bg(gpui::Hsla { a: 0.6, ..t.bg_app })
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .w(px(640.))
                    .h(px(460.))
                    .rounded(px(8.))
                    .bg(t.bg_card)
                    .border_1()
                    .border_color(t.border)
                    .shadow_xl()
                    .flex()
                    .flex_col()
                    // Header
                    .child(
                        div()
                            .h(px(44.))
                            .px(px(16.))
                            .border_b_1()
                            .border_color(t.border)
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(10.))
                                    .child(
                                        div()
                                            .text_sm()
                                            .font_weight(gpui::FontWeight::SEMIBOLD)
                                            .text_color(t.text_primary)
                                            .child("Manage Environments"),
                                    )
                                    .child({
                                        let ws_add_env = ws_clone.clone();
                                        div()
                                            .id("btn-add-environment")
                                            .px(px(8.))
                                            .py(px(3.))
                                            .rounded(px(4.))
                                            .bg(t.primary)
                                            .text_color(t.primary_foreground)
                                            .cursor_pointer()
                                            .hover(|s| s.bg(t.primary_hover))
                                            .text_xs()
                                            .font_weight(gpui::FontWeight::MEDIUM)
                                            .on_click(move |_, _, cx| {
                                                ws_add_env.update(cx, |state, cx| {
                                                    let ws_id = state.active_workspace.as_ref().map(|w| w.id.clone()).unwrap_or_else(|| "default".to_string());
                                                    let mut new_env = Environment::new(ws_id, format!("env-{}", state.environments.len() + 1));
                                                    new_env.variables.push(EnvironmentVariable {
                                                        enabled: true,
                                                        key: "BASE_URL".to_string(),
                                                        value: "http://localhost:8080".to_string(),
                                                        secret: false,
                                                    });
                                                    let new_id = new_env.id.clone();
                                                    let mut list = state.environments.clone();
                                                    list.push(new_env);
                                                    state.save_environments(list, cx);
                                                    if state.active_environment_id.is_none() {
                                                        state.set_active_environment(Some(new_id), cx);
                                                    }
                                                });
                                            })
                                            .child("+ New Environment")
                                    }),
                            )
                            .child({
                                let ws_s = ws_clone.clone();
                                div()
                                    .id("close-env-modal")
                                    .cursor_pointer()
                                    .text_color(t.text_muted)
                                    .hover(|s| s.text_color(t.text_primary))
                                    .on_click(move |_, _, cx| {
                                        ws_s.update(cx, |state, cx| {
                                            state.toggle_env_modal(cx);
                                        });
                                    })
                                    .child("✕")
                            }),
                    )
                    // Content: Environments list + variables
                    .child(
                        div()
                            .flex_1()
                            .p(px(16.))
                            .overflow_y_scrollbar()
                            .flex()
                            .flex_col()
                            .gap(px(12.))
                            .when(envs.is_empty(), |this| {
                                this.child(
                                    div()
                                        .py(px(40.))
                                        .flex()
                                        .flex_col()
                                        .items_center()
                                        .justify_center()
                                        .gap(px(8.))
                                        .text_xs()
                                        .text_color(t.text_muted)
                                        .child("No environments configured yet. Click '+ New Environment' above to create one."),
                                )
                            })
                            .children(envs.into_iter().enumerate().map(|(env_idx, env)| {
                                let is_active = active_env_id.as_deref() == Some(&env.id);
                                let env_id = env.id.clone();
                                let env_id_click = env.id.clone();
                                let ws_s = ws_clone.clone();
                                let ws_vars = ws_clone.clone();
                                let ws_add_var = ws_clone.clone();

                                div()
                                    .flex()
                                    .flex_col()
                                    .gap(px(8.))
                                    .p(px(10.))
                                    .rounded(px(6.))
                                    .border_1()
                                    .border_color(if is_active { t.primary } else { t.border })
                                    .bg(t.bg_panel)
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .justify_between()
                                            .child(
                                                div()
                                                    .flex()
                                                    .items_center()
                                                    .gap(px(8.))
                                                    .child(
                                                        div()
                                                            .text_sm()
                                                            .font_weight(gpui::FontWeight::SEMIBOLD)
                                                            .text_color(t.text_primary)
                                                            .child(env.name.clone()),
                                                    )
                                                    .when(is_active, |this| {
                                                        this.child(
                                                            div()
                                                                .px(px(6.))
                                                                .py(px(1.))
                                                                .rounded(px(10.))
                                                                .bg(t.primary)
                                                                .text_xs()
                                                                .text_color(t.primary_foreground)
                                                                .child("Active"),
                                                        )
                                                    }),
                                            )
                                            .child(
                                                div()
                                                    .flex()
                                                    .items_center()
                                                    .gap(px(6.))
                                                    .child({
                                                        let env_id_add = env_id.clone();
                                                        div()
                                                            .id(format!("add-var-env-{}", env.id))
                                                            .px(px(8.))
                                                            .py(px(2.))
                                                            .rounded(px(4.))
                                                            .bg(t.bg_card)
                                                            .border_1()
                                                            .border_color(t.border)
                                                            .cursor_pointer()
                                                            .hover(|s| s.bg(t.bg_hover))
                                                            .text_xs()
                                                            .text_color(t.text_secondary)
                                                            .on_click(move |_, _, cx| {
                                                                ws_add_var.update(cx, |state, cx| {
                                                                    let mut list = state.environments.clone();
                                                                    if let Some(e) = list.iter_mut().find(|x| x.id == env_id_add) {
                                                                        e.variables.push(EnvironmentVariable {
                                                                            enabled: true,
                                                                            key: "".to_string(),
                                                                            value: "".to_string(),
                                                                            secret: false,
                                                                        });
                                                                    }
                                                                    state.save_environments(list, cx);
                                                                });
                                                            })
                                                            .child("+ Variable")
                                                    })
                                                    .child(
                                                        div()
                                                            .id(format!("select-env-{}", env.id))
                                                            .px(px(8.))
                                                            .py(px(2.))
                                                            .rounded(px(4.))
                                                            .bg(t.bg_card)
                                                            .border_1()
                                                            .border_color(t.border)
                                                            .cursor_pointer()
                                                            .hover(|s| s.bg(t.bg_hover))
                                                            .text_xs()
                                                            .text_color(t.text_secondary)
                                                            .on_click(move |_, _, cx| {
                                                                ws_s.update(cx, |state, cx| {
                                                                    if is_active {
                                                                        state.set_active_environment(None, cx);
                                                                    } else {
                                                                        state.set_active_environment(Some(env_id_click.clone()), cx);
                                                                    }
                                                                });
                                                            })
                                                            .child(if is_active { "Deactivate" } else { "Activate" }),
                                                    ),
                                            ),
                                    )
                                    // Variable rows
                                    .children(env.variables.into_iter().enumerate().map(|(var_idx, var)| {
                                        let v_key = var.key.clone();
                                        let v_val = var.value.clone();
                                        let row_id = format!("env-{}-var-{}", env_idx, var_idx);
                                        let ws_up_k = ws_vars.clone();
                                        let ws_up_v = ws_vars.clone();
                                        let ws_del = ws_vars.clone();

                                        let holder = window.use_keyed_state(
                                            gpui::SharedString::from(format!("{}-holder", row_id)),
                                            cx,
                                            |window, cx| {
                                                let k_in = cx.new(|cx| InputState::new(window, cx).placeholder("Variable key..."));
                                                let v_in = cx.new(|cx| InputState::new(window, cx).placeholder("Variable value..."));

                                                let k1 = k_in.clone();
                                                let v1 = v_in.clone();
                                                let sub1 = cx.subscribe_in(&k_in, window, move |_, _, ev, _, cx| {
                                                    if let InputEvent::Change = ev {
                                                        let nk = k1.read(cx).value().to_string();
                                                        let nv = v1.read(cx).value().to_string();
                                                        ws_up_k.update(cx, |state, cx| {
                                                            let mut list = state.environments.clone();
                                                            if let Some(e) = list.get_mut(env_idx) {
                                                                if let Some(vr) = e.variables.get_mut(var_idx) {
                                                                    vr.key = nk;
                                                                    vr.value = nv;
                                                                }
                                                            }
                                                            state.save_environments(list, cx);
                                                        });
                                                    }
                                                });

                                                let k2 = k_in.clone();
                                                let v2 = v_in.clone();
                                                let sub2 = cx.subscribe_in(&v_in, window, move |_, _, ev, _, cx| {
                                                    if let InputEvent::Change = ev {
                                                        let nk = k2.read(cx).value().to_string();
                                                        let nv = v2.read(cx).value().to_string();
                                                        ws_up_v.update(cx, |state, cx| {
                                                            let mut list = state.environments.clone();
                                                            if let Some(e) = list.get_mut(env_idx) {
                                                                if let Some(vr) = e.variables.get_mut(var_idx) {
                                                                    vr.key = nk;
                                                                    vr.value = nv;
                                                                }
                                                            }
                                                            state.save_environments(list, cx);
                                                        });
                                                    }
                                                });

                                                (k_in, v_in, vec![sub1, sub2])
                                            },
                                        );

                                        let (k_input, v_input) = {
                                            let h = holder.read(cx);
                                            (h.0.clone(), h.1.clone())
                                        };
                                        if k_input.read(cx).value().as_str() != v_key {
                                            k_input.update(cx, |inp, cx| inp.set_value(v_key.clone(), window, cx));
                                        }
                                        if v_input.read(cx).value().as_str() != v_val {
                                            v_input.update(cx, |inp, cx| inp.set_value(v_val.clone(), window, cx));
                                        }

                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(8.))
                                            .child(
                                                div()
                                                    .w(px(180.))
                                                    .h(px(28.))
                                                    .px(px(4.))
                                                    .rounded(px(4.))
                                                    .bg(t.bg_input)
                                                    .border_1()
                                                    .border_color(t.border)
                                                    .flex()
                                                    .items_center()
                                                    .child(
                                                        Input::new(&k_input)
                                                            .id(gpui::SharedString::from(format!("{}-key", row_id)))
                                                            .appearance(false)
                                                            .bordered(false)
                                                            .focus_bordered(false),
                                                    ),
                                            )
                                            .child(
                                                div()
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
                                                        Input::new(&v_input)
                                                            .id(gpui::SharedString::from(format!("{}-val", row_id)))
                                                            .appearance(false)
                                                            .bordered(false)
                                                            .focus_bordered(false),
                                                    ),
                                            )
                                            .child(
                                                div()
                                                    .id(gpui::SharedString::from(format!("{}-del", row_id)))
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
                                                    .on_click(move |_, _, cx| {
                                                        ws_del.update(cx, |state, cx| {
                                                            let mut list = state.environments.clone();
                                                            if let Some(e) = list.get_mut(env_idx) {
                                                                if var_idx < e.variables.len() {
                                                                    e.variables.remove(var_idx);
                                                                }
                                                            }
                                                            state.save_environments(list, cx);
                                                        });
                                                    })
                                                    .child("×"),
                                            )
                                    }))
                            })),
                    ),
            )
    }
}

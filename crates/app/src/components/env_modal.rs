use crate::state::WorkspaceState;
use crate::theme::theme;
use gpui::prelude::FluentBuilder as _;
use gpui::{
    div, px, AnyElement, App, AppContext as _, Entity, InteractiveElement as _, IntoElement, MouseButton,
    ParentElement, RenderOnce, StatefulInteractiveElement as _, Styled, Window,
};
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::menu::{ContextMenuExt as _, PopupMenuItem};
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
        let global_env = ws.global_environment.clone();
        let active_env_id = ws.active_environment_id.clone();
        let selected_id = ws
            .selected_env_modal_id
            .clone()
            .unwrap_or_else(|| "globals".to_string());
        let env_query = ws.env_filter_query.to_lowercase();
        let var_query = ws.var_filter_query.to_lowercase();
        let ws_clone = self.workspace_state.clone();
        let renaming_env_id = ws.renaming_env_id.clone();
        let rename_env_input = ws.rename_env_input.clone();

        let is_global_selected = selected_id == "globals";
        let active_custom_env = envs.iter().find(|e| e.id == selected_id).cloned();

        // 1. Env search input holder
        let env_search_holder = window.use_keyed_state(
            gpui::SharedString::from("env-modal-search-input"),
            cx,
            |window, cx| {
                let input = cx.new(|cx| InputState::new(window, cx).placeholder("Filter environments..."));
                let inp = input.clone();
                let ws_s = ws_clone.clone();
                let sub = cx.subscribe_in(&input, window, move |_, _, ev, _, cx| {
                    if let InputEvent::Change = ev {
                        let val = inp.read(cx).value().to_string();
                        ws_s.update(cx, |state, cx| {
                            state.set_env_filter_query(val, cx);
                        });
                    }
                });
                (input, sub)
            },
        );
        let env_search_input = env_search_holder.read(cx).0.clone();

        // 2. Variable search input holder
        let var_search_holder = window.use_keyed_state(
            gpui::SharedString::from(format!("var-modal-search-{}", selected_id)),
            cx,
            |window, cx| {
                let input = cx.new(|cx| InputState::new(window, cx).placeholder("Filter variables..."));
                let inp = input.clone();
                let ws_s = ws_clone.clone();
                let sub = cx.subscribe_in(&input, window, move |_, _, ev, _, cx| {
                    if let InputEvent::Change = ev {
                        let val = inp.read(cx).value().to_string();
                        ws_s.update(cx, |state, cx| {
                            state.set_var_filter_query(val, cx);
                        });
                    }
                });
                (input, sub)
            },
        );
        let var_search_input = var_search_holder.read(cx).0.clone();

        // Filtered environments for sidebar
        let filtered_envs: Vec<Environment> = envs
            .iter()
            .filter(|e| {
                if env_query.is_empty() {
                    true
                } else {
                    e.name.to_lowercase().contains(&env_query)
                }
            })
            .cloned()
            .collect();

        // Target variables to display in the right pane
        let (_target_env_name, target_variables): (String, Vec<(usize, EnvironmentVariable)>) = {
            if is_global_selected {
                let vars: Vec<_> = global_env
                    .variables
                    .iter()
                    .enumerate()
                    .filter(|(_, v)| {
                        if var_query.is_empty() {
                            true
                        } else {
                            v.key.to_lowercase().contains(&var_query)
                                || v.value.to_lowercase().contains(&var_query)
                        }
                    })
                    .map(|(i, v)| (i, v.clone()))
                    .collect();
                ("Globals".to_string(), vars)
            } else if let Some(custom) = &active_custom_env {
                let vars: Vec<_> = custom
                    .variables
                    .iter()
                    .enumerate()
                    .filter(|(_, v)| {
                        if var_query.is_empty() {
                            true
                        } else {
                            v.key.to_lowercase().contains(&var_query)
                                || v.value.to_lowercase().contains(&var_query)
                        }
                    })
                    .map(|(i, v)| (i, v.clone()))
                    .collect();
                (custom.name.clone(), vars)
            } else {
                ("No Environment Selected".to_string(), Vec::new())
            }
        };

        let total_vars_in_target = if is_global_selected {
            global_env.variables.len()
        } else if let Some(custom) = &active_custom_env {
            custom.variables.len()
        } else {
            0
        };

        let enabled_vars_in_target = if is_global_selected {
            global_env.variables.iter().filter(|v| v.enabled).count()
        } else if let Some(custom) = &active_custom_env {
            custom.variables.iter().filter(|v| v.enabled).count()
        } else {
            0
        };

        let is_target_active = !is_global_selected
            && active_custom_env.as_ref().map(|e| e.id.as_str()) == active_env_id.as_deref();

        div()
            .absolute()
            .inset_0()
            .bg(gpui::Hsla { a: 0.65, ..t.bg_app })
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .w(px(920.))
                    .h(px(580.))
                    .rounded(px(10.))
                    .bg(t.bg_card)
                    .border_1()
                    .border_color(t.border)
                    .shadow_xl()
                    .flex()
                    .flex_col()
                    .overflow_hidden()
                    // Modal Header
                    .child(
                        div()
                            .h(px(46.))
                            .px(px(16.))
                            .border_b_1()
                            .border_color(t.border)
                            .bg(t.bg_titlebar)
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
                                            .font_weight(gpui::FontWeight::BOLD)
                                            .text_color(t.text_primary)
                                            .child("Environments & Variables"),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(t.text_muted)
                                            .child("• Manage shared globals and custom environment configurations"),
                                    ),
                            )
                            .child({
                                let ws_s = ws_clone.clone();
                                div()
                                    .id("close-env-modal")
                                    .cursor_pointer()
                                    .px(px(6.))
                                    .py(px(2.))
                                    .rounded(px(4.))
                                    .text_color(t.text_muted)
                                    .hover(|s| s.bg(t.bg_hover).text_color(t.text_primary))
                                    .on_click(move |_, _, cx| {
                                        ws_s.update(cx, |state, cx| {
                                            state.toggle_env_modal(cx);
                                        });
                                    })
                                    .child("✕")
                            }),
                    )
                    // Modal Body: Sidebar (Left) + Detail Editor (Right)
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .overflow_hidden()
                            // LEFT SIDEBAR
                            .child(
                                div()
                                    .w(px(260.))
                                    .h_full()
                                    .border_r_1()
                                    .border_color(t.border)
                                    .bg(t.bg_sidebar)
                                    .flex()
                                    .flex_col()
                                    // Search environments
                                    .child(
                                        div()
                                            .p(px(8.))
                                            .border_b_1()
                                            .border_color(t.border)
                                            .child(
                                                div()
                                                    .h(px(28.))
                                                    .px(px(6.))
                                                    .rounded(px(4.))
                                                    .bg(t.bg_input)
                                                    .border_1()
                                                    .border_color(t.border)
                                                    .flex()
                                                    .items_center()
                                                    .child(
                                                        Input::new(&env_search_input)
                                                            .id("env-sidebar-search")
                                                            .appearance(false)
                                                            .bordered(false)
                                                            .focus_bordered(false),
                                                    ),
                                            ),
                                    )
                                    // Global Environment section
                                    .child(
                                        div()
                                            .px(px(8.))
                                            .pt(px(8.))
                                            .pb(px(4.))
                                            .flex()
                                            .flex_col()
                                            .gap(px(2.))
                                            .child(
                                                div()
                                                    .px(px(6.))
                                                    .py(px(2.))
                                                    .text_xs()
                                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                                    .text_color(t.text_muted)
                                                    .child("GLOBAL"),
                                            )
                                            .child({
                                                let ws_s = ws_clone.clone();
                                                let is_sel = is_global_selected;
                                                let count = global_env.variables.len();
                                                div()
                                                    .id("env-item-globals")
                                                    .px(px(8.))
                                                    .py(px(6.))
                                                    .rounded(px(5.))
                                                    .cursor_pointer()
                                                    .bg(if is_sel { t.bg_selected } else { gpui::transparent_black() })
                                                    .hover(|s| s.bg(if is_sel { t.bg_selected } else { t.bg_hover }))
                                                    .flex()
                                                    .items_center()
                                                    .justify_between()
                                                    .on_click(move |_, _, cx| {
                                                        ws_s.update(cx, |state, cx| {
                                                            state.select_modal_environment(Some("globals".to_string()), cx);
                                                        });
                                                    })
                                                    .child(
                                                        div()
                                                            .flex()
                                                            .items_center()
                                                            .gap(px(8.))
                                                            .child(div().text_xs().child("🌐"))
                                                            .child(
                                                                div()
                                                                    .text_xs()
                                                                    .font_weight(if is_sel { gpui::FontWeight::SEMIBOLD } else { gpui::FontWeight::NORMAL })
                                                                    .text_color(if is_sel { t.text_primary } else { t.text_secondary })
                                                                    .child("Global Variables"),
                                                            ),
                                                    )
                                                    .child(
                                                        div()
                                                            .px(px(6.))
                                                            .py(px(1.))
                                                            .rounded(px(10.))
                                                            .bg(t.bg_card)
                                                            .border_1()
                                                            .border_color(t.border)
                                                            .text_xs()
                                                            .text_color(t.text_muted)
                                                            .child(format!("{}", count)),
                                                    )
                                            }),
                                    )
                                    // Environments Section Header
                                    .child(
                                        div()
                                            .px(px(14.))
                                            .pt(px(10.))
                                            .pb(px(4.))
                                            .flex()
                                            .items_center()
                                            .justify_between()
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                                    .text_color(t.text_muted)
                                                    .child(format!("ENVIRONMENTS ({})", envs.len())),
                                            )
                                            .child({
                                                let ws_add = ws_clone.clone();
                                                div()
                                                    .id("btn-new-custom-env")
                                                    .cursor_pointer()
                                                    .px(px(6.))
                                                    .py(px(1.))
                                                    .rounded(px(3.))
                                                    .bg(t.primary)
                                                    .text_color(t.primary_foreground)
                                                    .hover(|s| s.bg(t.primary_hover))
                                                    .text_xs()
                                                    .font_weight(gpui::FontWeight::MEDIUM)
                                                    .on_click(move |_, _, cx| {
                                                        ws_add.update(cx, |state, cx| {
                                                            state.create_environment(None, cx);
                                                        });
                                                    })
                                                    .child("+ New")
                                            }),
                                    )
                                    // Custom environments scrollable list
                                    .child(
                                        div()
                                            .flex_1()
                                            .px(px(8.))
                                            .overflow_y_scrollbar()
                                            .flex()
                                            .flex_col()
                                            .gap(px(2.))
                                            .when(filtered_envs.is_empty(), |this| {
                                                this.child(
                                                    div()
                                                        .py(px(20.))
                                                        .flex()
                                                        .items_center()
                                                        .justify_center()
                                                        .text_xs()
                                                        .text_color(t.text_muted)
                                                        .child("No environments found"),
                                                )
                                            })
                                            .children(filtered_envs.into_iter().map(|env| {
                                                let is_sel = selected_id == env.id;
                                                let is_act = active_env_id.as_deref() == Some(&env.id);
                                                let is_renaming = renaming_env_id.as_deref() == Some(&env.id);
                                                let env_id = env.id.clone();
                                                let env_name = env.name.clone();
                                                let env_id_click = env.id.clone();
                                                let env_name_click = env.name.clone();
                                                let env_id_dup = env.id.clone();
                                                let env_id_del = env.id.clone();
                                                let count = env.variables.len();

                                                let ws_sel = ws_clone.clone();
                                                let ws_dup = ws_clone.clone();
                                                let ws_del = ws_clone.clone();

                                                div()
                                                    .id(gpui::SharedString::from(format!("env-sidebar-item-{}", env.id)))
                                                    .px(px(8.))
                                                    .py(px(6.))
                                                    .rounded(px(5.))
                                                    .cursor_pointer()
                                                    .bg(if is_sel { t.bg_selected } else { gpui::transparent_black() })
                                                    .hover(|s| s.bg(if is_sel { t.bg_selected } else { t.bg_hover }))
                                                    .flex()
                                                    .items_center()
                                                    .justify_between()
                                                    .on_click(move |ev, window, cx| {
                                                        if is_renaming {
                                                            return;
                                                        }
                                                        if ev.click_count() > 1 {
                                                            let cid = env_id_click.clone();
                                                            let cname = env_name_click.clone();
                                                            ws_sel.update(cx, |state, cx| {
                                                                state.start_renaming_environment(cid, cname, window, cx);
                                                            });
                                                        } else {
                                                            ws_sel.update(cx, |state, cx| {
                                                                state.select_modal_environment(Some(env_id_click.clone()), cx);
                                                            });
                                                        }
                                                    })
                                                    .context_menu({
                                                        let ws_ren = ws_clone.clone();
                                                        let ren_id = env.id.clone();
                                                        let ren_name = env.name.clone();
                                                        let ws_du = ws_clone.clone();
                                                        let du_id = env.id.clone();
                                                        let ws_de = ws_clone.clone();
                                                        let del_id = env.id.clone();

                                                        move |menu, _window, _cx| {
                                                            let ws_r = ws_ren.clone();
                                                            let r_id = ren_id.clone();
                                                            let r_name = ren_name.clone();
                                                            let ws_d = ws_du.clone();
                                                            let d_id = du_id.clone();
                                                            let ws_delete = ws_de.clone();
                                                            let id_delete = del_id.clone();

                                                            menu
                                                                .item(
                                                                    PopupMenuItem::new("Rename")
                                                                        .on_click(move |_, window, cx| {
                                                                            ws_r.update(cx, |state, cx| {
                                                                                state.start_renaming_environment(r_id.clone(), r_name.clone(), window, cx);
                                                                            });
                                                                        }),
                                                                )
                                                                .item(
                                                                    PopupMenuItem::new("Duplicate")
                                                                        .on_click(move |_, _, cx| {
                                                                            ws_d.update(cx, |state, cx| {
                                                                                state.duplicate_environment(&d_id, cx);
                                                                            });
                                                                        }),
                                                                )
                                                                .separator()
                                                                .item(
                                                                    PopupMenuItem::new("Delete")
                                                                        .on_click(move |_, _, cx| {
                                                                            ws_delete.update(cx, |state, cx| {
                                                                                state.delete_environment(&id_delete, cx);
                                                                            });
                                                                        }),
                                                                )
                                                        }
                                                    })
                                                    .child(
                                                        div()
                                                            .flex_1()
                                                            .min_w(px(0.))
                                                            .flex()
                                                            .items_center()
                                                            .gap(px(6.))
                                                            .overflow_hidden()
                                                            .child(
                                                                div()
                                                                    .text_xs()
                                                                    .text_color(if is_act { t.status_2xx } else { t.text_muted })
                                                                    .child(if is_act { "●" } else { "○" }),
                                                            )
                                                            .child(if is_renaming && rename_env_input.is_some() {
                                                                let inp = rename_env_input.as_ref().unwrap();
                                                                let ws_esc = ws_clone.clone();
                                                                div()
                                                                    .flex_1()
                                                                    .min_w(px(0.))
                                                                    .h(px(22.))
                                                                    .px(px(4.))
                                                                    .rounded(px(4.))
                                                                    .bg(t.bg_input)
                                                                    .border_1()
                                                                    .border_color(t.border_focus)
                                                                    .flex()
                                                                    .items_center()
                                                                    .on_mouse_down(MouseButton::Left, |_, _, cx| {
                                                                        cx.stop_propagation();
                                                                    })
                                                                    .on_key_down(move |event, _, cx| {
                                                                        if event.keystroke.key.as_str() == "escape" {
                                                                            ws_esc.update(cx, |state, cx| {
                                                                                state.cancel_renaming_environment(cx);
                                                                            });
                                                                        }
                                                                    })
                                                                    .child(
                                                                        Input::new(inp)
                                                                            .id(format!("rename-env-input-field-{}", env_id))
                                                                            .appearance(false)
                                                                            .bordered(false)
                                                                            .focus_bordered(false)
                                                                            .cleanable(false),
                                                                    )
                                                                    .into_any_element()
                                                            } else {
                                                                div()
                                                                    .truncate()
                                                                    .text_xs()
                                                                    .font_weight(if is_sel { gpui::FontWeight::SEMIBOLD } else { gpui::FontWeight::NORMAL })
                                                                    .text_color(if is_sel { t.text_primary } else { t.text_secondary })
                                                                    .child(env_name)
                                                                    .into_any_element()
                                                            }),
                                                    )
                                                    .child(
                                                        div()
                                                            .flex()
                                                            .items_center()
                                                            .gap(px(4.))
                                                            // Duplicate icon on hover
                                                            .child(
                                                                div()
                                                                    .id(gpui::SharedString::from(format!("dup-btn-{}", env_id)))
                                                                    .cursor_pointer()
                                                                    .px(px(4.))
                                                                    .py(px(1.))
                                                                    .rounded(px(3.))
                                                                    .hover(|s| s.bg(t.bg_card).text_color(t.text_primary))
                                                                    .text_color(t.text_muted)
                                                                    .text_xs()
                                                                    .on_click(move |_, _, cx| {
                                                                        cx.stop_propagation();
                                                                        ws_dup.update(cx, |state, cx| {
                                                                            state.duplicate_environment(&env_id_dup, cx);
                                                                        });
                                                                    })
                                                                    .child("❐"),
                                                            )
                                                            // Delete icon on hover
                                                            .child(
                                                                div()
                                                                    .id(gpui::SharedString::from(format!("del-btn-{}", env_id)))
                                                                    .cursor_pointer()
                                                                    .px(px(4.))
                                                                    .py(px(1.))
                                                                    .rounded(px(3.))
                                                                    .hover(|s| s.bg(t.bg_card).text_color(t.status_5xx))
                                                                    .text_color(t.text_muted)
                                                                    .text_xs()
                                                                    .on_click(move |_, _, cx| {
                                                                        cx.stop_propagation();
                                                                        ws_del.update(cx, |state, cx| {
                                                                            state.delete_environment(&env_id_del, cx);
                                                                        });
                                                                    })
                                                                    .child("×"),
                                                            )
                                                            // Count badge
                                                            .child(
                                                                div()
                                                                    .px(px(5.))
                                                                    .py(px(1.))
                                                                    .rounded(px(8.))
                                                                    .bg(t.bg_card)
                                                                    .border_1()
                                                                    .border_color(t.border)
                                                                    .text_xs()
                                                                    .text_color(t.text_muted)
                                                                    .child(format!("{}", count)),
                                                            ),
                                                    )
                                            })),
                                    )
                                    // Sidebar footer hint
                                    .child(
                                        div()
                                            .p(px(8.))
                                            .border_t_1()
                                            .border_color(t.border)
                                            .text_xs()
                                            .text_color(t.text_muted)
                                            .child("💡 Use {{variable}} in URLs, headers & scripts."),
                                    ),
                            )
                            // RIGHT DETAIL PANE
                            .child(
                                div()
                                    .flex_1()
                                    .h_full()
                                    .bg(t.bg_panel)
                                    .flex()
                                    .flex_col()
                                    // Detail Header
                                    .child(
                                        div()
                                            .h(px(54.))
                                            .px(px(16.))
                                            .border_b_1()
                                            .border_color(t.border)
                                            .bg(t.bg_card)
                                            .flex()
                                            .items_center()
                                            .justify_between()
                                            .child(
                                                // Left of header: Name + Active toggle
                                                div()
                                                    .flex()
                                                    .items_center()
                                                    .gap(px(10.))
                                                    .when(is_global_selected, |this| {
                                                        this.child(
                                                            div()
                                                                .flex()
                                                                .flex_col()
                                                                .gap(px(1.))
                                                                .child(
                                                                    div()
                                                                        .text_sm()
                                                                        .font_weight(gpui::FontWeight::BOLD)
                                                                        .text_color(t.text_primary)
                                                                        .child("🌐 Global Variables"),
                                                                )
                                                                .child(
                                                                    div()
                                                                        .text_xs()
                                                                        .text_color(t.text_muted)
                                                                        .child("Shared across all requests. Active environment overrides matching keys."),
                                                                ),
                                                        )
                                                    })
                                                    .when(!is_global_selected, |this| {
                                                        if let Some(custom) = &active_custom_env {
                                                            this.child(
                                                                div()
                                                                    .flex()
                                                                    .items_center()
                                                                    .gap(px(8.))
                                                                    .child(
                                                                        div()
                                                                            .text_sm()
                                                                            .font_weight(gpui::FontWeight::BOLD)
                                                                            .text_color(t.text_primary)
                                                                            .child(custom.name.clone()),
                                                                    )
                                                                    .when(is_target_active, |this| {
                                                                        this.child(
                                                                            div()
                                                                                .px(px(6.))
                                                                                .py(px(2.))
                                                                                .rounded(px(4.))
                                                                                .bg(gpui::Hsla { a: 0.15, ..t.status_2xx })
                                                                                .text_color(t.status_2xx)
                                                                                .text_xs()
                                                                                .font_weight(gpui::FontWeight::MEDIUM)
                                                                                .child("● Active"),
                                                                        )
                                                                    }),
                                                            )
                                                        } else {
                                                            this.child(div().text_sm().child("Select an environment"))
                                                        }
                                                    }),
                                            )
                                            .child(
                                                // Right of header: Add variable
                                                div()
                                                    .flex()
                                                    .items_center()
                                                    .gap(px(8.))
                                                    .child({
                                                        let ws_add = ws_clone.clone();
                                                        let is_glob = is_global_selected;
                                                        let cur_id = selected_id.clone();

                                                        div()
                                                            .id("btn-detail-add-variable")
                                                            .cursor_pointer()
                                                            .px(px(10.))
                                                            .py(px(4.))
                                                            .rounded(px(4.))
                                                            .bg(t.primary)
                                                            .text_color(t.primary_foreground)
                                                            .hover(|s| s.bg(t.primary_hover))
                                                            .text_xs()
                                                            .font_weight(gpui::FontWeight::MEDIUM)
                                                            .on_click(move |_, _, cx| {
                                                                ws_add.update(cx, |state, cx| {
                                                                    let new_var = EnvironmentVariable {
                                                                        enabled: true,
                                                                        key: "".to_string(),
                                                                        value: "".to_string(),
                                                                        secret: false,
                                                                    };
                                                                    if is_glob {
                                                                        let mut g = state.global_environment.clone();
                                                                        g.variables.push(new_var);
                                                                        state.save_global_environment(g, cx);
                                                                    } else {
                                                                        let mut list = state.environments.clone();
                                                                        if let Some(e) = list.iter_mut().find(|x| x.id == cur_id) {
                                                                            e.variables.push(new_var);
                                                                        }
                                                                        state.save_environments(list, cx);
                                                                    }
                                                                });
                                                            })
                                                            .child("+ Add Variable")
                                                    }),
                                            ),
                                    )
                                    // Variables Search & Summary Bar
                                    .child(
                                        div()
                                            .h(px(38.))
                                            .px(px(16.))
                                            .border_b_1()
                                            .border_color(t.border)
                                            .flex()
                                            .items_center()
                                            .justify_between()
                                            .child(
                                                div()
                                                    .w(px(240.))
                                                    .h(px(26.))
                                                    .px(px(6.))
                                                    .rounded(px(4.))
                                                    .bg(t.bg_input)
                                                    .border_1()
                                                    .border_color(t.border)
                                                    .flex()
                                                    .items_center()
                                                    .child(
                                                        Input::new(&var_search_input)
                                                            .id("var-detail-search")
                                                            .appearance(false)
                                                            .bordered(false)
                                                            .focus_bordered(false),
                                                    ),
                                            )
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(t.text_muted)
                                                    .child(format!(
                                                        "Showing {} of {} variables ({} enabled)",
                                                        target_variables.len(),
                                                        total_vars_in_target,
                                                        enabled_vars_in_target
                                                    )),
                                            ),
                                    )
                                    // Variables Table Container
                                    .child(
                                        div()
                                            .flex_1()
                                            .flex()
                                            .flex_col()
                                            .overflow_hidden()
                                            // Table Header
                                            .child(
                                                div()
                                                    .h(px(30.))
                                                    .px(px(16.))
                                                    .bg(t.bg_card)
                                                    .border_b_1()
                                                    .border_color(t.border)
                                                    .flex()
                                                    .items_center()
                                                    .gap(px(8.))
                                                    .text_xs()
                                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                                    .text_color(t.text_muted)
                                                    .child(div().w(px(24.)).child(""))
                                                    .child(div().w(px(210.)).child("VARIABLE NAME"))
                                                    .child(div().flex_1().child("INITIAL / CURRENT VALUE"))
                                                    .child(div().w(px(48.)).text_center().child("SECRET"))
                                                    .child(div().w(px(28.)).child("")),
                                            )
                                            // Table Rows
                                            .child(
                                                div()
                                                    .flex_1()
                                                    .px(px(12.))
                                                    .py(px(6.))
                                                    .overflow_y_scrollbar()
                                                    .flex()
                                                    .flex_col()
                                                    .gap(px(4.))
                                                    .when(target_variables.is_empty(), |this| {
                                                        this.child(
                                                            div()
                                                                .py(px(60.))
                                                                .flex()
                                                                .flex_col()
                                                                .items_center()
                                                                .justify_center()
                                                                .gap(px(8.))
                                                                .text_xs()
                                                                .text_color(t.text_muted)
                                                                .child("No variables match the current filter.")
                                                                .child(
                                                                    div()
                                                                        .text_xs()
                                                                        .text_color(t.primary)
                                                                        .child("Click '+ Add Variable' above to add a new variable."),
                                                                ),
                                                        )
                                                    })
                                                    .children(target_variables.into_iter().map(|(orig_idx, var)| {
                                                        let is_glob = is_global_selected;
                                                        let cur_id = selected_id.clone();
                                                        let row_id = format!("{}-row-{}", selected_id, orig_idx);
                                                        let v_key = var.key.clone();
                                                        let v_val = var.value.clone();
                                                        let enabled = var.enabled;
                                                        let secret = var.secret;

                                                        let ws_up_k = ws_clone.clone();
                                                        let ws_up_v = ws_clone.clone();
                                                        let ws_toggle = ws_clone.clone();
                                                        let ws_secret = ws_clone.clone();
                                                        let ws_del = ws_clone.clone();

                                                        let holder = window.use_keyed_state(
                                                            gpui::SharedString::from(format!("{}-holder", row_id)),
                                                            cx,
                                                            |window, cx| {
                                                                let k_in = cx.new(|cx| InputState::new(window, cx).placeholder("Variable key..."));
                                                                let v_in = cx.new(|cx| InputState::new(window, cx).placeholder("Variable value..."));

                                                                let k1 = k_in.clone();
                                                                let v1 = v_in.clone();
                                                                let cur_id1 = cur_id.clone();
                                                                let sub1 = cx.subscribe_in(&k_in, window, move |_, _, ev, _, cx| {
                                                                    if let InputEvent::Change = ev {
                                                                        let nk = k1.read(cx).value().to_string();
                                                                        let nv = v1.read(cx).value().to_string();
                                                                        let cid = cur_id1.clone();
                                                                        ws_up_k.update(cx, |state, cx| {
                                                                            if is_glob {
                                                                                let mut g = state.global_environment.clone();
                                                                                if let Some(vr) = g.variables.get_mut(orig_idx) {
                                                                                    vr.key = nk;
                                                                                    vr.value = nv;
                                                                                }
                                                                                state.save_global_environment(g, cx);
                                                                            } else {
                                                                                let mut list = state.environments.clone();
                                                                                if let Some(e) = list.iter_mut().find(|x| x.id == cid) {
                                                                                    if let Some(vr) = e.variables.get_mut(orig_idx) {
                                                                                        vr.key = nk;
                                                                                        vr.value = nv;
                                                                                    }
                                                                                }
                                                                                state.save_environments(list, cx);
                                                                            }
                                                                        });
                                                                    }
                                                                });

                                                                let k2 = k_in.clone();
                                                                let v2 = v_in.clone();
                                                                let cur_id2 = cur_id.clone();
                                                                let sub2 = cx.subscribe_in(&v_in, window, move |_, _, ev, _, cx| {
                                                                    if let InputEvent::Change = ev {
                                                                        let nk = k2.read(cx).value().to_string();
                                                                        let nv = v2.read(cx).value().to_string();
                                                                        let cid = cur_id2.clone();
                                                                        ws_up_v.update(cx, |state, cx| {
                                                                            if is_glob {
                                                                                let mut g = state.global_environment.clone();
                                                                                if let Some(vr) = g.variables.get_mut(orig_idx) {
                                                                                    vr.key = nk;
                                                                                    vr.value = nv;
                                                                                }
                                                                                state.save_global_environment(g, cx);
                                                                            } else {
                                                                                let mut list = state.environments.clone();
                                                                                if let Some(e) = list.iter_mut().find(|x| x.id == cid) {
                                                                                    if let Some(vr) = e.variables.get_mut(orig_idx) {
                                                                                        vr.key = nk;
                                                                                        vr.value = nv;
                                                                                    }
                                                                                }
                                                                                state.save_environments(list, cx);
                                                                            }
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
                                                            .px(px(4.))
                                                            .py(px(2.))
                                                            .gap(px(8.))
                                                            .rounded(px(4.))
                                                            .hover(|s| s.bg(t.bg_hover))
                                                            // Checkbox: enable/disable variable
                                                            .child({
                                                                let cur_id_c = cur_id.clone();
                                                                div()
                                                                    .id(gpui::SharedString::from(format!("{}-check", row_id)))
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
                                                                    .on_click(move |_, _, cx| {
                                                                        let cid = cur_id_c.clone();
                                                                        ws_toggle.update(cx, |state, cx| {
                                                                            if is_glob {
                                                                                let mut g = state.global_environment.clone();
                                                                                if let Some(vr) = g.variables.get_mut(orig_idx) {
                                                                                    vr.enabled = !enabled;
                                                                                }
                                                                                state.save_global_environment(g, cx);
                                                                            } else {
                                                                                let mut list = state.environments.clone();
                                                                                if let Some(e) = list.iter_mut().find(|x| x.id == cid) {
                                                                                    if let Some(vr) = e.variables.get_mut(orig_idx) {
                                                                                        vr.enabled = !enabled;
                                                                                    }
                                                                                }
                                                                                state.save_environments(list, cx);
                                                                            }
                                                                        });
                                                                    })
                                                                    .child(if enabled { "✓" } else { "" })
                                                            })
                                                            // Key Input
                                                            .child(
                                                                div()
                                                                    .w(px(210.))
                                                                    .h(px(28.))
                                                                    .px(px(6.))
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
                                                            // Value Input
                                                            .child(
                                                                div()
                                                                    .flex_1()
                                                                    .h(px(28.))
                                                                    .px(px(6.))
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
                                                            // Secret toggle button
                                                            .child({
                                                                let cur_id_s = cur_id.clone();
                                                                div()
                                                                    .id(gpui::SharedString::from(format!("{}-secret", row_id)))
                                                                    .w(px(48.))
                                                                    .h(px(24.))
                                                                    .rounded(px(4.))
                                                                    .bg(if secret { gpui::Hsla { a: 0.15, ..t.status_4xx } } else { gpui::transparent_black() })
                                                                    .hover(|s| s.bg(t.bg_card))
                                                                    .cursor_pointer()
                                                                    .flex()
                                                                    .items_center()
                                                                    .justify_center()
                                                                    .text_xs()
                                                                    .text_color(if secret { t.status_4xx } else { t.text_muted })
                                                                    .on_click(move |_, _, cx| {
                                                                        let cid = cur_id_s.clone();
                                                                        ws_secret.update(cx, |state, cx| {
                                                                            if is_glob {
                                                                                let mut g = state.global_environment.clone();
                                                                                if let Some(vr) = g.variables.get_mut(orig_idx) {
                                                                                    vr.secret = !secret;
                                                                                }
                                                                                state.save_global_environment(g, cx);
                                                                            } else {
                                                                                let mut list = state.environments.clone();
                                                                                if let Some(e) = list.iter_mut().find(|x| x.id == cid) {
                                                                                    if let Some(vr) = e.variables.get_mut(orig_idx) {
                                                                                        vr.secret = !secret;
                                                                                    }
                                                                                }
                                                                                state.save_environments(list, cx);
                                                                            }
                                                                        });
                                                                    })
                                                                    .child(if secret { "🔒 Secret" } else { "🔓 Visible" })
                                                            })
                                                            // Delete row button
                                                            .child({
                                                                let cur_id_d = cur_id.clone();
                                                                div()
                                                                    .id(gpui::SharedString::from(format!("{}-del", row_id)))
                                                                    .w(px(24.))
                                                                    .h(px(24.))
                                                                    .rounded(px(4.))
                                                                    .flex()
                                                                    .items_center()
                                                                    .justify_center()
                                                                    .cursor_pointer()
                                                                    .hover(|s| s.bg(t.bg_card).text_color(t.status_5xx))
                                                                    .text_color(t.text_muted)
                                                                    .text_xs()
                                                                    .on_click(move |_, _, cx| {
                                                                        let cid = cur_id_d.clone();
                                                                        ws_del.update(cx, |state, cx| {
                                                                            if is_glob {
                                                                                let mut g = state.global_environment.clone();
                                                                                if orig_idx < g.variables.len() {
                                                                                    g.variables.remove(orig_idx);
                                                                                }
                                                                                state.save_global_environment(g, cx);
                                                                            } else {
                                                                                let mut list = state.environments.clone();
                                                                                if let Some(e) = list.iter_mut().find(|x| x.id == cid) {
                                                                                    if orig_idx < e.variables.len() {
                                                                                        e.variables.remove(orig_idx);
                                                                                    }
                                                                                }
                                                                                state.save_environments(list, cx);
                                                                            }
                                                                        });
                                                                    })
                                                                    .child("×")
                                                            })
                                                    }))
                                                    .child({
                                                        let ws_add = ws_clone.clone();
                                                        let is_glob = is_global_selected;
                                                        let cur_id = selected_id.clone();
                                                        div()
                                                            .id("btn-table-add-row")
                                                            .mt(px(4.))
                                                            .py(px(6.))
                                                            .px(px(10.))
                                                            .rounded(px(4.))
                                                            .border_1()
                                                            .border_color(t.border)
                                                            .bg(t.bg_card)
                                                            .text_color(t.text_secondary)
                                                            .hover(|s| s.bg(t.bg_hover).border_color(t.primary).text_color(t.primary))
                                                            .cursor_pointer()
                                                            .flex()
                                                            .items_center()
                                                            .gap(px(6.))
                                                            .text_xs()
                                                            .on_click(move |_, _, cx| {
                                                                ws_add.update(cx, |state, cx| {
                                                                    let new_var = EnvironmentVariable {
                                                                        enabled: true,
                                                                        key: "".to_string(),
                                                                        value: "".to_string(),
                                                                        secret: false,
                                                                    };
                                                                    if is_glob {
                                                                        let mut g = state.global_environment.clone();
                                                                        g.variables.push(new_var);
                                                                        state.save_global_environment(g, cx);
                                                                    } else {
                                                                        let mut list = state.environments.clone();
                                                                        if let Some(e) = list.iter_mut().find(|x| x.id == cur_id) {
                                                                            e.variables.push(new_var);
                                                                        }
                                                                        state.save_environments(list, cx);
                                                                    }
                                                                });
                                                            })
                                                            .child("+ Add Variable Row")
                                                    }),
                                            ),
                                    ),
                            ),
                    ),
            )
    }
}

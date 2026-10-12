use crate::state::WorkspaceState;
use crate::theme::theme;
use gpui::prelude::FluentBuilder as _;
use gpui::{
    div, px, App, Entity, InteractiveElement as _, IntoElement, ParentElement,
    RenderOnce, StatefulInteractiveElement as _, Styled, Window,
};
use gpui_component::scroll::ScrollableElement as _;

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
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
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
                    .w(px(600.))
                    .h(px(420.))
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
                                    .text_sm()
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(t.text_primary)
                                    .child("Manage Environments"),
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
                            .children(envs.into_iter().map(|env| {
                                let is_active = active_env_id.as_deref() == Some(&env.id);
                                let env_id_click = env.id.clone();
                                let ws_s = ws_clone.clone();

                                div()
                                    .flex()
                                    .flex_col()
                                    .gap(px(6.))
                                    .p(px(8.))
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
                                                            state.set_active_environment(Some(env_id_click.clone()), cx);
                                                        });
                                                    })
                                                    .child(if is_active { "Selected" } else { "Activate" }),
                                            ),
                                    )
                                    // Variable rows
                                    .children(env.variables.iter().map(|var| {
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(8.))
                                            .text_xs()
                                            .font_family(".SystemMonoFont")
                                            .child(div().w(px(140.)).text_color(t.primary).child(var.key.clone()))
                                            .child(
                                                div()
                                                    .flex_1()
                                                    .text_color(t.text_secondary)
                                                    .child(if var.secret { "••••••••".to_string() } else { var.value.clone() }),
                                            )
                                    }))
                            })),
                    ),
            )
    }
}

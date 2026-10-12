use crate::state::{EditorState, WorkspaceState};
use crate::theme::theme;
use gpui::{
    div, px, App, Entity, InteractiveElement as _, IntoElement, ParentElement,
    RenderOnce, StatefulInteractiveElement as _, Styled, Window, WindowControlArea,
};

#[derive(IntoElement)]
pub struct AppTitleBar {
    workspace_state: Entity<WorkspaceState>,
    editor_state: Entity<EditorState>,
}

impl AppTitleBar {
    pub fn new(
        workspace_state: Entity<WorkspaceState>,
        editor_state: Entity<EditorState>,
    ) -> Self {
        Self {
            workspace_state,
            editor_state,
        }
    }
}

impl RenderOnce for AppTitleBar {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = theme();
        let ws = self.workspace_state.read(cx);
        let ed = self.editor_state.read(cx);

        let ws_name = ws
            .active_workspace
            .as_ref()
            .map(|w| w.name.clone())
            .unwrap_or_else(|| "api".to_string());

        let env_name = ws
            .active_environment()
            .map(|e| e.name.clone())
            .unwrap_or_else(|| "No Environment".to_string());

        let current_url = ed
            .active_request
            .as_ref()
            .map(|r| r.url.clone())
            .unwrap_or_else(|| "localhost:8002/api/tool-logs".to_string());

        let ws_clone = self.workspace_state.clone();
        let ed_clone = self.editor_state.clone();

        #[cfg(target_os = "macos")]
        let left_padding = px(78.);
        #[cfg(not(target_os = "macos"))]
        let left_padding = px(8.);

        let bar = div()
            .h(px(36.))
            .w_full()
            .bg(t.bg_titlebar)
            .border_b_1()
            .border_color(t.border)
            .flex()
            .items_center()
            .justify_between()
            .pl(left_padding)
            .pr(px(8.))
            .window_control_area(WindowControlArea::Drag);

        // Left section
        let left_section = div()
            .flex()
            .items_center()
            .gap(px(6.))
            // Sidebar toggle button
            .child({
                let ws_s = ws_clone.clone();
                div()
                    .id("btn-toggle-sidebar")
                    .px(px(6.))
                    .py(px(3.))
                    .rounded(px(4.))
                    .cursor_pointer()
                    .hover(|s| s.bg(t.bg_hover))
                    .text_color(t.text_secondary)
                    .on_click(move |_, _, cx| {
                        ws_s.update(cx, |state, cx| {
                            state.toggle_sidebar(cx);
                        });
                    })
                    .child("☰")
            })
            // New request (+) button
            .child({
                let ws_s = ws_clone.clone();
                let ed_s = ed_clone.clone();
                div()
                    .id("btn-new-request")
                    .px(px(6.))
                    .py(px(3.))
                    .rounded(px(4.))
                    .cursor_pointer()
                    .hover(|s| s.bg(t.bg_hover))
                    .text_color(t.text_secondary)
                    .on_click(move |_, _, cx| {
                        let new_req = ws_s.update(cx, |state, cx| {
                            state.create_request(None, cx)
                        });
                        ed_s.update(cx, |state, cx| {
                            state.set_active_request(new_req, true, cx);
                        });
                    })
                    .child("+")
            })
            // Cookie jar button
            .child({
                let ws_s = ws_clone.clone();
                div()
                    .id("btn-cookies")
                    .px(px(6.))
                    .py(px(3.))
                    .rounded(px(4.))
                    .cursor_pointer()
                    .hover(|s| s.bg(t.bg_hover))
                    .text_color(t.text_secondary)
                    .on_click(move |_, _, cx| {
                        ws_s.update(cx, |state, cx| {
                            state.toggle_cookies_modal(cx);
                        });
                    })
                    .child("🍪")
            })
            // Breadcrumb: "api > ● local"
            .child({
                let ws_s = ws_clone.clone();
                div()
                    .id("btn-env-breadcrumb")
                    .flex()
                    .items_center()
                    .gap(px(4.))
                    .px(px(6.))
                    .py(px(2.))
                    .rounded(px(4.))
                    .cursor_pointer()
                    .hover(|s| s.bg(t.bg_hover))
                    .on_click(move |_, _, cx| {
                        ws_s.update(cx, |state, cx| {
                            state.toggle_env_modal(cx);
                        });
                    })
                    .child(
                        div()
                            .text_xs()
                            .text_color(t.text_secondary)
                            .child(ws_name),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(t.text_muted)
                            .child(">"),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(t.status_2xx)
                            .child("●"),
                    )
                    .child(
                        div()
                            .text_xs()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(t.text_primary)
                            .child(env_name),
                    )
            });

        // Center section: current request URL
        let center_section = div()
            .flex_1()
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .text_xs()
                    .font_family(".SystemMonoFont")
                    .text_color(t.text_secondary)
                    .child(current_url),
            );

        // Right section: split toggle, search, settings, and caption buttons
        let mut right_section = div()
            .flex()
            .items_center()
            .gap(px(4.))
            // Split layout toggle button
            .child({
                let ws_s = ws_clone.clone();
                let is_vert = ws.split_layout_vertical;
                div()
                    .id("btn-split-toggle")
                    .px(px(6.))
                    .py(px(3.))
                    .rounded(px(4.))
                    .cursor_pointer()
                    .hover(|s| s.bg(t.bg_hover))
                    .text_color(t.text_secondary)
                    .on_click(move |_, _, cx| {
                        ws_s.update(cx, |state, cx| {
                            state.toggle_split_layout(cx);
                        });
                    })
                    .child(if is_vert { "⬒" } else { "◫" })
            })
            // Command palette search button
            .child({
                let ws_s = ws_clone.clone();
                div()
                    .id("btn-cmd-palette")
                    .px(px(6.))
                    .py(px(3.))
                    .rounded(px(4.))
                    .cursor_pointer()
                    .hover(|s| s.bg(t.bg_hover))
                    .text_color(t.text_secondary)
                    .on_click(move |_, _, cx| {
                        ws_s.update(cx, |state, cx| {
                            state.toggle_command_palette(cx);
                        });
                    })
                    .child("🔍")
            })
            // Settings button
            .child({
                let ws_s = ws_clone.clone();
                div()
                    .id("btn-settings")
                    .px(px(6.))
                    .py(px(3.))
                    .rounded(px(4.))
                    .cursor_pointer()
                    .hover(|s| s.bg(t.bg_hover))
                    .text_color(t.text_secondary)
                    .on_click(move |_, _, cx| {
                        ws_s.update(cx, |state, cx| {
                            state.toggle_settings_modal(cx);
                        });
                    })
                    .child("⚙")
            });

        #[cfg(not(target_os = "macos"))]
        {
            right_section = right_section.child(
                div()
                    .flex()
                    .items_center()
                    .ml(px(8.))
                    .child(
                        div()
                            .id("caption-min")
                            .w(px(32.))
                            .h(px(28.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .window_control_area(WindowControlArea::Min)
                            .hover(|s| s.bg(t.bg_hover))
                            .text_color(t.text_secondary)
                            .text_xs()
                            .child("─"),
                    )
                    .child(
                        div()
                            .id("caption-max")
                            .w(px(32.))
                            .h(px(28.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .window_control_area(WindowControlArea::Max)
                            .hover(|s| s.bg(t.bg_hover))
                            .text_color(t.text_secondary)
                            .text_xs()
                            .child("□"),
                    )
                    .child(
                        div()
                            .id("caption-close")
                            .w(px(32.))
                            .h(px(28.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .window_control_area(WindowControlArea::Close)
                            .hover(|s| s.bg(hex_to_hsla(0xe81123)).text_color(hex_to_hsla(0xffffff)))
                            .text_color(t.text_secondary)
                            .text_xs()
                            .child("✕"),
                    ),
            );
        }

        bar.child(left_section)
            .child(center_section)
            .child(right_section)
    }
}

fn hex_to_hsla(hex: u32) -> gpui::Hsla {
    let r = ((hex >> 16) & 0xFF) as f32 / 255.0;
    let g = ((hex >> 8) & 0xFF) as f32 / 255.0;
    let b = (hex & 0xFF) as f32 / 255.0;
    gpui::Hsla::from(gpui::Rgba { r, g, b, a: 1.0 })
}

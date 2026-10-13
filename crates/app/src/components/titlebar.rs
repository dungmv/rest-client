use crate::state::{EditorState, WorkspaceState};
use crate::theme::theme;
use gpui::{
    div, img, px, App, Entity, Image, ImageFormat, InteractiveElement as _, IntoElement,
    ParentElement, RenderOnce, StatefulInteractiveElement as _, Styled, Window, WindowControlArea,
};
use std::sync::{Arc, OnceLock};

fn app_icon_image() -> Arc<Image> {
    static ICON: OnceLock<Arc<Image>> = OnceLock::new();
    ICON.get_or_init(|| {
        Arc::new(Image {
            format: ImageFormat::Png,
            bytes: include_bytes!("../../resources/app_icon_64.png").to_vec(),
            id: 1,
        })
    })
    .clone()
}

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

        let current_title = ed
            .active_request
            .as_ref()
            .map(crate::state::effective_request_name)
            .unwrap_or_default();

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
            .overflow_hidden()
            .pl(left_padding);

        // Left section
        let left_section = div()
            .flex()
            .items_center()
            .flex_shrink_0()
            .gap(px(8.))
            // Sidebar toggle button
            .child({
                let ws_s = ws_clone.clone();
                div()
                    .id("btn-toggle-sidebar")
                    .occlude()
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
                    .child("◫")
            })
            // App badge pill: [icon + RestClient]
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .px(px(6.))
                    .py(px(2.))
                    .rounded(px(5.))
                    .bg(t.bg_card)
                    .border_1()
                    .border_color(t.border)
                    .child(
                        img(app_icon_image())
                            .w(px(16.))
                            .h(px(16.))
                            .rounded(px(3.))
                    )
                    .child(
                        div()
                            .text_xs()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(t.text_primary)
                            .child(format!("RestClient • {}", ws_name))
                    )
            )
            // Environment pill: [● local ▾]
            .child({
                let ws_s = ws_clone.clone();
                div()
                    .id("btn-env-breadcrumb")
                    .occlude()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .px(px(8.))
                    .py(px(2.))
                    .rounded(px(5.))
                    .bg(t.bg_card)
                    .border_1()
                    .border_color(t.border)
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
                            .text_color(t.status_2xx)
                            .child("●"),
                    )
                    .child(
                        div()
                            .text_xs()
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .text_color(t.text_primary)
                            .child(env_name),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(t.text_muted)
                            .child("▾"),
                    )
            })
            // Cookie jar button
            .child({
                let ws_s = ws_clone.clone();
                div()
                    .id("btn-cookies")
                    .occlude()
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
            // New request (+) button
            .child({
                let ws_s = ws_clone.clone();
                let ed_s = ed_clone.clone();
                div()
                    .id("btn-new-request")
                    .occlude()
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
            });

        // Center section: current request title (truncated domain / custom name)
        let center_section = div()
            .flex_1()
            .min_w(px(0.))
            .h_full()
            .px(px(8.))
            .overflow_hidden()
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .max_w(px(340.))
                    .overflow_hidden()
                    .truncate()
                    .text_xs()
                    .font_family(".SystemMonoFont")
                    .text_color(t.text_secondary)
                    .child(current_title),
            );

        // Right toolbar buttons: split toggle, search, settings
        let right_toolbar = div()
            .flex()
            .items_center()
            .flex_shrink_0()
            .gap(px(4.))
            .pr(px(8.))
            // Split layout toggle button
            .child({
                let ws_s = ws_clone.clone();
                let is_vert = ws.split_layout_vertical;
                div()
                    .id("btn-split-toggle")
                    .occlude()
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
                    .occlude()
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
                let ed_s = ed_clone.clone();
                div()
                    .id("btn-settings")
                    .occlude()
                    .px(px(6.))
                    .py(px(3.))
                    .rounded(px(4.))
                    .cursor_pointer()
                    .hover(|s| s.bg(t.bg_hover))
                    .text_color(t.text_secondary)
                    .on_click(move |_, _, cx| {
                        ed_s.update(cx, |state, cx| {
                            state.set_active_tab_kind(crate::state::RequestTabKind::Settings, cx);
                        });
                        ws_s.update(cx, |state, cx| {
                            state.toggle_settings_modal(cx);
                        });
                    })
                    .child("⚙")
            });

        let drag_region = div()
            .flex_1()
            .min_w(px(0.))
            .h_full()
            .overflow_hidden()
            .flex()
            .items_center()
            .justify_between()
            .window_control_area(WindowControlArea::Drag)
            .child(left_section)
            .child(center_section)
            .child(right_toolbar);

        #[allow(unused_mut)]
        let mut bar = bar.child(drag_region);

        #[cfg(not(target_os = "macos"))]
        {
            bar = bar.child(
                div()
                    .h_full()
                    .flex()
                    .items_center()
                    .flex_shrink_0()
                    .child(
                        div()
                            .id("caption-min")
                            .occlude()
                            .w(px(36.))
                            .h_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .window_control_area(WindowControlArea::Min)
                            .hover(|s| s.bg(t.bg_hover))
                            .text_color(t.text_secondary)
                            .text_xs()
                            .on_click(|_, window, cx| {
                                cx.stop_propagation();
                                window.minimize_window();
                            })
                            .child("─"),
                    )
                    .child(
                        div()
                            .id("caption-max")
                            .occlude()
                            .w(px(36.))
                            .h_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .window_control_area(WindowControlArea::Max)
                            .hover(|s| s.bg(t.bg_hover))
                            .text_color(t.text_secondary)
                            .text_xs()
                            .on_click(|_, window, cx| {
                                cx.stop_propagation();
                                window.zoom_window();
                            })
                            .child("□"),
                    )
                    .child(
                        div()
                            .id("caption-close")
                            .occlude()
                            .w(px(36.))
                            .h_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .window_control_area(WindowControlArea::Close)
                            .hover(|s| s.bg(hex_to_hsla(0xe81123)).text_color(hex_to_hsla(0xffffff)))
                            .text_color(t.text_secondary)
                            .text_xs()
                            .on_click(|_, window, cx| {
                                cx.stop_propagation();
                                window.remove_window();
                                cx.quit();
                            })
                            .child("✕"),
                    ),
            );
        }

        bar
    }
}

fn hex_to_hsla(hex: u32) -> gpui::Hsla {
    let r = ((hex >> 16) & 0xFF) as f32 / 255.0;
    let g = ((hex >> 8) & 0xFF) as f32 / 255.0;
    let b = (hex & 0xFF) as f32 / 255.0;
    gpui::Hsla::from(gpui::Rgba { r, g, b, a: 1.0 })
}

use crate::state::{EditorState, WorkspaceState};
use crate::theme::theme;
use gpui::{
    div, img, px, App, Entity, Image, ImageFormat, InteractiveElement as _, IntoElement,
    ParentElement, RenderOnce, StatefulInteractiveElement as _, Styled, Window, WindowControlArea,
};
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::menu::{DropdownMenu as _, PopupMenuItem};
use gpui_component::{Icon, IconName};
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
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = theme();
        let ws = self.workspace_state.read(cx);
        let ed = self.editor_state.read(cx);

        let folder_name = ws
            .workspace_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Workspace")
            .to_string();

        let ws_name = ws
            .active_workspace
            .as_ref()
            .map(|w| {
                if !w.name.trim().is_empty() {
                    w.name.clone()
                } else {
                    folder_name.clone()
                }
            })
            .unwrap_or(folder_name);

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
                let sidebar_collapsed = ws_clone.read(cx).sidebar_collapsed;
                div()
                    .id("btn-toggle-sidebar")
                    .occlude()
                    .px(px(6.))
                    .py(px(3.))
                    .rounded(px(4.))
                    .cursor_pointer()
                    .bg(if !sidebar_collapsed { t.bg_hover } else { gpui::transparent_black() })
                    .hover(|s| s.bg(t.bg_hover))
                    .text_color(if !sidebar_collapsed { t.text_primary } else { t.text_secondary })
                    .on_click(move |_, _, cx| {
                        ws_s.update(cx, |state, cx| {
                            state.toggle_sidebar(cx);
                        });
                    })
                    .child("◫")
            })
            // Workspace switcher pill dropdown: [icon + ws_name ▾]
            .child({
                let ws_s = ws_clone.clone();
                let recent = ws.recent_workspaces.clone();
                let ws_path = ws.workspace_path.clone();

                Button::new("btn-workspace-switcher")
                    .ghost()
                    .compact()
                    .px(px(6.))
                    .py(px(2.))
                    .h(px(24.))
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
                            .child(ws_name)
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(t.text_muted)
                            .child("▾")
                    )
                    .dropdown_menu(move |menu, _window, _cx| {
                        let ws_new = ws_s.clone();
                        let ws_open = ws_s.clone();
                        let ws_rename = ws_s.clone();
                        let ws_folder = ws_s.clone();
                        let ws_clean = ws_s.clone();
                        let ws_import = ws_s.clone();

                        let mut m = menu
                            .item(
                                PopupMenuItem::new("✏️ Rename Workspace...")
                                    .on_click(move |_, window, cx| {
                                        ws_rename.update(cx, |state, cx| {
                                            state.start_renaming_workspace(window, cx);
                                        });
                                    })
                            )
                            .item(
                                PopupMenuItem::new("📂 Open Folder")
                                    .on_click(move |_, _, cx| {
                                        ws_folder.update(cx, |state, _| {
                                            state.open_workspace_folder();
                                        });
                                    })
                            )
                            .item(
                                PopupMenuItem::new("🧹 Clean History")
                                    .on_click(move |_, _, cx| {
                                        ws_clean.update(cx, |state, cx| {
                                            state.clean_history(cx);
                                        });
                                    })
                            )
                            .item(
                                PopupMenuItem::new("📥 Import...")
                                    .on_click(move |_, _, cx| {
                                        ws_import.update(cx, |state, cx| {
                                            state.open_import_modal(None, cx);
                                        });
                                    })
                            )
                            .separator()
                            .item(
                                PopupMenuItem::new("📁 Open Workspace...")
                                    .on_click(move |_, _, cx| {
                                        if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                                            ws_open.update(cx, |state, cx| {
                                                state.open_workspace(folder, cx);
                                            });
                                        }
                                    })
                            )
                            .item(
                                PopupMenuItem::new("✨ New Workspace...")
                                    .on_click(move |_, _, cx| {
                                        if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                                            let name = folder
                                                .file_name()
                                                .and_then(|n| n.to_str())
                                                .unwrap_or("Workspace")
                                                .to_string();
                                            ws_new.update(cx, |state, cx| {
                                                state.new_workspace(folder, &name, cx);
                                            });
                                        }
                                    })
                            );

                        if !recent.is_empty() {
                            m = m.separator();
                            for r_path_str in &recent {
                                let r_path = std::path::PathBuf::from(r_path_str);
                                let display_name = r_path
                                    .file_name()
                                    .and_then(|n| n.to_str())
                                    .unwrap_or(r_path_str.as_str())
                                    .to_string();
                                let ws_r = ws_s.clone();
                                let is_curr = r_path == ws_path;
                                let r_path_clone = r_path.clone();

                                m = m.item(
                                    PopupMenuItem::new(format!("{} ({})", display_name, r_path_str))
                                        .checked(is_curr)
                                        .on_click(move |_, _, cx| {
                                            ws_r.update(cx, |state, cx| {
                                                state.open_workspace(r_path_clone.clone(), cx);
                                            });
                                        })
                                );
                            }
                        }

                        m
                    })
            })
            // Environment pill dropdown: [● local ▾]
            .child({
                let ws_s = ws_clone.clone();
                let envs = ws.environments.clone();
                let active_env_id = ws.active_environment_id.clone();
                let has_active = ws.active_environment().is_some();

                Button::new("btn-env-breadcrumb")
                    .ghost()
                    .compact()
                    .px(px(8.))
                    .py(px(2.))
                    .h(px(24.))
                    .rounded(px(5.))
                    .bg(t.bg_card)
                    .border_1()
                    .border_color(t.border)
                    .child(
                        div()
                            .text_xs()
                            .text_color(if has_active { t.status_2xx } else { t.text_muted })
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
                    .dropdown_menu(move |menu, _window, _cx| {
                        let mut m = menu;

                        // 1. "No Environment" option
                        let ws_none = ws_s.clone();
                        let is_none_active = active_env_id.is_none();
                        m = m.item(
                            PopupMenuItem::new("No Environment")
                                .checked(is_none_active)
                                .on_click(move |_, _, cx| {
                                    ws_none.update(cx, |state, cx| {
                                        state.set_active_environment(None, cx);
                                    });
                                }),
                        );

                        // 2. All workspace environments
                        for env in &envs {
                            let env_id = env.id.clone();
                            let is_act = active_env_id.as_deref() == Some(&env_id);
                            let ws_env = ws_s.clone();
                            m = m.item(
                                PopupMenuItem::new(env.name.clone())
                                    .checked(is_act)
                                    .on_click(move |_, _, cx| {
                                        ws_env.update(cx, |state, cx| {
                                            state.set_active_environment(Some(env_id.clone()), cx);
                                        });
                                    }),
                            );
                        }

                        // 3. Separator and "Environment Settings..."
                        let ws_settings = ws_s.clone();
                        m = m.separator().item(
                            PopupMenuItem::new("⚙ Environment Settings...")
                                .on_click(move |_, _, cx| {
                                    ws_settings.update(cx, |state, cx| {
                                        state.show_env_modal = true;
                                        cx.notify();
                                    });
                                }),
                        );

                        m
                    })
            });

        // Center section: search button + current request name
        let center_section = div()
            .flex_1()
            .min_w(px(0.))
            .h_full()
            .px(px(8.))
            .overflow_hidden()
            .flex()
            .items_center()
            .justify_center()
            .child({
                let ws_s = ws_clone.clone();
                let has_title = !current_title.trim().is_empty();
                let label = if has_title {
                    current_title.clone()
                } else {
                    "Search requests, commands...".to_string()
                };

                div()
                    .id("btn-titlebar-search")
                    .occlude()
                    .h(px(24.))
                    .w(px(320.))
                    .max_w(px(400.))
                    .px(px(8.))
                    .rounded(px(5.))
                    .bg(t.bg_card)
                    .border_1()
                    .border_color(t.border)
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(6.))
                    .cursor_pointer()
                    .hover(|s| s.bg(t.bg_hover).border_color(t.border_focus))
                    .on_click(move |_, _, cx| {
                        ws_s.update(cx, |state, cx| {
                            state.toggle_command_palette(cx);
                        });
                    })
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .min_w(px(0.))
                            .flex_1()
                            .overflow_hidden()
                            .child(
                                div()
                                    .truncate()
                                    .text_xs()
                                    .font_family(".SystemMonoFont")
                                    .text_color(if has_title { t.text_primary } else { t.text_muted })
                                    .child(label),
                            ),
                    )
                    .child(
                        div()
                            .flex_shrink_0()
                            .px(px(4.))
                            .py(px(1.))
                            .rounded(px(3.))
                            .bg(t.bg_input)
                            .border_1()
                            .border_color(t.border)
                            .text_color(t.text_muted)
                            .font_family(".SystemMonoFont")
                            .text_size(px(10.))
                            .child("Ctrl+K"),
                    )
            });

        // Right toolbar buttons: split toggle, settings
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
            // Settings button
            .child({
                let ws_s = ws_clone.clone();
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
            let is_max = _window.is_maximized();
            let icon_color = t.text_primary;

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
                            .w(px(46.))
                            .h_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .window_control_area(WindowControlArea::Min)
                            .hover(|s| s.bg(t.bg_hover))
                            .text_color(icon_color)
                            .on_click(|_, window, cx| {
                                cx.stop_propagation();
                                window.minimize_window();
                            })
                            .child(Icon::new(IconName::WindowMinimize).size(px(16.))),
                    )
                    .child(
                        div()
                            .id("caption-max")
                            .occlude()
                            .w(px(46.))
                            .h_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .window_control_area(WindowControlArea::Max)
                            .hover(|s| s.bg(t.bg_hover))
                            .text_color(icon_color)
                            .on_click(|_, window, cx| {
                                cx.stop_propagation();
                                window.zoom_window();
                            })
                            .child(if is_max {
                                Icon::new(IconName::WindowRestore).size(px(16.))
                            } else {
                                Icon::new(IconName::WindowMaximize).size(px(16.))
                            }),
                    )
                    .child(
                        div()
                            .id("caption-close")
                            .occlude()
                            .w(px(46.))
                            .h_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .window_control_area(WindowControlArea::Close)
                            .hover(|s| s.bg(hex_to_hsla(0xe81123)).text_color(hex_to_hsla(0xffffff)))
                            .text_color(icon_color)
                            .on_click(|_, window, cx| {
                                cx.stop_propagation();
                                window.remove_window();
                                cx.quit();
                            })
                            .child(Icon::new(IconName::WindowClose).size(px(16.))),
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

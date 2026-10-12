use crate::components::status_badge::{MethodBadge, StatusCodeBadge};
use crate::state::{EditorState, WorkspaceState};
use crate::theme::theme;
use gpui::prelude::FluentBuilder as _;
use gpui::{
    div, px, App, Entity, InteractiveElement as _, IntoElement, ParentElement,
    RenderOnce, StatefulInteractiveElement as _, Styled, Window,
};
use gpui_component::scroll::ScrollableElement as _;

#[derive(IntoElement)]
pub struct SidebarTree {
    workspace_state: Entity<WorkspaceState>,
    editor_state: Entity<EditorState>,
}

impl SidebarTree {
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

impl RenderOnce for SidebarTree {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = theme();
        let ws = self.workspace_state.read(cx);
        let ed = self.editor_state.read(cx);

        let active_req_id = ed.active_request.as_ref().map(|r| r.id.clone());
        let git_branch = ws.git_branch.clone().unwrap_or_else(|| "main".to_string());
        let query = ws.filter_query.to_lowercase();

        let ws_clone = self.workspace_state.clone();
        let ed_clone = self.editor_state.clone();

        div()
            .w_full()
            .h_full()
            .bg(t.bg_sidebar)
            .border_r_1()
            .border_color(t.border)
            .flex()
            .flex_col()
            .justify_between()
            // Top section: search bar + overflow menu
            .child(
                div()
                    .p(px(8.))
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .child(
                        div()
                            .flex_1()
                            .h(px(28.))
                            .px(px(8.))
                            .rounded(px(6.))
                            .bg(t.bg_input)
                            .border_1()
                            .border_color(t.border_subtle)
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(t.text_muted)
                                    .child("🔍"),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(if ws.filter_query.is_empty() {
                                        t.text_muted
                                    } else {
                                        t.text_primary
                                    })
                                    .child(if ws.filter_query.is_empty() {
                                        "Search".to_string()
                                    } else {
                                        ws.filter_query.clone()
                                    }),
                            ),
                    )
                    // Overflow menu (⋮)
                    .child({
                        let ws_s = ws_clone.clone();
                        div()
                            .id("btn-sidebar-overflow")
                            .w(px(28.))
                            .h(px(28.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(px(4.))
                            .cursor_pointer()
                            .hover(|s| s.bg(t.bg_hover))
                            .text_color(t.text_secondary)
                            .on_click(move |_, _, cx| {
                                ws_s.update(cx, |state, cx| {
                                    state.create_folder("new-folder".to_string(), cx);
                                });
                            })
                            .child("⋮")
                    }),
            )
            // Middle section: Folders & Requests tree
            .child(
                div()
                    .flex_1()
                    .overflow_y_scrollbar()
                    .px(px(6.))
                    .py(px(4.))
                    .flex()
                    .flex_col()
                    .gap(px(2.))
                    .children(ws.folders.iter().map(|folder| {
                        let f_id = folder.id.clone();
                        let folder_requests: Vec<_> = ws
                            .requests
                            .iter()
                            .filter(|r| {
                                r.folder_id.as_deref() == Some(&f_id)
                                    && (query.is_empty()
                                        || r.name.to_lowercase().contains(&query)
                                        || r.url.to_lowercase().contains(&query))
                            })
                            .cloned()
                            .collect();

                        div()
                            .flex()
                            .flex_col()
                            .gap(px(2.))
                            // Folder header
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(6.))
                                    .px(px(6.))
                                    .py(px(4.))
                                    .rounded(px(4.))
                                    .text_color(t.text_secondary)
                                    .text_xs()
                                    .child(div().child("⌄"))
                                    .child(div().child("📁"))
                                    .child(
                                        div()
                                            .font_weight(gpui::FontWeight::SEMIBOLD)
                                            .text_color(t.status_2xx)
                                            .child(folder.name.clone()),
                                    ),
                            )
                            // Folder items
                            .children(folder_requests.into_iter().map(|req| {
                                let is_selected = active_req_id.as_ref() == Some(&req.id);
                                let req_for_click = req.clone();
                                let ed_s = ed_clone.clone();

                                div()
                                    .id(format!("req-item-{}", req.id))
                                    .ml(px(14.))
                                    .px(px(6.))
                                    .py(px(4.))
                                    .rounded(px(6.))
                                    .border_1()
                                    .border_color(if is_selected {
                                        t.border
                                    } else {
                                        t.transparent
                                    })
                                    .bg(if is_selected {
                                        t.bg_selected
                                    } else {
                                        t.transparent
                                    })
                                    .hover(|s| s.bg(t.bg_hover))
                                    .cursor_pointer()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .on_click(move |ev, _, cx| {
                                        let pin = ev.click_count() > 1;
                                        ed_s.update(cx, |state, cx| {
                                            state.set_active_request(req_for_click.clone(), pin, cx);
                                        });
                                    })
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(6.))
                                            .overflow_hidden()
                                            .child(MethodBadge::new(req.method))
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .font_family(".SystemMonoFont")
                                                    .text_color(t.status_2xx)
                                                    .truncate()
                                                    .child(req.name.clone()),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(4.))
                                            .when_some(req.last_status_code, |this, code| {
                                                this.child(StatusCodeBadge::new(code))
                                            }),
                                    )
                            }))
                    }))
                    // Top-level requests without folder
                    .children(
                        ws.requests
                            .iter()
                            .filter(|r| {
                                r.folder_id.is_none()
                                    && (query.is_empty()
                                        || r.name.to_lowercase().contains(&query)
                                        || r.url.to_lowercase().contains(&query))
                            })
                            .map(|req| {
                                let is_selected = active_req_id.as_ref() == Some(&req.id);
                                let req_for_click = req.clone();
                                let ed_s = ed_clone.clone();

                                div()
                                    .id(format!("req-top-{}", req.id))
                                    .px(px(6.))
                                    .py(px(4.))
                                    .rounded(px(6.))
                                    .border_1()
                                    .border_color(if is_selected {
                                        t.border
                                    } else {
                                        t.transparent
                                    })
                                    .bg(if is_selected {
                                        t.bg_selected
                                    } else {
                                        t.transparent
                                    })
                                    .hover(|s| s.bg(t.bg_hover))
                                    .cursor_pointer()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .on_click(move |ev, _, cx| {
                                        let pin = ev.click_count() > 1;
                                        ed_s.update(cx, |state, cx| {
                                            state.set_active_request(req_for_click.clone(), pin, cx);
                                        });
                                    })
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(6.))
                                            .child(MethodBadge::new(req.method))
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .font_family(".SystemMonoFont")
                                                    .text_color(t.text_primary)
                                                    .child(req.name.clone()),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .when_some(req.last_status_code, |this, code| {
                                                this.child(StatusCodeBadge::new(code))
                                            }),
                                    )
                            }),
                    ),
            )
            // Footer: git branch chip
            .child(
                div()
                    .h(px(32.))
                    .px(px(10.))
                    .border_t_1()
                    .border_color(t.border)
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .text_xs()
                    .text_color(t.text_muted)
                    .child("⎇")
                    .child(
                        div()
                            .text_color(t.text_secondary)
                            .font_family(".SystemMonoFont")
                            .child(git_branch),
                    ),
            )
    }
}

use super::auth_tab::{render_auth_tab, AuthTabInputs};
use super::sync::save_active_request_without_redraw;
use super::AppView;
use crate::components::*;
use crate::state::*;
use crate::theme::theme;
use gpui::prelude::FluentBuilder as _;
use gpui::{
    div, px, AnyElement, Context, InteractiveElement as _, IntoElement, ParentElement,
    StatefulInteractiveElement as _, Styled, Window,
};
use gpui_component::Icon;
use gpui_kit::assets::IconName;
use restclient_core::models::{Auth, HttpMethod};

impl AppView {
    pub(crate) fn render_editor_pane(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let t = theme();
        let active_tab_kind = self.editor_state.read(cx).active_tab_kind;
        let method_dropdown_open = self.editor_state.read(cx).method_dropdown_open;
        let (active_req_id, active_req_info) = {
            let ed = self.editor_state.read(cx);
            let id = ed.active_request.as_ref().map(|r| r.id.clone());
            let info = ed.active_request.as_ref().map(|r| {
                (
                    r.params.len(),
                    r.headers.len(),
                    r.script
                        .as_ref()
                        .and_then(|s| s.pre_request.as_ref())
                        .map(|s| !s.trim().is_empty())
                        .unwrap_or(false),
                    r.script
                        .as_ref()
                        .and_then(|s| s.post_response.as_ref())
                        .map(|s| !s.trim().is_empty())
                        .unwrap_or(false),
                )
            });
            (id, info)
        };

        let ws_entity = self.workspace_state.clone();
        let ed_entity = self.editor_state.clone();
        let per_entity = self.persistence_state.clone();
        let env_entity = self.env_state.clone();
        let app_view_entity = cx.entity().clone();
        let req_id_str = active_req_id.unwrap_or_else(|| "scratch".to_string());
        let ws_read = ws_entity.read(cx);
        let per_read = per_entity.read(cx);
        let env_read = env_entity.read(cx);
        let active_conflict = per_read.conflicts.get(&req_id_str).cloned();
        let active_parse_err = ws_read.parse_errors.get(&req_id_str).cloned();
        let corrupt_sec_err = env_read.corrupt_secrets_warning.clone();

        div()
            .size_full()
            .bg(t.bg_editor)
            .relative()
            .flex()
            .flex_col()
            // Conflict banner
            .when_some(active_conflict, |this, msg| {
                let per_rel = per_entity.clone();
                let per_keep = per_entity.clone();
                let ws_rel = ws_entity.clone();
                let ws_keep = ws_entity.clone();
                let cid = req_id_str.clone();
                let cid_k = req_id_str.clone();
                this.child(
                    div()
                        .px(px(12.))
                        .py(px(6.))
                        .bg(gpui::Hsla {
                            a: 0.15,
                            ..t.status_4xx
                        })
                        .border_b_1()
                        .border_color(t.status_4xx)
                        .flex()
                        .items_center()
                        .justify_between()
                        .text_xs()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(8.))
                                .child(
                                    div()
                                        .font_weight(gpui::FontWeight::BOLD)
                                        .text_color(t.status_4xx)
                                        .child("! Conflict:"),
                                )
                                .child(div().text_color(t.text_primary).child(msg)),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(6.))
                                .child(
                                    div()
                                        .id("btn-conflict-reload")
                                        .cursor_pointer()
                                        .px(px(8.))
                                        .py(px(2.))
                                        .rounded(px(4.))
                                        .bg(t.primary)
                                        .text_color(t.primary_foreground)
                                        .on_click(move |_, _, cx| {
                                            per_rel.update(cx, |per, cx| {
                                                per.resolve_conflict_reload(&cid, ws_rel.clone(), cx);
                                            });
                                        })
                                        .child("Reload"),
                                )
                                .child(
                                    div()
                                        .id("btn-conflict-keep")
                                        .cursor_pointer()
                                        .px(px(8.))
                                        .py(px(2.))
                                        .rounded(px(4.))
                                        .bg(t.bg_card)
                                        .border_1()
                                        .border_color(t.border)
                                        .text_color(t.text_primary)
                                        .on_click(move |_, _, cx| {
                                            per_keep.update(cx, |per, cx| {
                                                per.resolve_conflict_keep(&cid_k, ws_keep.clone(), cx);
                                            });
                                        })
                                        .child("Keep mine"),
                                ),
                        ),
                )
            })
            // Parse error banner
            .when_some(active_parse_err, |this, err| {
                this.child(
                    div()
                        .px(px(12.))
                        .py(px(6.))
                        .bg(gpui::Hsla {
                            a: 0.12,
                            ..t.status_5xx
                        })
                        .border_b_1()
                        .border_color(t.status_5xx)
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .text_xs()
                        .text_color(t.status_5xx)
                        .child("! Parse error on disk (showing last good in-memory version):")
                        .child(
                            div()
                                .font_family("monospace")
                                .text_color(t.text_primary)
                                .child(err),
                        ),
                )
            })
            // Corrupt secrets banner
            .when_some(corrupt_sec_err, |this, msg| {
                this.child(
                    div()
                        .px(px(12.))
                        .py(px(6.))
                        .bg(gpui::Hsla {
                            a: 0.15,
                            ..t.status_4xx
                        })
                        .border_b_1()
                        .border_color(t.status_4xx)
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .text_xs()
                        .text_color(t.status_4xx)
                        .child(format!("! Sensitive Data Warning: {}", msg)),
                )
            })
            // Top Request Bar: Method + URL + Send
            .child({
                let view_handle = app_view_entity.clone();
                RequestBar::new(
                    self.env_state.clone(),
                    self.modal_state.clone(),
                    ed_entity.clone(),
                    self.url_input.clone(),
                    move |_, cx| {
                        view_handle.update(cx, |view, cx| {
                            view.send_current_request(cx);
                        });
                    },
                )
            })
            // Request Tabs with badges: Body v, Params [count], Headers [count], Auth, Settings
            .child(
                div()
                    .h(px(34.))
                    .px(px(8.))
                    .border_b_1()
                    .border_color(t.border)
                    .flex()
                    .items_center()
                    .gap(px(4.))
                    .child({
                        let ed_c = ed_entity.clone();
                        TabItem::new("tab-body", "Body")
                            .active(active_tab_kind == RequestTabKind::Body)
                            .on_click(move |_, _, cx| {
                                ed_c.update(cx, |state, cx| {
                                    state.set_active_tab_kind(RequestTabKind::Body, cx);
                                });
                            })
                    })
                    .child({
                        let ed_c = ed_entity.clone();
                        let param_count = active_req_info.as_ref().map(|info| info.0).unwrap_or(0);
                        TabItem::new("tab-params", "Params")
                            .active(active_tab_kind == RequestTabKind::Params)
                            .badge(format!("{}", param_count))
                            .on_click(move |_, _, cx| {
                                ed_c.update(cx, |state, cx| {
                                    state.set_active_tab_kind(RequestTabKind::Params, cx);
                                });
                            })
                    })
                    .child({
                        let ed_c = ed_entity.clone();
                        let header_count = active_req_info.as_ref().map(|info| info.1).unwrap_or(0);
                        TabItem::new("tab-headers", "Headers")
                            .active(active_tab_kind == RequestTabKind::Headers)
                            .badge(format!("{}", header_count))
                            .on_click(move |_, _, cx| {
                                ed_c.update(cx, |state, cx| {
                                    state.set_active_tab_kind(RequestTabKind::Headers, cx);
                                });
                            })
                    })
                    .child({
                        let ed_c = ed_entity.clone();
                        TabItem::new("tab-auth", "Auth")
                            .active(active_tab_kind == RequestTabKind::Auth)
                            .on_click(move |_, _, cx| {
                                ed_c.update(cx, |state, cx| {
                                    state.set_active_tab_kind(RequestTabKind::Auth, cx);
                                });
                            })
                    })
                    .child({
                        let ed_c = ed_entity.clone();
                        let has_pre = active_req_info.as_ref().map(|info| info.2).unwrap_or(false);
                        TabItem::new("tab-pre-script", "Pre-request")
                            .active(active_tab_kind == RequestTabKind::PreRequestScript)
                            .when(has_pre, |tab| tab.badge("*"))
                            .on_click(move |_, _, cx| {
                                ed_c.update(cx, |state, cx| {
                                    state.set_active_tab_kind(RequestTabKind::PreRequestScript, cx);
                                });
                            })
                    })
                    .child({
                        let ed_c = ed_entity.clone();
                        let has_test = active_req_info.as_ref().map(|info| info.3).unwrap_or(false);
                        TabItem::new("tab-tests", "Post-response")
                            .active(active_tab_kind == RequestTabKind::Tests)
                            .when(has_test, |tab| tab.badge("*"))
                            .on_click(move |_, _, cx| {
                                ed_c.update(cx, |state, cx| {
                                    state.set_active_tab_kind(RequestTabKind::Tests, cx);
                                });
                            })
                    }),
            )
            // Request Tab Content
            .child(div().flex_1().overflow_hidden().child({
                match active_tab_kind {
                    RequestTabKind::Body => div().size_full().child(
                        BodyEditor::new(
                            ed_entity.clone(),
                            self.json_editor.clone(),
                            self.xml_editor.clone(),
                            self.raw_textarea.clone(),
                        )
                        .workspace_state(ws_entity.clone())
                        .persistence_state(self.persistence_state.clone())
                        .env_state(self.env_state.clone())
                        .toast_state(self.toast_state.clone()),
                    ),
                    RequestTabKind::Params => {
                        let params = ed_entity
                            .read(cx)
                            .active_request
                            .as_ref()
                            .map(|r| r.params.clone())
                            .unwrap_or_default();
                        let ed_up = ed_entity.clone();
                        let ws_up = ws_entity.clone();
                        let per_up = self.persistence_state.clone();
                        let ed_add = ed_entity.clone();
                        let ed_del = ed_entity.clone();
                        div().size_full().child(
                            KvTable::new(
                                format!("params-kv-{}", req_id_str),
                                params,
                                move |idx, k, v, enabled, _, cx| {
                                    ed_up.update(cx, |state, cx| {
                                        state.update_param(idx, k, v, enabled, cx);
                                    });
                                    save_active_request_without_redraw(&ed_up, &per_up, &ws_up, cx);
                                },
                                move |_, cx| {
                                    ed_add.update(cx, |state, cx| {
                                        state.add_param(cx);
                                    });
                                },
                                move |idx, _, cx| {
                                    ed_del.update(cx, |state, cx| {
                                        state.remove_param(idx, cx);
                                    });
                                },
                            )
                            .env_state(self.env_state.clone())
                            .modal_state(self.modal_state.clone()),
                        )
                    }
                    RequestTabKind::Headers => {
                        let headers = ed_entity
                            .read(cx)
                            .active_request
                            .as_ref()
                            .map(|r| r.headers.clone())
                            .unwrap_or_default();
                        let inherited_count = {
                            let active_r = ed_entity.read(cx).active_request.clone();
                            if let Some(r) = active_r {
                                let ws_r = ws_entity.read(cx);
                                let ws_h = ws_r
                                    .active_workspace
                                    .as_ref()
                                    .map(|w| w.headers.len())
                                    .unwrap_or(0);
                                let folder_h: usize = ws_r
                                    .get_folder_hierarchy(r.folder_id.as_deref())
                                    .iter()
                                    .map(|f| f.headers.len())
                                    .sum();
                                ws_h + folder_h
                            } else {
                                0
                            }
                        };
                        let ed_up = ed_entity.clone();
                        let ws_up = ws_entity.clone();
                        let per_up = self.persistence_state.clone();
                        let ed_add = ed_entity.clone();
                        let ed_del = ed_entity.clone();
                        div()
                            .size_full()
                            .flex()
                            .flex_col()
                            .when(inherited_count > 0, |this| {
                                this.child(
                                    div()
                                        .px(px(12.))
                                        .py(px(6.))
                                        .bg(t.bg_app)
                                        .border_b_1()
                                        .border_color(t.border)
                                        .flex()
                                        .items_center()
                                        .gap(px(6.))
                                        .child(Icon::new(IconName::Info).size(px(14.)))
                                        .child(
                                            div().text_xs().text_color(t.text_secondary).child(
                                                format!(
                                                "{} inherited header(s) from parent folder/workspace will be merged on send",
                                                inherited_count
                                            ),
                                            ),
                                        ),
                                )
                            })
                            .child(
                                div().flex_1().child(
                                    KvTable::new(
                                        format!("headers-kv-{}", req_id_str),
                                        headers,
                                        move |idx, k, v, enabled, _, cx| {
                                            ed_up.update(cx, |state, cx| {
                                                state.update_header(idx, k, v, enabled, cx);
                                            });
                                            save_active_request_without_redraw(&ed_up, &per_up, &ws_up, cx);
                                        },
                                        move |_, cx| {
                                            ed_add.update(cx, |state, cx| {
                                                state.add_header(cx);
                                            });
                                        },
                                        move |idx, _, cx| {
                                            ed_del.update(cx, |state, cx| {
                                                state.remove_header(idx, cx);
                                            });
                                        },
                                    )
                                    .env_state(self.env_state.clone())
                                    .modal_state(self.modal_state.clone()),
                                ),
                            )
                    }
                    RequestTabKind::Auth => {
                        let auth = ed_entity
                            .read(cx)
                            .active_request
                            .as_ref()
                            .map(|r| r.auth.clone())
                            .unwrap_or(Auth::Inherit);
                        let folder_id = ed_entity
                            .read(cx)
                            .active_request
                            .as_ref()
                            .and_then(|r| r.folder_id.clone());

                        render_auth_tab(
                            &auth,
                            folder_id.as_deref(),
                            &self.workspace_state,
                            &self.env_state,
                            &self.modal_state,
                            &self.editor_state,
                            AuthTabInputs {
                                bearer_token_input: &self.bearer_token_input,
                                basic_user_input: &self.basic_user_input,
                                basic_pass_input: &self.basic_pass_input,
                                apikey_key_input: &self.apikey_key_input,
                                apikey_val_input: &self.apikey_val_input,
                            },
                            window,
                            cx,
                        )
                    }
                    RequestTabKind::PreRequestScript => {
                        let pre_txt = self.pre_script_editor.clone();
                        let script_err = ed_entity.read(cx).script_error.clone();
                        div().size_full().child(ScriptEditor::new(
                            self.pre_script_editor.clone(),
                            ScriptEditorKind::PreRequest,
                            script_err,
                            move |snippet, window, cx| {
                                pre_txt.update(cx, |t, cx| {
                                    let cur = t.value().to_string();
                                    let new_val = if cur.is_empty() {
                                        snippet.to_string()
                                    } else if cur.ends_with('\n') {
                                        format!("{}{}", cur, snippet)
                                    } else {
                                        format!("{}\n{}", cur, snippet)
                                    };
                                    t.set_value(new_val, window, cx);
                                });
                            },
                        ))
                    }
                    RequestTabKind::Tests => {
                        let test_txt = self.test_script_editor.clone();
                        let script_err = ed_entity.read(cx).script_error.clone();
                        div().size_full().child(ScriptEditor::new(
                            self.test_script_editor.clone(),
                            ScriptEditorKind::Tests,
                            script_err,
                            move |snippet, window, cx| {
                                test_txt.update(cx, |t, cx| {
                                    let cur = t.value().to_string();
                                    let new_val = if cur.is_empty() {
                                        snippet.to_string()
                                    } else if cur.ends_with('\n') {
                                        format!("{}{}", cur, snippet)
                                    } else {
                                        format!("{}\n{}", cur, snippet)
                                    };
                                    t.set_value(new_val, window, cx);
                                });
                            },
                        ))
                    }
                }
            }))
            // Method dropdown if open (rendered LAST inside editor_pane so it paints on top of tabs and editor content)
            .when(method_dropdown_open, |this| {
                let ed_c = ed_entity.clone();
                this.child(
                    div()
                        .absolute()
                        .top(px(42.))
                        .left(px(12.))
                        .w(px(110.))
                        .bg(t.bg_card)
                        .border_1()
                        .border_color(t.border)
                        .rounded(px(6.))
                        .shadow_xl()
                        .p(px(4.))
                        .flex()
                        .flex_col()
                        .gap(px(2.))
                        .children(
                            [
                                HttpMethod::Get,
                                HttpMethod::Post,
                                HttpMethod::Put,
                                HttpMethod::Delete,
                                HttpMethod::Patch,
                                HttpMethod::Head,
                                HttpMethod::Options,
                            ]
                            .into_iter()
                            .map(|m| {
                                let ed_set = ed_c.clone();
                                div()
                                    .id(format!("select-method-{}", m.as_str()))
                                    .px(px(8.))
                                    .py(px(5.))
                                    .rounded(px(4.))
                                    .cursor_pointer()
                                    .hover(|s| s.bg(t.bg_hover))
                                    .text_xs()
                                    .font_weight(gpui::FontWeight::BOLD)
                                    .text_color(t.method_color(m))
                                    .on_click(move |_, _, cx| {
                                        ed_set.update(cx, |state, cx| {
                                            state.set_method(m, cx);
                                        });
                                    })
                                    .child(m.as_str())
                            }),
                        ),
                )
            })
            .into_any_element()
    }
}

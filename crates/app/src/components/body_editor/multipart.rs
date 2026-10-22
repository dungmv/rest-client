use super::persist_active_request_without_redraw;
use crate::state::{PersistenceState, RequestSessionState, WorkspaceDataState};
use crate::theme::theme;
use gpui::prelude::FluentBuilder as _;
use gpui::{
    div, px, AnyElement, App, AppContext as _, Entity, InteractiveElement as _, IntoElement,
    ParentElement, StatefulInteractiveElement as _, Styled, Window,
};
use gpui_component::scroll::ScrollableElement as _;
use gpui_component::Icon;
use gpui_kit::assets::IconName;
use restclient_core::models::MultipartItem;

pub fn render_multipart_editor(
    req_id_str: &str,
    items: &[MultipartItem],
    editor_state: &Entity<RequestSessionState>,
    workspace_state: Option<&Entity<WorkspaceDataState>>,
    persistence_state: Option<&Entity<PersistenceState>>,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let t = theme();
    let items = items.to_vec();
    let ed_c = editor_state.clone();
    let req_id_str = req_id_str.to_string();

    div()
        .size_full()
        .overflow_y_scrollbar()
        .p(px(8.))
        .flex()
        .flex_col()
        .gap(px(6.))
        // Multipart Table Header
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
                .child(div().w(px(24.)).child(""))
                .child(div().w(px(160.)).child("Key"))
                .child(div().w(px(70.)).child("Type"))
                .child(div().flex_1().child("Value / File Path"))
                .child(div().w(px(80.)).child("Browse"))
                .child(div().w(px(28.)).child("")),
        )
        // Multipart Rows
        .children(items.into_iter().enumerate().map(|(idx, item)| {
            let ed_row = ed_c.clone();
            let ed_del = ed_c.clone();
            let ed_pick = ed_c.clone();
            let ws_row = workspace_state.cloned();
            let ws_del = ws_row.clone();
            let ws_pick = ws_row.clone();
            let per_row = persistence_state.cloned();
            let per_del = per_row.clone();
            let per_pick = per_row.clone();
            let id_row = req_id_str.clone();
            let id_del = id_row.clone();
            let id_pick = id_row.clone();
            let key_str = item.key.clone();
            let val_str = item.value.clone();
            let is_file = item.is_file;
            let file_path = item.file_path.clone();
            let enabled = item.enabled;
            let mp_prefix = format!("mp-{}-row-{}", req_id_str, idx);

            let ed_up_k = ed_row.clone();
            let ed_up_v = ed_row.clone();
            let ws_up = workspace_state.cloned();
            let ws_up_v = ws_up.clone();
            let per_up = persistence_state.cloned();
            let per_up_v = per_up.clone();
            let fp_k = file_path.clone();
            let fp_v = file_path.clone();
            let mp_holder = window.use_keyed_state(
                gpui::SharedString::from(format!("{}-holder", mp_prefix)),
                cx,
                |window, cx| {
                    let init_k = key_str.clone();
                    let init_v = val_str.clone();
                    let key_input = cx.new(|cx| {
                        let mut s = gpui_component::input::InputState::new(window, cx)
                            .placeholder("Key...");
                        s.set_value(init_k, window, cx);
                        s
                    });
                    let val_input = cx.new(|cx| {
                        let mut s = gpui_component::input::InputState::new(window, cx)
                            .placeholder("Value...");
                        s.set_value(init_v, window, cx);
                        s
                    });

                    let k_in_1 = key_input.clone();
                    let v_in_1 = val_input.clone();
                    let id_k = req_id_str.clone();
                    let sub1 = cx.subscribe_in(&key_input, window, move |_, _, event, _, cx| {
                        if let gpui_component::input::InputEvent::Change = event {
                            let k = k_in_1.read(cx).value().to_string();
                            let v = v_in_1.read(cx).value().to_string();
                            if let Some(ws) = &ws_up {
                                ws.update(cx, |ws, _| {
                                    ws.update_active_request(&id_k, |r| {
                                        r.update_multipart_item(
                                            idx,
                                            k,
                                            v,
                                            is_file,
                                            fp_k.clone(),
                                            true,
                                        );
                                    });
                                });
                            }
                            ed_up_k.update(cx, |state, cx| {
                                state.mark_dirty(cx);
                            });
                            persist_active_request_without_redraw(
                                &ed_up_k,
                                per_up.as_ref(),
                                ws_up.as_ref(),
                                cx,
                            );
                        }
                    });

                    let k_in_2 = key_input.clone();
                    let v_in_2 = val_input.clone();
                    let id_v = req_id_str.clone();
                    let sub2 = cx.subscribe_in(&val_input, window, move |_, _, event, _, cx| {
                        if let gpui_component::input::InputEvent::Change = event {
                            let k = k_in_2.read(cx).value().to_string();
                            let v = v_in_2.read(cx).value().to_string();
                            if let Some(ws) = &ws_up_v {
                                ws.update(cx, |ws, _| {
                                    ws.update_active_request(&id_v, |r| {
                                        r.update_multipart_item(
                                            idx,
                                            k,
                                            v,
                                            is_file,
                                            fp_v.clone(),
                                            true,
                                        );
                                    });
                                });
                            }
                            ed_up_v.update(cx, |state, cx| {
                                state.mark_dirty(cx);
                            });
                            persist_active_request_without_redraw(
                                &ed_up_v,
                                per_up_v.as_ref(),
                                ws_up_v.as_ref(),
                                cx,
                            );
                        }
                    });

                    (key_input, val_input, vec![sub1, sub2])
                },
            );

            let (key_input, val_input) = {
                let h = mp_holder.read(cx);
                (h.0.clone(), h.1.clone())
            };

            div()
                .flex()
                .items_center()
                .px(px(6.))
                .py(px(4.))
                .gap(px(8.))
                .rounded(px(4.))
                .hover(|s| s.bg(t.bg_hover))
                // Checkbox
                .child({
                    let ed_toggle = ed_row.clone();
                    let ws_toggle = ws_row.clone();
                    let per_toggle = per_row.clone();
                    let id_toggle = id_row.clone();
                    let k = key_str.clone();
                    let v = val_str.clone();
                    let fp = file_path.clone();
                    div()
                        .id(format!("check-mp-{}", idx))
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
                            if let Some(ws) = &ws_toggle {
                                ws.update(cx, |ws, _| {
                                    ws.update_active_request(&id_toggle, |r| {
                                        r.update_multipart_item(
                                            idx,
                                            k.clone(),
                                            v.clone(),
                                            is_file,
                                            fp.clone(),
                                            !enabled,
                                        );
                                    });
                                });
                            }
                            ed_toggle.update(cx, |state, cx| {
                                state.mark_dirty(cx);
                            });
                            persist_active_request_without_redraw(
                                &ed_toggle,
                                per_toggle.as_ref(),
                                ws_toggle.as_ref(),
                                cx,
                            );
                        })
                        .child(if enabled { "✓" } else { "" })
                })
                // Key
                .child(
                    div()
                        .w(px(160.))
                        .h(px(28.))
                        .px(px(4.))
                        .rounded(px(4.))
                        .bg(t.bg_input)
                        .border_1()
                        .border_color(t.border)
                        .flex()
                        .items_center()
                        .child(
                            gpui_component::input::Input::new(&key_input)
                                .id(gpui::SharedString::from(format!("{}-key", mp_prefix)))
                                .appearance(false)
                                .bordered(false)
                                .focus_bordered(false),
                        ),
                )
                // Type toggle: Text | File
                .child({
                    let ed_toggle_type = ed_row.clone();
                    let ws_toggle_type = ws_row.clone();
                    let per_toggle_type = per_row.clone();
                    let id_toggle_type = id_row.clone();
                    let k = key_str.clone();
                    let v = val_str.clone();
                    let fp = file_path.clone();
                    div()
                        .id(format!("toggle-mp-type-{}", idx))
                        .w(px(70.))
                        .h(px(28.))
                        .rounded(px(4.))
                        .bg(t.bg_card)
                        .border_1()
                        .border_color(t.border)
                        .cursor_pointer()
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_xs()
                        .text_color(t.text_secondary)
                        .on_click(move |_, _, cx| {
                            if let Some(ws) = &ws_toggle_type {
                                ws.update(cx, |ws, _| {
                                    ws.update_active_request(&id_toggle_type, |r| {
                                        r.update_multipart_item(
                                            idx,
                                            k.clone(),
                                            v.clone(),
                                            !is_file,
                                            fp.clone(),
                                            enabled,
                                        );
                                    });
                                });
                            }
                            ed_toggle_type.update(cx, |state, cx| {
                                state.mark_dirty(cx);
                            });
                            persist_active_request_without_redraw(
                                &ed_toggle_type,
                                per_toggle_type.as_ref(),
                                ws_toggle_type.as_ref(),
                                cx,
                            );
                        })
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(3.))
                                .child(
                                    Icon::new(if is_file {
                                        IconName::File
                                    } else {
                                        IconName::Type
                                    })
                                    .size(px(12.)),
                                )
                                .child(if is_file { "File" } else { "Text" }),
                        )
                })
                // Value or Path
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
                        .child(if is_file {
                            div()
                                .px(px(4.))
                                .text_xs()
                                .font_family(".SystemMonoFont")
                                .text_color(t.text_primary)
                                .child(
                                    file_path
                                        .clone()
                                        .unwrap_or_else(|| "Select a file...".to_string()),
                                )
                                .into_any_element()
                        } else {
                            gpui_component::input::Input::new(&val_input)
                                .id(gpui::SharedString::from(format!("{}-val", mp_prefix)))
                                .appearance(false)
                                .bordered(false)
                                .focus_bordered(false)
                                .into_any_element()
                        }),
                )
                // Browse button (only enabled if is_file)
                .child({
                    let ed_p = ed_pick.clone();
                    let ws_p = ws_pick.clone();
                    let per_p = per_pick.clone();
                    let id_p = id_pick.clone();
                    let k = key_str.clone();
                    let v = val_str.clone();
                    let btn = div()
                        .id(format!("browse-mp-{}", idx))
                        .w(px(80.))
                        .h(px(28.))
                        .rounded(px(4.))
                        .bg(if is_file {
                            t.bg_card
                        } else {
                            gpui::transparent_black()
                        })
                        .border_1()
                        .border_color(if is_file {
                            t.border
                        } else {
                            gpui::transparent_black()
                        })
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_xs()
                        .text_color(if is_file {
                            t.text_secondary
                        } else {
                            t.text_muted
                        });

                    if is_file {
                        btn.cursor_pointer()
                            .hover(|s| s.bg(t.bg_hover))
                            .on_click(move |_, _, cx| {
                                let ed_bg = ed_p.clone();
                                let ws_bg = ws_p.clone();
                                let per_bg = per_p.clone();
                                let id_bg = id_p.clone();
                                let k_bg = k.clone();
                                let v_bg = v.clone();
                                if let Some(path) = rfd::FileDialog::new().pick_file() {
                                    let p = path.to_string_lossy().to_string();
                                    if let Some(ws) = &ws_bg {
                                        ws.update(cx, |ws, _| {
                                            ws.update_active_request(&id_bg, |r| {
                                                r.update_multipart_item(
                                                    idx,
                                                    k_bg,
                                                    v_bg,
                                                    true,
                                                    Some(p),
                                                    enabled,
                                                );
                                            });
                                        });
                                    }
                                    ed_bg.update(cx, |state, cx| {
                                        state.mark_dirty(cx);
                                    });
                                    persist_active_request_without_redraw(
                                        &ed_bg,
                                        per_bg.as_ref(),
                                        ws_bg.as_ref(),
                                        cx,
                                    );
                                }
                            })
                            .child("Browse...")
                    } else {
                        btn.child("")
                    }
                })
                // Delete row
                .child(
                    div()
                        .id(format!("del-mp-row-{}", idx))
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
                            if let Some(ws) = &ws_del {
                                ws.update(cx, |ws, _| {
                                    ws.update_active_request(&id_del, |r| {
                                        r.remove_multipart_item(idx);
                                    });
                                });
                            }
                            ed_del.update(cx, |state, cx| {
                                state.mark_dirty(cx);
                            });
                            persist_active_request_without_redraw(
                                &ed_del,
                                per_del.as_ref(),
                                ws_del.as_ref(),
                                cx,
                            );
                        })
                        .child("×"),
                )
        }))
        // Add Part button
        .child(div().pt(px(4.)).child({
            let ed_add = ed_c.clone();
            let ws_add = workspace_state.cloned();
            let per_add = persistence_state.cloned();
            let id_add = req_id_str.clone();
            div()
                .id("btn-add-multipart-part")
                .px(px(8.))
                .py(px(4.))
                .rounded(px(4.))
                .w(px(90.))
                .bg(t.bg_card)
                .border_1()
                .border_color(t.border)
                .cursor_pointer()
                .hover(|s| s.bg(t.bg_hover))
                .flex()
                .items_center()
                .justify_center()
                .gap(px(4.))
                .text_xs()
                .text_color(t.text_secondary)
                .on_click(move |_, _, cx| {
                    if let Some(ws) = &ws_add {
                        ws.update(cx, |ws, _| {
                            ws.update_active_request(&id_add, |r| {
                                r.add_multipart_item();
                            });
                        });
                    }
                    ed_add.update(cx, |state, cx| {
                        state.mark_dirty(cx);
                    });
                    persist_active_request_without_redraw(
                        &ed_add,
                        per_add.as_ref(),
                        ws_add.as_ref(),
                        cx,
                    );
                })
                .child("+ Add Part")
        }))
        .into_any_element()
}

use super::persist_active_request_without_redraw;
use crate::components::kv_table::KvTable;
use crate::state::{EnvironmentState, PersistenceState, RequestSessionState, WorkspaceDataState};
use gpui::{div, AnyElement, Entity, IntoElement, ParentElement, Styled};
use gpui_component::scroll::ScrollableElement as _;
use restclient_core::models::KeyValuePair;

pub fn render_form_editor(
    req_id_str: &str,
    pairs: &[KeyValuePair],
    editor_state: &Entity<RequestSessionState>,
    workspace_state: Option<&Entity<WorkspaceDataState>>,
    persistence_state: Option<&Entity<PersistenceState>>,
    env_state: Option<&Entity<EnvironmentState>>,
) -> AnyElement {
    let pairs = pairs.to_vec();
    let ed_up = editor_state.clone();
    let ws_up = workspace_state.cloned();
    let ws_add = ws_up.clone();
    let ws_del = ws_up.clone();
    let per_up = persistence_state.cloned();
    let ed_add = editor_state.clone();
    let ed_del = editor_state.clone();
    let req_id_change = req_id_str.to_string();
    let req_id_add = req_id_str.to_string();
    let req_id_del = req_id_str.to_string();

    div()
        .size_full()
        .overflow_y_scrollbar()
        .child(
            KvTable::new(
                format!("form-kv-{}", req_id_str),
                pairs,
                move |idx, k, v, enabled, _, cx| {
                    if let Some(ws) = &ws_up {
                        ws.update(cx, |ws, _| {
                            ws.update_active_request(&req_id_change, |r| {
                                r.update_form_param(idx, k, v, enabled);
                            });
                        });
                    }
                    ed_up.update(cx, |state, cx| {
                        state.mark_dirty(cx);
                    });
                    persist_active_request_without_redraw(
                        &ed_up,
                        per_up.as_ref(),
                        ws_up.as_ref(),
                        cx,
                    );
                },
                move |_, cx| {
                    if let Some(ws) = &ws_add {
                        ws.update(cx, |ws, _| {
                            ws.update_active_request(&req_id_add, |r| {
                                r.add_form_param();
                            });
                        });
                    }
                    ed_add.update(cx, |state, cx| {
                        state.mark_dirty(cx);
                    });
                },
                move |idx, _, cx| {
                    if let Some(ws) = &ws_del {
                        ws.update(cx, |ws, _| {
                            ws.update_active_request(&req_id_del, |r| {
                                r.remove_form_param(idx);
                            });
                        });
                    }
                    ed_del.update(cx, |state, cx| {
                        state.mark_dirty(cx);
                    });
                },
            )
            .optional_env_state(env_state.cloned()),
        )
        .into_any_element()
}

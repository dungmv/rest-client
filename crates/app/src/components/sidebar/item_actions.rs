use super::request_node::SidebarReqItem;
use crate::state::{EditorState, WorkspaceState};
use gpui::{App, AppContext as _, Entity};
use restclient_core::models::Folder;

pub fn delete_request_item(
    ws_entity: &Entity<WorkspaceState>,
    ed_entity: &Entity<EditorState>,
    req_id: &str,
    cx: &mut App,
) {
    let is_active = ed_entity
        .read(cx)
        .active_request
        .as_ref()
        .map(|r| r.id.as_str() == req_id)
        .unwrap_or(false);

    let (remaining_requests, deleted_idx) = ws_entity.update(cx, |ws, cx| {
        let idx = ws.requests.iter().position(|r| r.id == req_id);
        ws.delete_request(req_id, cx);
        (ws.requests.clone(), idx)
    });

    let next_tab_id = ed_entity.update(cx, |ed, cx| ed.close_tab(req_id, cx));

    if is_active {
        let next_req = next_tab_id
            .and_then(|id| remaining_requests.iter().find(|r| r.id == id).cloned())
            .or_else(|| {
                if let Some(idx) = deleted_idx {
                    if !remaining_requests.is_empty() {
                        let new_idx = idx.min(remaining_requests.len() - 1);
                        return Some(remaining_requests[new_idx].clone());
                    }
                }
                remaining_requests.first().cloned()
            });

        if let Some(req) = next_req {
            ed_entity.update(cx, |ed, cx| {
                ed.set_active_request(req, false, cx);
            });
        } else {
            let new_req = ws_entity.update(cx, |ws, cx| ws.create_request(None, cx));
            ed_entity.update(cx, |ed, cx| {
                ed.set_active_request(new_req, true, cx);
            });
        }
    }
}

pub fn folder_has_matches(
    folder_id: &str,
    folders: &[Folder],
    requests: &[SidebarReqItem],
    query: &str,
) -> bool {
    if query.is_empty() {
        return true;
    }
    if let Some(f) = folders.iter().find(|f| f.id == folder_id) {
        if f.name.to_lowercase().contains(query) {
            return true;
        }
    }
    if requests.iter().any(|r| {
        r.folder_id.as_deref() == Some(folder_id)
            && (r.name.to_lowercase().contains(query)
                || r.url.to_lowercase().contains(query)
                || r.examples.iter().any(|e| e.name.to_lowercase().contains(query)))
    }) {
        return true;
    }
    for child in folders.iter().filter(|f| f.parent_id.as_deref() == Some(folder_id)) {
        if folder_has_matches(&child.id, folders, requests, query) {
            return true;
        }
    }
    false
}

use super::request_node::SidebarReqItem;
use crate::state::{RequestSessionState, WorkspaceState};
use gpui::{App, AppContext as _, Entity};
use zest_core::models::Folder;

pub fn delete_request_item(
    ws_entity: &Entity<WorkspaceState>,
    ed_entity: &Entity<RequestSessionState>,
    req_id: &str,
    cx: &mut App,
) {
    let is_active = ed_entity.read(cx).active_request_id() == req_id;

    let (remaining_requests, deleted_idx) = ws_entity.update(cx, |ws, cx| {
        let idx = ws.requests.iter().position(|r| r.id == req_id);
        ws.delete_request(req_id, cx);
        (ws.requests.clone(), idx)
    });

    if is_active {
        let next_req_id = if let Some(idx) = deleted_idx {
            if !remaining_requests.is_empty() {
                let new_idx = idx.min(remaining_requests.len() - 1);
                Some(remaining_requests[new_idx].id.clone())
            } else {
                None
            }
        } else {
            remaining_requests.first().map(|r| r.id.clone())
        };

        if let Some(id) = next_req_id {
            ed_entity.update(cx, |ed, cx| {
                ed.set_active_request_id(&id, cx);
            });
        } else {
            let new_req = ws_entity.update(cx, |ws, cx| ws.create_request(None, cx));
            ed_entity.update(cx, |ed, cx| {
                ed.set_active_request_id(&new_req.id, cx);
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
                || r.examples
                    .iter()
                    .any(|e| e.name.to_lowercase().contains(query)))
    }) {
        return true;
    }
    for child in folders
        .iter()
        .filter(|f| f.parent_id.as_deref() == Some(folder_id))
    {
        if folder_has_matches(&child.id, folders, requests, query) {
            return true;
        }
    }
    false
}

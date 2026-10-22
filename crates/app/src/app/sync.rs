use crate::state::{PersistenceState, RequestSessionState, WorkspaceDataState};
use gpui::{App, Entity};

pub fn save_active_request_without_redraw(
    editor: &Entity<RequestSessionState>,
    persistence: &Entity<PersistenceState>,
    workspace: &Entity<WorkspaceDataState>,
    cx: &mut App,
) {
    let req_id = editor.read(cx).active_request_id().to_string();
    if req_id != "scratch" && !req_id.is_empty() {
        let ed = editor.clone();
        let ws = workspace.clone();
        persistence.update(cx, |per, cx| per.schedule_save_active_request(req_id, ed, ws, cx));
    }
}


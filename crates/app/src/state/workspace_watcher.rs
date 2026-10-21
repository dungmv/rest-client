use super::WorkspaceDataState;
use crate::state::EditorState;
use gpui::{Context, Entity};
use restclient_core::models::HttpRequestItem;
use restclient_core::storage::models::RequestFile;

impl WorkspaceDataState {
    pub fn update_request(&mut self, mut req: HttpRequestItem, cx: &mut Context<Self>) {
        let Some(idx) = self.requests.iter().position(|r| r.id == req.id) else {
            return;
        };
        if req.last_status_code.is_none() {
            req.last_status_code = self.requests[idx].last_status_code;
        }
        let was_unsaved = self.unsaved_requests.contains(&req.id);
        let prev_name = self.requests[idx].name.clone();
        let prev_method = self.requests[idx].method;
        let visual_changed = !was_unsaved || prev_name != req.name || prev_method != req.method;

        self.requests[idx] = req.clone();
        self.unsaved_requests.insert(req.id.clone());
        if visual_changed {
            cx.notify();
        }

        let req_id = req.id.clone();
        let store = self.store.clone();
        let task = cx.spawn(async move |this, cx| {
            cx.background_executor().timer(std::time::Duration::from_millis(500)).await;

            let Some(this_ent) = this.upgrade() else { return; };
            let Some(r) = this_ent.read_with(cx, |this, _| this.requests.iter().find(|r| r.id == req_id).cloned()) else { return; };

            let r_id = req_id.clone();
            let store_clone = store.clone();
            cx.background_executor().spawn(async move {
                let mut req_file = store_clone.read_request(&r_id).unwrap_or_else(|_| RequestFile {
                    name: r.name.clone(),
                    method: r.method,
                    url: r.url.clone(),
                    ..Default::default()
                });
                req_file.update_from_item(&r);
                let _ = store_clone.write_request(&r_id, &req_file);
            }).await;

            if let Some(this_ent) = this.upgrade() {
                this_ent.update(cx, |ws, cx| {
                    ws.unsaved_requests.remove(&req_id);
                    cx.notify();
                });
            }
        });
        self.request_save_tasks.insert(req.id, task);
    }

    pub fn update_request_silently(&mut self, mut req: HttpRequestItem, cx: &mut Context<Self>) {
        let Some(idx) = self.requests.iter().position(|r| r.id == req.id) else {
            return;
        };
        if req.last_status_code.is_none() {
            req.last_status_code = self.requests[idx].last_status_code;
        }
        let req_id = req.id.clone();
        self.requests[idx] = req;

        let store = self.store.clone();
        let task_req_id = req_id.clone();
        let task = cx.spawn(async move |this, cx| {
            cx.background_executor().timer(std::time::Duration::from_millis(500)).await;

            let Some(this_ent) = this.upgrade() else { return; };
            let Some(r) = this_ent.read_with(cx, |this, _| {
                this.requests.iter().find(|r| r.id == task_req_id).cloned()
            }) else { return; };

            let store_clone = store.clone();
            let r_id = task_req_id.clone();
            cx.background_executor().spawn(async move {
                let mut req_file = store_clone.read_request(&r_id).unwrap_or_else(|_| RequestFile {
                    name: r.name.clone(),
                    method: r.method,
                    url: r.url.clone(),
                    ..Default::default()
                });
                req_file.update_from_item(&r);
                let _ = store_clone.write_request(&r_id, &req_file);
            }).await;

            let final_req_id = task_req_id.clone();
            if let Some(this_ent) = this.upgrade() {
                this_ent.update(cx, |ws, cx| {
                    ws.unsaved_requests.remove(&final_req_id);
                    cx.notify();
                });
            }
        });
        self.request_save_tasks.insert(req_id, task);
    }

    pub fn schedule_save_active_request(
        &mut self,
        req_id: String,
        editor: Entity<EditorState>,
        cx: &mut Context<Self>,
    ) {
        let store = self.store.clone();
        let task_req_id = req_id.clone();
        let task = cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(std::time::Duration::from_millis(500))
                .await;

            let Some(req) = editor.read_with(cx, |ed, _| {
                if ed.active_request.as_ref().map(|r| &r.id) == Some(&task_req_id) {
                    ed.active_request.clone()
                } else {
                    None
                }
            }) else {
                return;
            };

            let Some(this_ent) = this.upgrade() else { return; };
            this_ent.update(cx, |ws, _| {
                if let Some(idx) = ws.requests.iter().position(|r| r.id == task_req_id) {
                    let mut r = req.clone();
                    if r.last_status_code.is_none() {
                        r.last_status_code = ws.requests[idx].last_status_code;
                    }
                    ws.requests[idx] = r;
                }
            });

            let store_clone = store.clone();
            let r_id = task_req_id.clone();
            let r_clone = req;
            cx.background_executor().spawn(async move {
                let mut req_file = store_clone.read_request(&r_id).unwrap_or_else(|_| RequestFile {
                    name: r_clone.name.clone(),
                    method: r_clone.method,
                    url: r_clone.url.clone(),
                    ..Default::default()
                });
                req_file.update_from_item(&r_clone);
                let _ = store_clone.write_request(&r_id, &req_file);
            }).await;

            let final_req_id = task_req_id.clone();
            if let Some(this_ent) = this.upgrade() {
                this_ent.update(cx, |ws, cx| {
                    ws.unsaved_requests.remove(&final_req_id);
                    cx.notify();
                });
            }
        });
        self.request_save_tasks.insert(req_id, task);
    }

    pub fn mark_dirty(&mut self, req_id: String, cx: &mut Context<Self>) {
        self.unsaved_requests.insert(req_id);
        cx.notify();
    }

    pub fn get_request_file(&self, req_id: &str) -> Option<RequestFile> {
        self.store.read_request(req_id).ok()
    }

    pub fn save_request_file(&mut self, req_id: &str, req_file: &RequestFile, cx: &mut Context<Self>) {
        let _ = self.store.write_request(req_id, req_file);
        self.reload_tree();
        cx.notify();
    }

    pub fn start_watcher(&mut self, cx: &mut Context<Self>) {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<restclient_core::storage::FsEvent>();
        let sup = self.store.suppressed_paths();
        if let Ok(watcher) = restclient_core::storage::start_workspace_watcher(&self.workspace_path, sup, tx) {
            self.watcher = Some(watcher);
            let task = cx.spawn(async move |this, cx| {
                while let Some(event) = rx.recv().await {
                    if let Some(this) = this.upgrade() {
                        this.update(cx, |ws, cx| {
                            ws.handle_fs_event(event, cx);
                        });
                    }
                }
            });
            self.watcher_task = Some(task);
        }
    }

    pub fn handle_fs_event(&mut self, event: restclient_core::storage::FsEvent, cx: &mut Context<Self>) {
        match event {
            restclient_core::storage::FsEvent::RequestChanged(rel_path) => {
                if self.unsaved_requests.contains(&rel_path) {
                    self.conflicts.insert(
                        rel_path,
                        "File was modified externally while you have unsaved changes.".to_string(),
                    );
                    cx.notify();
                } else {
                    match self.store.read_request(&rel_path) {
                        Ok(req_file) => {
                            self.parse_errors.remove(&rel_path);
                            let ws_name = self.workspace_meta.name.clone();
                            if let Some(r) = self.requests.iter_mut().find(|r| r.id == rel_path) {
                                let prev_status = r.last_status_code.or_else(|| {
                                    self.store.load_response_cache(&r.id).ok().flatten().map(|l| l.response.status)
                                });
                                *r = req_file.to_item(&r.id, &ws_name, r.folder_id.clone());
                                r.last_status_code = prev_status;
                            }
                            cx.notify();
                        }
                        Err(e) => {
                            self.parse_errors.insert(rel_path, e.to_string());
                            cx.notify();
                        }
                    }
                }
            }
            restclient_core::storage::FsEvent::RequestDeleted(rel_path) => {
                self.parse_errors.remove(&rel_path);
                self.conflicts.remove(&rel_path);
                self.requests.retain(|r| r.id != rel_path);
                self.reload_tree();
                cx.notify();
            }
            restclient_core::storage::FsEvent::EnvironmentChanged(_) => {
                self.reload_environments();
                cx.notify();
            }
            restclient_core::storage::FsEvent::TreeChanged => {
                self.reload_tree();
                cx.notify();
            }
        }
    }

    pub fn resolve_conflict_reload(&mut self, rel_path: &str, cx: &mut Context<Self>) {
        self.conflicts.remove(rel_path);
        self.unsaved_requests.remove(rel_path);
        if let Ok(req_file) = self.store.read_request(rel_path) {
            self.parse_errors.remove(rel_path);
            let ws_name = self.workspace_meta.name.clone();
            if let Some(r) = self.requests.iter_mut().find(|r| r.id == rel_path) {
                let prev_status = r.last_status_code.or_else(|| {
                    self.store.load_response_cache(&r.id).ok().flatten().map(|l| l.response.status)
                });
                *r = req_file.to_item(&r.id, &ws_name, r.folder_id.clone());
                r.last_status_code = prev_status;
            }
        }
        cx.notify();
    }

    pub fn resolve_conflict_keep(&mut self, rel_path: &str, cx: &mut Context<Self>) {
        self.conflicts.remove(rel_path);
        if let Some(r) = self.requests.iter().find(|r| r.id == rel_path).cloned() {
            let mut req_file = self.store.read_request(rel_path).unwrap_or_else(|_| RequestFile {
                name: r.name.clone(),
                method: r.method,
                url: r.url.clone(),
                ..Default::default()
            });
            req_file.update_from_item(&r);
            let _ = self.store.write_request(rel_path, &req_file);
            self.unsaved_requests.remove(rel_path);
        }
        cx.notify();
    }
}

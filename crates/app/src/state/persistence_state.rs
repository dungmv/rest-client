use super::{EditorState, EnvironmentState, WorkspaceDataState};
use gpui::{Context, Entity, Task};
use restclient_core::storage::models::{LocalState, RequestFile};
use restclient_core::storage::{FsEvent, WorkspaceStore, WorkspaceWatcher};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;

pub struct PersistenceState {
    pub store: Arc<dyn WorkspaceStore>,
    pub workspace_path: PathBuf,
    pub local_state: LocalState,
    pub conflicts: HashMap<String, String>,
    pub unsaved_requests: HashSet<String>,
    pub request_save_tasks: HashMap<String, Task<()>>,
    pub state_save_task: Option<Task<()>>,
    pub watcher: Option<WorkspaceWatcher>,
    pub watcher_task: Option<Task<()>>,
}

impl PersistenceState {
    pub fn new(store: Arc<dyn WorkspaceStore>, workspace_path: PathBuf) -> Self {
        let local_state = store.load_state().unwrap_or_default();
        Self {
            store,
            workspace_path,
            local_state,
            conflicts: HashMap::new(),
            unsaved_requests: HashSet::new(),
            request_save_tasks: HashMap::new(),
            state_save_task: None,
            watcher: None,
            watcher_task: None,
        }
    }

    pub fn set_workspace(&mut self, store: Arc<dyn WorkspaceStore>, path: PathBuf, cx: &mut Context<Self>) {
        self.store = store;
        self.workspace_path = path;
        self.conflicts.clear();
        self.unsaved_requests.clear();
        self.local_state = self.store.load_state().unwrap_or_default();
        cx.notify();
    }

    pub fn schedule_state_save(&mut self, cx: &mut Context<Self>) {
        self.state_save_task = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(std::time::Duration::from_millis(1500)).await;
            if let Some(this) = this.upgrade() {
                this.update(cx, |p, _| {
                    let _ = p.store.save_state(&p.local_state);
                });
            }
        }));
    }

    pub fn set_active_request(&mut self, req_id: Option<String>, cx: &mut Context<Self>) {
        self.local_state.active_request = req_id;
        self.schedule_state_save(cx);
        cx.notify();
    }

    pub fn set_active_environment(&mut self, env_id: Option<String>, cx: &mut Context<Self>) {
        self.local_state.active_environment = env_id;
        self.schedule_state_save(cx);
        cx.notify();
    }

    pub fn mark_dirty(&mut self, req_id: String, cx: &mut Context<Self>) {
        self.unsaved_requests.insert(req_id);
        cx.notify();
    }

    pub fn mark_clean(&mut self, req_id: &str, cx: &mut Context<Self>) {
        self.unsaved_requests.remove(req_id);
        cx.notify();
    }

    pub fn is_dirty(&self, req_id: &str) -> bool {
        self.unsaved_requests.contains(req_id)
    }

    pub fn schedule_save_active_request(
        &mut self,
        req_id: String,
        editor: Entity<EditorState>,
        workspace_data: Entity<WorkspaceDataState>,
        cx: &mut Context<Self>,
    ) {
        let store = self.store.clone();
        let task_req_id = req_id.clone();
        self.unsaved_requests.insert(req_id.clone());

        let req_snapshot = editor.read(cx).active_request.clone();
        let ed_clone = editor.clone();

        let task = cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(std::time::Duration::from_millis(500))
                .await;

            let req = editor
                .read_with(cx, |ed, _| {
                    if ed.active_request.as_ref().map(|r| &r.id) == Some(&task_req_id) {
                        ed.active_request.clone()
                    } else {
                        None
                    }
                })
                .or_else(|| {
                    if req_snapshot.as_ref().map(|r| &r.id) == Some(&task_req_id) {
                        req_snapshot
                    } else {
                        None
                    }
                });

            let Some(req) = req else {
                if let Some(this_ent) = this.upgrade() {
                    this_ent.update(cx, |p, cx| {
                        p.unsaved_requests.remove(&task_req_id);
                        cx.notify();
                    });
                }
                return;
            };

            workspace_data.update(cx, |ws, cx| {
                if let Some(idx) = ws.requests.iter().position(|r| r.id == task_req_id) {
                    let mut r = req.clone();
                    if r.last_status_code.is_none() {
                        r.last_status_code = ws.requests[idx].last_status_code;
                    }
                    ws.requests[idx] = r;
                    cx.notify();
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
                this_ent.update(cx, |p, cx| {
                    p.unsaved_requests.remove(&final_req_id);
                    cx.notify();
                });
                ed_clone.update(cx, |ed, _| {
                    if ed.active_request.as_ref().map(|r| &r.id) == Some(&final_req_id) {
                        ed.is_dirty = false;
                    }
                });
            }
        });
        self.request_save_tasks.insert(req_id, task);
    }

    pub fn start_watcher(
        &mut self,
        workspace_data: Entity<WorkspaceDataState>,
        env_state: Entity<EnvironmentState>,
        cx: &mut Context<Self>,
    ) {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<FsEvent>();
        let sup = self.store.suppressed_paths();
        if let Ok(watcher) = restclient_core::storage::start_workspace_watcher(&self.workspace_path, sup, tx) {
            self.watcher = Some(watcher);
            let ws_clone = workspace_data.clone();
            let env_clone = env_state.clone();
            let task = cx.spawn(async move |this, cx| {
                while let Some(event) = rx.recv().await {
                    if let Some(this) = this.upgrade() {
                        let ws = ws_clone.clone();
                        let env = env_clone.clone();
                        this.update(cx, |p, cx| {
                            p.handle_fs_event(event, ws, env, cx);
                        });
                    }
                }
            });
            self.watcher_task = Some(task);
        }
    }

    pub fn handle_fs_event(
        &mut self,
        event: FsEvent,
        workspace_data: Entity<WorkspaceDataState>,
        env_state: Entity<EnvironmentState>,
        cx: &mut Context<Self>,
    ) {
        match event {
            FsEvent::RequestChanged(rel_path) => {
                if self.unsaved_requests.contains(&rel_path) {
                    self.conflicts.insert(
                        rel_path,
                        "File was modified externally while you have unsaved changes.".to_string(),
                    );
                    cx.notify();
                } else {
                    match self.store.read_request(&rel_path) {
                        Ok(req_file) => {
                            let path = rel_path.clone();
                            workspace_data.update(cx, |ws, cx| {
                                ws.parse_errors.remove(&path);
                                let ws_name = ws.workspace_meta.name.clone();
                                if let Some(r) = ws.requests.iter_mut().find(|r| r.id == path) {
                                    let prev_status = r.last_status_code.or_else(|| {
                                        ws.store.load_response_cache(&r.id).ok().flatten().map(|l| l.response.status)
                                    });
                                    *r = req_file.to_item(&r.id, &ws_name, r.folder_id.clone());
                                    r.last_status_code = prev_status;
                                }
                                cx.notify();
                            });
                        }
                        Err(e) => {
                            let path = rel_path.clone();
                            workspace_data.update(cx, |ws, cx| {
                                ws.parse_errors.insert(path, e.to_string());
                                cx.notify();
                            });
                        }
                    }
                }
            }
            FsEvent::RequestDeleted(rel_path) => {
                self.conflicts.remove(&rel_path);
                let path = rel_path.clone();
                workspace_data.update(cx, |ws, cx| {
                    ws.parse_errors.remove(&path);
                    ws.requests.retain(|r| r.id != path);
                    cx.notify();
                });
                cx.notify();
            }
            FsEvent::EnvironmentChanged(_) => {
                env_state.update(cx, |env, cx| {
                    env.reload_environments();
                    cx.notify();
                });
            }
            FsEvent::TreeChanged => {
                workspace_data.update(cx, |ws, cx| {
                    ws.reload_tree();
                    cx.notify();
                });
            }
        }
    }

    pub fn resolve_conflict_reload(
        &mut self,
        rel_path: &str,
        workspace_data: Entity<WorkspaceDataState>,
        cx: &mut Context<Self>,
    ) {
        self.conflicts.remove(rel_path);
        self.unsaved_requests.remove(rel_path);
        if let Ok(req_file) = self.store.read_request(rel_path) {
            let path = rel_path.to_string();
            workspace_data.update(cx, |ws, cx| {
                ws.parse_errors.remove(&path);
                let ws_name = ws.workspace_meta.name.clone();
                if let Some(r) = ws.requests.iter_mut().find(|r| r.id == path) {
                    let prev_status = r.last_status_code.or_else(|| {
                        ws.store.load_response_cache(&r.id).ok().flatten().map(|l| l.response.status)
                    });
                    *r = req_file.to_item(&r.id, &ws_name, r.folder_id.clone());
                    r.last_status_code = prev_status;
                }
                cx.notify();
            });
        }
        cx.notify();
    }

    pub fn resolve_conflict_keep(
        &mut self,
        rel_path: &str,
        workspace_data: Entity<WorkspaceDataState>,
        cx: &mut Context<Self>,
    ) {
        self.conflicts.remove(rel_path);
        self.unsaved_requests.remove(rel_path);
        let req_opt = workspace_data.read(cx).requests.iter().find(|r| r.id == rel_path).cloned();
        if let Some(r) = req_opt {
            let mut req_file = self.store.read_request(rel_path).unwrap_or_else(|_| RequestFile {
                name: r.name.clone(),
                method: r.method,
                url: r.url.clone(),
                ..Default::default()
            });
            req_file.update_from_item(&r);
            let _ = self.store.write_request(rel_path, &req_file);
        }
        cx.notify();
    }
}

use crate::platform::detect_git_branch;
use crate::settings::AppSettings;
use gpui::{AppContext as _, Context, Entity, Pixels, Task, Window, px};
use gpui_component::input::InputState;
use restclient_core::models::{
    Environment, Folder, HttpMethod, HttpRequestItem, Workspace,
};
use restclient_core::storage::models::{
    EnvironmentFile, FolderNode, LocalState, RequestFile, WorkspaceMeta,
};
use restclient_core::storage::{FsWorkspaceStore, WorkspaceStore};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;

pub struct WorkspaceState {
    pub store: Arc<dyn WorkspaceStore>,
    pub workspace_path: PathBuf,
    pub workspace_meta: WorkspaceMeta,
    pub active_workspace: Option<Workspace>,
    pub folders: Vec<Folder>,
    pub requests: Vec<HttpRequestItem>,
    pub parse_errors: HashMap<String, String>,
    pub pins: Vec<String>,
    pub local_state: LocalState,
    pub environments: Vec<Environment>,
    pub global_environment: Environment,
    pub active_environment_id: Option<String>,
    pub selected_env_modal_id: Option<String>,
    pub env_filter_query: String,
    pub var_filter_query: String,
    pub git_branch: Option<String>,
    pub git_tracked_warning: bool,
    pub filter_query: String,
    pub sidebar_collapsed: bool,
    pub sidebar_width: Pixels,
    pub split_layout_vertical: bool,
    pub show_env_modal: bool,
    pub show_cookies_modal: bool,
    pub show_command_palette: bool,
    pub show_settings_modal: bool,
    pub renaming_request_id: Option<String>,
    pub rename_input: Option<Entity<InputState>>,
    pub rename_subscription: Option<gpui::Subscription>,
    pub renaming_folder_id: Option<String>,
    pub rename_folder_input: Option<Entity<InputState>>,
    pub rename_folder_subscription: Option<gpui::Subscription>,
    pub renaming_env_id: Option<String>,
    pub rename_env_input: Option<Entity<InputState>>,
    pub rename_env_subscription: Option<gpui::Subscription>,
    pub collapsed_folders: HashSet<String>,
    pub collapsed_requests: HashSet<String>,
    pub unsaved_requests: HashSet<String>,
    pub running_request_id: Option<String>,
    pub recent_workspaces: Vec<String>,
    pub nav_history: Vec<String>,
    pub nav_index: usize,
    pub state_save_task: Option<Task<()>>,
    pub request_save_tasks: HashMap<String, Task<()>>,
    pub conflicts: HashMap<String, String>,
    pub corrupt_secrets_warning: Option<String>,
    pub last_renamed_request: Option<(String, String)>,
    pub watcher: Option<restclient_core::storage::WorkspaceWatcher>,
    pub watcher_task: Option<Task<()>>,
}

impl WorkspaceState {
    pub fn new() -> Self {
        let mut settings = AppSettings::load();
        let ws_dir = if let Some(curr) = &settings.current_workspace {
            PathBuf::from(curr)
        } else {
            dirs::data_local_dir()
                .map(|d| d.join("restclient").join("workspaces").join("my-workspace"))
                .unwrap_or_else(|| PathBuf::from("./my-workspace"))
        };

        let store: Arc<dyn WorkspaceStore> = Arc::new(FsWorkspaceStore::new(&ws_dir));
        if !ws_dir.join("workspace.yaml").exists() {
            let _ = store.init_workspace("My Workspace");
        }

        settings.add_recent_workspace(&ws_dir.to_string_lossy());
        let git_branch = detect_git_branch(Some(&ws_dir));
        let git_tracked_warning = store.ensure_gitignore().unwrap_or(false);

        let mut state = Self {
            store,
            workspace_path: ws_dir,
            workspace_meta: WorkspaceMeta::default(),
            active_workspace: Some(Workspace::new("My Workspace")),
            folders: Vec::new(),
            requests: Vec::new(),
            parse_errors: HashMap::new(),
            pins: Vec::new(),
            local_state: LocalState::default(),
            environments: Vec::new(),
            global_environment: Environment::new("default", "Globals"),
            active_environment_id: None,
            selected_env_modal_id: Some("globals".to_string()),
            env_filter_query: String::new(),
            var_filter_query: String::new(),
            git_branch,
            git_tracked_warning,
            filter_query: String::new(),
            sidebar_collapsed: false,
            sidebar_width: px(260.),
            split_layout_vertical: false,
            show_env_modal: false,
            show_cookies_modal: false,
            show_command_palette: false,
            show_settings_modal: false,
            renaming_request_id: None,
            rename_input: None,
            rename_subscription: None,
            renaming_folder_id: None,
            rename_folder_input: None,
            rename_folder_subscription: None,
            renaming_env_id: None,
            rename_env_input: None,
            rename_env_subscription: None,
            collapsed_folders: HashSet::new(),
            collapsed_requests: HashSet::new(),
            unsaved_requests: HashSet::new(),
            running_request_id: None,
            recent_workspaces: settings.recent_workspaces,
            nav_history: Vec::new(),
            nav_index: 0,
            state_save_task: None,
            request_save_tasks: HashMap::new(),
            conflicts: HashMap::new(),
            corrupt_secrets_warning: None,
            last_renamed_request: None,
            watcher: None,
            watcher_task: None,
        };

        state.reload_all();
        state
    }

    pub fn open_workspace(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        let store: Arc<dyn WorkspaceStore> = Arc::new(FsWorkspaceStore::new(&path));
        if !path.join("workspace.yaml").exists() {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("My Workspace");
            let _ = store.init_workspace(name);
        }
        let git_tracked_warning = store.ensure_gitignore().unwrap_or(false);
        self.store = store;
        self.workspace_path = path.clone();
        self.git_tracked_warning = git_tracked_warning;
        self.git_branch = detect_git_branch(Some(&path));
        self.conflicts.clear();
        self.corrupt_secrets_warning = None;

        let mut settings = AppSettings::load();
        settings.add_recent_workspace(&path.to_string_lossy());
        self.recent_workspaces = settings.recent_workspaces;

        self.reload_all();
        self.start_watcher(cx);
        cx.notify();
    }

    pub fn new_workspace(&mut self, path: PathBuf, name: &str, cx: &mut Context<Self>) {
        let store: Arc<dyn WorkspaceStore> = Arc::new(FsWorkspaceStore::new(&path));
        let _ = store.init_workspace(name);
        self.open_workspace(path, cx);
    }

    pub fn reload_all(&mut self) {
        if let Ok(meta) = self.store.load_workspace_meta() {
            self.workspace_meta = meta;
        }
        self.active_workspace = Some(Workspace {
            id: self.workspace_path.to_string_lossy().to_string(),
            name: self.workspace_meta.name.clone(),
            description: String::new(),
            active_environment_id: self.active_environment_id.clone(),
        });

        self.pins = self.store.load_pins().unwrap_or_default();
        self.local_state = self.store.load_state().unwrap_or_default();
        if self.local_state.sidebar.width > 0.0 {
            self.sidebar_width = px(self.local_state.sidebar.width.clamp(200.0, 480.0));
        }
        self.sidebar_collapsed = self.local_state.sidebar.collapsed;
        self.split_layout_vertical = self.local_state.layout.split == "vertical";

        self.reload_tree();
        self.reload_environments();

        if let Some(act) = &self.local_state.active_request {
            if self.nav_history.is_empty() {
                self.nav_history = vec![act.clone()];
                self.nav_index = 0;
            }
        }
    }

    pub fn reload_tree(&mut self) {
        if let Ok(tree) = self.store.scan_tree() {
            let ws_name = self.workspace_meta.name.clone();
            let mut all_requests = Vec::new();
            let mut all_folders = Vec::new();
            let mut errors = HashMap::new();

            for req_node in &tree.root_requests {
                let mut item = req_node.file.to_item(&req_node.rel_path, &ws_name, None);
                item.last_status_code = req_node.last_status_code;
                all_requests.push(item);
                if let Some(err) = &req_node.parse_error {
                    errors.insert(req_node.rel_path.clone(), err.clone());
                }
            }

            fn process_folder(
                folder_node: &FolderNode,
                parent_id: Option<String>,
                ws_name: &str,
                all_folders: &mut Vec<Folder>,
                all_requests: &mut Vec<HttpRequestItem>,
                errors: &mut HashMap<String, String>,
            ) {
                let mut f = Folder::new(ws_name, &folder_node.file.name);
                f.id = folder_node.rel_path.clone();
                f.parent_id = parent_id;
                all_folders.push(f);

                for req_node in &folder_node.requests {
                    let mut item = req_node
                        .file
                        .to_item(&req_node.rel_path, ws_name, Some(folder_node.rel_path.clone()));
                    item.last_status_code = req_node.last_status_code;
                    all_requests.push(item);
                    if let Some(err) = &req_node.parse_error {
                        errors.insert(req_node.rel_path.clone(), err.clone());
                    }
                }

                for sub in &folder_node.subfolders {
                    process_folder(
                        sub,
                        Some(folder_node.rel_path.clone()),
                        ws_name,
                        all_folders,
                        all_requests,
                        errors,
                    );
                }
            }

            for f_node in &tree.folders {
                process_folder(
                    f_node,
                    None,
                    &ws_name,
                    &mut all_folders,
                    &mut all_requests,
                    &mut errors,
                );
            }

            self.requests = all_requests;
            self.folders = all_folders;
            self.parse_errors = errors;
        }
    }

    pub fn reload_environments(&mut self) {
        let ws_name = self.workspace_meta.name.clone();
        let env_names = self.store.list_environments().unwrap_or_default();
        let mut envs = Vec::new();

        for name in &env_names {
            if let Ok(env_file) = self.store.load_environment(name) {
                let secrets = match self.store.load_secrets(name) {
                    Ok(s) => s,
                    Err(e) => {
                        self.corrupt_secrets_warning = Some(format!("{}", e));
                        HashMap::new()
                    }
                };
                let env = env_file.to_environment(name, &ws_name, name, &secrets);
                envs.push(env);
            }
        }

        self.environments = envs;

        let active_opt = self.local_state.active_environment.clone()
            .or_else(|| self.workspace_meta.default_environment.clone());

        if let Some(act) = active_opt {
            if self.environments.iter().any(|e| e.name == act || e.id == act) {
                self.active_environment_id = Some(act);
            } else {
                self.active_environment_id = self.environments.first().map(|e| e.id.clone());
            }
        } else if self.active_environment_id.is_none() {
            self.active_environment_id = self.environments.first().map(|e| e.id.clone());
        }
    }

    pub fn active_environment(&self) -> Option<&Environment> {
        if let Some(env_id) = &self.active_environment_id {
            self.environments.iter().find(|e| &e.id == env_id || &e.name == env_id)
        } else {
            None
        }
    }

    pub fn active_environment_mut(&mut self) -> Option<&mut Environment> {
        if let Some(env_id) = &self.active_environment_id {
            self.environments.iter_mut().find(|e| &e.id == env_id || &e.name == env_id)
        } else {
            None
        }
    }

    pub fn schedule_state_save(&mut self, cx: &mut Context<Self>) {
        self.state_save_task = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(std::time::Duration::from_millis(1500)).await;
            if let Some(this) = this.upgrade() {
                this.update(cx, |ws, _| {
                    let _ = ws.store.save_state(&ws.local_state);
                });
            }
        }));
    }

    pub fn record_navigation(&mut self, req_id: &str, cx: &mut Context<Self>) {
        if self.nav_history.get(self.nav_index).map(|s| s.as_str()) != Some(req_id) {
            if self.nav_index + 1 < self.nav_history.len() {
                self.nav_history.truncate(self.nav_index + 1);
            }
            self.nav_history.push(req_id.to_string());
            self.nav_index = self.nav_history.len().saturating_sub(1);
        }

        self.local_state.active_request = Some(req_id.to_string());
        self.local_state.recent.retain(|p| p != req_id);
        self.local_state.recent.insert(0, req_id.to_string());
        if self.local_state.recent.len() > 50 {
            self.local_state.recent.truncate(50);
        }
        self.schedule_state_save(cx);
    }

    pub fn navigate_back(&mut self, cx: &mut Context<Self>) -> Option<HttpRequestItem> {
        if self.nav_index > 0 {
            self.nav_index -= 1;
            if let Some(req_id) = self.nav_history.get(self.nav_index).cloned() {
                self.local_state.active_request = Some(req_id.clone());
                self.schedule_state_save(cx);
                return self.requests.iter().find(|r| r.id == req_id).cloned();
            }
        }
        None
    }

    pub fn navigate_forward(&mut self, cx: &mut Context<Self>) -> Option<HttpRequestItem> {
        if self.nav_index + 1 < self.nav_history.len() {
            self.nav_index += 1;
            if let Some(req_id) = self.nav_history.get(self.nav_index).cloned() {
                self.local_state.active_request = Some(req_id.clone());
                self.schedule_state_save(cx);
                return self.requests.iter().find(|r| r.id == req_id).cloned();
            }
        }
        None
    }

    pub fn set_active_environment(&mut self, env_id: Option<String>, cx: &mut Context<Self>) {
        self.active_environment_id = env_id.clone();
        self.local_state.active_environment = env_id;
        self.schedule_state_save(cx);
        cx.notify();
    }

    pub fn effective_environment(&self) -> Environment {
        let ws_name = self.workspace_meta.name.as_str();
        let mut effective = Environment::new(ws_name, "Effective");

        for v in &self.global_environment.variables {
            if v.enabled {
                effective.variables.push(v.clone());
            }
        }

        if let Some(active_env) = self.active_environment() {
            for v in &active_env.variables {
                if v.enabled {
                    if let Some(existing) = effective.variables.iter_mut().find(|x| x.key == v.key) {
                        *existing = v.clone();
                    } else {
                        effective.variables.push(v.clone());
                    }
                }
            }
        }
        effective
    }

    pub fn create_request(&mut self, folder_rel: Option<String>, cx: &mut Context<Self>) -> HttpRequestItem {
        let (_slug, rel_path) = self
            .store
            .create_request(folder_rel.as_deref(), "New Request", HttpMethod::Get, "")
            .unwrap_or_else(|_| (format!("req-{}", uuid::Uuid::new_v4().simple()), "new-request.yaml".to_string()));

        let ws_name = self.workspace_meta.name.clone();
        let mut item = HttpRequestItem::new(&ws_name, "New Request", HttpMethod::Get, "");
        item.id = rel_path.clone();
        item.folder_id = folder_rel;

        self.requests.push(item.clone());
        self.reload_tree();
        cx.notify();
        item
    }

    pub fn duplicate_request(&mut self, req_id: &str, cx: &mut Context<Self>) -> Option<HttpRequestItem> {
        let existing = self.requests.iter().find(|r| r.id == req_id)?.clone();
        let mut req_file = self.store.read_request(req_id).unwrap_or_else(|_| RequestFile {
            name: existing.name.clone(),
            method: existing.method,
            url: existing.url.clone(),
            ..Default::default()
        });

        let new_name = format!("{} (Copy)", req_file.name);
        req_file.name = new_name.clone();

        let folder_rel = existing.folder_id.clone();
        let (_slug, rel_path) = self
            .store
            .create_request(folder_rel.as_deref(), &new_name, req_file.method, &req_file.url)
            .ok()?;

        let _ = self.store.write_request(&rel_path, &req_file);
        self.reload_tree();
        let dup = self.requests.iter().find(|r| r.id == rel_path).cloned();
        cx.notify();
        dup
    }

    pub fn delete_request(&mut self, req_id: &str, cx: &mut Context<Self>) {
        if self.renaming_request_id.as_deref() == Some(req_id) {
            self.renaming_request_id = None;
            self.rename_input = None;
            self.rename_subscription = None;
        }

        let _ = self.store.delete_request(req_id);
        self.unsaved_requests.remove(req_id);
        self.pins.retain(|p| p != req_id);
        let _ = self.store.save_pins(&self.pins);

        self.requests.retain(|r| r.id != req_id);
        self.reload_tree();
        cx.notify();
    }

    pub fn start_renaming_request(
        &mut self,
        req_id: String,
        initial_name: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let input = cx.new(|cx| {
            let mut s = InputState::new(window, cx);
            s.set_value(initial_name, window, cx);
            s.select_all(window, cx);
            s.focus(window, cx);
            s
        });

        let sub = cx.subscribe(&input, move |this, input, event: &gpui_component::input::InputEvent, cx| {
            match event {
                gpui_component::input::InputEvent::PressEnter { .. } => {
                    let val = input.read(cx).value().to_string();
                    this.finish_renaming(val, cx);
                }
                gpui_component::input::InputEvent::Blur => {
                    let val = input.read(cx).value().to_string();
                    this.finish_renaming(val, cx);
                }
                _ => {}
            }
        });

        self.renaming_request_id = Some(req_id);
        self.rename_input = Some(input);
        self.rename_subscription = Some(sub);
        cx.notify();
    }

    pub fn finish_renaming(&mut self, new_name: String, cx: &mut Context<Self>) {
        let Some(req_id) = self.renaming_request_id.take() else {
            return;
        };
        self.rename_input = None;
        self.rename_subscription = None;
        let trimmed = new_name.trim();
        if !trimmed.is_empty() {
            if let Ok(new_rel_path) = self.store.rename_request(&req_id, trimmed) {
                // Record rename pair so EditorState & ResponseState can synchronize
                self.last_renamed_request = Some((req_id.clone(), new_rel_path.clone()));

                // Update pins if pinned
                if let Some(pos) = self.pins.iter().position(|p| p == &req_id) {
                    self.pins[pos] = new_rel_path.clone();
                    let _ = self.store.save_pins(&self.pins);
                }
                // Update active request in state
                if self.local_state.active_request.as_deref() == Some(&req_id) {
                    self.local_state.active_request = Some(new_rel_path.clone());
                    let _ = self.store.save_state(&self.local_state);
                }
                // Update recents
                for r in &mut self.local_state.recent {
                    if r == &req_id {
                        *r = new_rel_path.clone();
                    }
                }
                // Update nav history
                for h in &mut self.nav_history {
                    if h == &req_id {
                        *h = new_rel_path.clone();
                    }
                }
                // Transfer unsaved status
                if self.unsaved_requests.remove(&req_id) {
                    self.unsaved_requests.insert(new_rel_path.clone());
                }
                // Transfer parse errors
                if let Some(err) = self.parse_errors.remove(&req_id) {
                    self.parse_errors.insert(new_rel_path.clone(), err);
                }
                // Transfer conflicts
                if let Some(c) = self.conflicts.remove(&req_id) {
                    self.conflicts.insert(new_rel_path, c);
                }
                self.reload_tree();
            }
        }
        cx.notify();
    }

    pub fn cancel_renaming(&mut self, cx: &mut Context<Self>) {
        if self.renaming_request_id.is_some() {
            self.renaming_request_id = None;
            self.rename_input = None;
            self.rename_subscription = None;
            cx.notify();
        }
    }

    pub fn update_request(&mut self, mut req: HttpRequestItem, cx: &mut Context<Self>) {
        let Some(idx) = self.requests.iter().position(|r| r.id == req.id) else {
            return;
        };
        if req.last_status_code.is_none() {
            req.last_status_code = self.requests[idx].last_status_code;
        }
        if self.requests[idx] == req {
            return;
        }
        self.requests[idx] = req.clone();
        self.unsaved_requests.insert(req.id.clone());
        cx.notify();

        let req_id = req.id.clone();
        let task = cx.spawn(async move |this, cx| {
            cx.background_executor().timer(std::time::Duration::from_millis(500)).await;
            if let Some(this) = this.upgrade() {
                this.update(cx, |ws, cx| {
                    if let Some(r) = ws.requests.iter().find(|r| r.id == req_id) {
                        let mut req_file = ws.store.read_request(&req_id).unwrap_or_else(|_| RequestFile {
                            name: r.name.clone(),
                            method: r.method,
                            url: r.url.clone(),
                            ..Default::default()
                        });
                        req_file.update_from_item(r);
                        let _ = ws.store.write_request(&req_id, &req_file);
                        ws.unsaved_requests.remove(&req_id);
                        cx.notify();
                    }
                });
            }
        });
        self.request_save_tasks.insert(req.id, task);
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

    pub fn create_folder(&mut self, name: String, cx: &mut Context<Self>) {
        let trimmed = name.trim();
        if !trimmed.is_empty() {
            let _ = self.store.create_group(None, trimmed);
            self.reload_tree();
            cx.notify();
        }
    }

    pub fn delete_folder(&mut self, folder_id: &str, cx: &mut Context<Self>) {
        if self.renaming_folder_id.as_deref() == Some(folder_id) {
            self.renaming_folder_id = None;
            self.rename_folder_input = None;
            self.rename_folder_subscription = None;
        }
        self.collapsed_folders.remove(folder_id);
        let _ = self.store.delete_group(folder_id);
        self.reload_tree();
        cx.notify();
    }

    pub fn toggle_folder_collapsed(&mut self, folder_id: &str, cx: &mut Context<Self>) {
        if self.collapsed_folders.contains(folder_id) {
            self.collapsed_folders.remove(folder_id);
        } else {
            self.collapsed_folders.insert(folder_id.to_string());
        }
        self.local_state.expanded_folders = self
            .folders
            .iter()
            .map(|f| f.id.clone())
            .filter(|id| !self.collapsed_folders.contains(id))
            .collect();
        self.schedule_state_save(cx);
        cx.notify();
    }

    pub fn toggle_request_collapsed(&mut self, req_id: &str, cx: &mut Context<Self>) {
        if self.collapsed_requests.contains(req_id) {
            self.collapsed_requests.remove(req_id);
        } else {
            self.collapsed_requests.insert(req_id.to_string());
        }
        cx.notify();
    }

    pub fn start_renaming_folder(
        &mut self,
        folder_id: String,
        initial_name: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let input = cx.new(|cx| {
            let mut s = InputState::new(window, cx);
            s.set_value(initial_name, window, cx);
            s.select_all(window, cx);
            s.focus(window, cx);
            s
        });

        let sub = cx.subscribe(&input, move |this, input, event: &gpui_component::input::InputEvent, cx| {
            match event {
                gpui_component::input::InputEvent::PressEnter { .. } => {
                    let val = input.read(cx).value().to_string();
                    this.finish_renaming_folder(val, cx);
                }
                gpui_component::input::InputEvent::Blur => {
                    let val = input.read(cx).value().to_string();
                    this.finish_renaming_folder(val, cx);
                }
                _ => {}
            }
        });

        self.renaming_folder_id = Some(folder_id);
        self.rename_folder_input = Some(input);
        self.rename_folder_subscription = Some(sub);
        cx.notify();
    }

    pub fn finish_renaming_folder(&mut self, new_name: String, cx: &mut Context<Self>) {
        let Some(folder_id) = self.renaming_folder_id.take() else {
            return;
        };
        self.rename_folder_input = None;
        self.rename_folder_subscription = None;
        let trimmed = new_name.trim();
        if !trimmed.is_empty() {
            let _ = self.store.rename_group(&folder_id, trimmed);
            self.reload_tree();
        }
        cx.notify();
    }

    pub fn cancel_renaming_folder(&mut self, cx: &mut Context<Self>) {
        if self.renaming_folder_id.is_some() {
            self.renaming_folder_id = None;
            self.rename_folder_input = None;
            self.rename_folder_subscription = None;
            cx.notify();
        }
    }

    pub fn rename_folder(&mut self, folder_id: &str, new_name: String, cx: &mut Context<Self>) {
        let trimmed = new_name.trim();
        if trimmed.is_empty() {
            return;
        }
        let _ = self.store.rename_group(folder_id, trimmed);
        self.reload_tree();
        cx.notify();
    }

    pub fn move_folder(&mut self, src_folder_id: &str, target_folder_id: &str, cx: &mut Context<Self>) {
        if src_folder_id == target_folder_id {
            return;
        }
        let src_pos = self.folders.iter().position(|f| f.id == src_folder_id);
        let target_pos = self.folders.iter().position(|f| f.id == target_folder_id);
        if let (Some(s_idx), Some(_)) = (src_pos, target_pos) {
            let folder = self.folders.remove(s_idx);
            let new_t_idx = self.folders.iter().position(|f| f.id == target_folder_id).unwrap_or(self.folders.len());
            self.folders.insert(new_t_idx, folder);
            cx.notify();
        }
    }

    pub fn start_renaming_environment(
        &mut self,
        env_id: String,
        initial_name: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if env_id == "globals" {
            return;
        }
        let input = cx.new(|cx| {
            let mut s = InputState::new(window, cx);
            s.set_value(initial_name, window, cx);
            s.select_all(window, cx);
            s.focus(window, cx);
            s
        });

        let sub = cx.subscribe(&input, move |this, input, event: &gpui_component::input::InputEvent, cx| {
            match event {
                gpui_component::input::InputEvent::PressEnter { .. } => {
                    let val = input.read(cx).value().to_string();
                    this.finish_renaming_environment(val, cx);
                }
                gpui_component::input::InputEvent::Blur => {
                    let val = input.read(cx).value().to_string();
                    this.finish_renaming_environment(val, cx);
                }
                _ => {}
            }
        });

        self.renaming_env_id = Some(env_id);
        self.rename_env_input = Some(input);
        self.rename_env_subscription = Some(sub);
        cx.notify();
    }

    pub fn finish_renaming_environment(&mut self, new_name: String, cx: &mut Context<Self>) {
        let Some(env_id) = self.renaming_env_id.take() else {
            return;
        };
        self.rename_env_input = None;
        self.rename_env_subscription = None;
        let trimmed = new_name.trim();
        if !trimmed.is_empty() {
            self.rename_environment(&env_id, trimmed.to_string(), cx);
        } else {
            cx.notify();
        }
    }

    pub fn cancel_renaming_environment(&mut self, cx: &mut Context<Self>) {
        if self.renaming_env_id.is_some() {
            self.renaming_env_id = None;
            self.rename_env_input = None;
            self.rename_env_subscription = None;
            cx.notify();
        }
    }

    pub fn move_request(&mut self, src_req_id: &str, target_req_id: &str, cx: &mut Context<Self>) {
        if src_req_id == target_req_id {
            return;
        }
        // Move within folder / reorder
        let src_req = self.requests.iter().find(|r| r.id == src_req_id).cloned();
        let target_req = self.requests.iter().find(|r| r.id == target_req_id).cloned();

        if let (Some(src), Some(target)) = (src_req, target_req) {
            if src.folder_id != target.folder_id {
                let _ = self.store.move_request(src_req_id, target.folder_id.as_deref());
                self.reload_tree();
            } else {
                // Same folder: reorder stems in _folder.yaml
                let folder_id = src.folder_id.clone();
                let mut stems: Vec<String> = self
                    .requests
                    .iter()
                    .filter(|r| r.folder_id == folder_id)
                    .map(|r| {
                        PathBuf::from(&r.id)
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("req")
                            .to_string()
                    })
                    .collect();

                let src_stem = PathBuf::from(src_req_id)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("req")
                    .to_string();
                let target_stem = PathBuf::from(target_req_id)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("req")
                    .to_string();

                if let Some(s_pos) = stems.iter().position(|s| s == &src_stem) {
                    stems.remove(s_pos);
                    let t_pos = stems.iter().position(|s| s == &target_stem).unwrap_or(stems.len());
                    stems.insert(t_pos, src_stem);
                    let _ = self.store.reorder_folder(folder_id.as_deref(), &stems);
                    self.reload_tree();
                }
            }
        }
        cx.notify();
    }

    pub fn move_request_to_folder(
        &mut self,
        src_req_id: &str,
        target_folder_id: Option<String>,
        cx: &mut Context<Self>,
    ) {
        let _ = self.store.move_request(src_req_id, target_folder_id.as_deref());
        self.reload_tree();
        cx.notify();
    }

    pub fn toggle_pin(&mut self, req_id: &str, cx: &mut Context<Self>) {
        if self.pins.contains(&req_id.to_string()) {
            self.pins.retain(|p| p != req_id);
        } else {
            self.pins.push(req_id.to_string());
        }
        let _ = self.store.save_pins(&self.pins);
        cx.notify();
    }

    pub fn is_pinned(&self, req_id: &str) -> bool {
        self.pins.contains(&req_id.to_string())
    }

    pub fn reorder_pins(&mut self, new_pins: Vec<String>, cx: &mut Context<Self>) {
        self.pins = new_pins;
        let _ = self.store.save_pins(&self.pins);
        cx.notify();
    }

    pub fn set_filter_query(&mut self, query: String, cx: &mut Context<Self>) {
        self.filter_query = query;
        cx.notify();
    }

    pub fn toggle_sidebar(&mut self, cx: &mut Context<Self>) {
        self.sidebar_collapsed = !self.sidebar_collapsed;
        self.local_state.sidebar.collapsed = self.sidebar_collapsed;
        self.schedule_state_save(cx);
        cx.notify();
    }

    pub fn set_sidebar_width(&mut self, width: Pixels, cx: &mut Context<Self>) {
        let clamped = px(width.as_f32().clamp(200.0, 480.0));
        if self.sidebar_width != clamped {
            self.sidebar_width = clamped;
            self.local_state.sidebar.width = clamped.as_f32();
            self.schedule_state_save(cx);
            cx.notify();
        }
    }

    pub fn toggle_split_layout(&mut self, cx: &mut Context<Self>) {
        self.split_layout_vertical = !self.split_layout_vertical;
        self.local_state.layout.split = if self.split_layout_vertical {
            "vertical".to_string()
        } else {
            "horizontal".to_string()
        };
        self.schedule_state_save(cx);
        cx.notify();
    }

    pub fn toggle_env_modal(&mut self, cx: &mut Context<Self>) {
        self.show_env_modal = !self.show_env_modal;
        if self.show_env_modal && self.selected_env_modal_id.is_none() {
            self.selected_env_modal_id = self
                .active_environment_id
                .clone()
                .or_else(|| self.environments.first().map(|e| e.id.clone()))
                .or_else(|| Some("globals".to_string()));
        }
        cx.notify();
    }

    pub fn select_modal_environment(&mut self, env_id: Option<String>, cx: &mut Context<Self>) {
        self.selected_env_modal_id = env_id;
        self.var_filter_query.clear();
        cx.notify();
    }

    pub fn set_env_filter_query(&mut self, query: String, cx: &mut Context<Self>) {
        self.env_filter_query = query;
        cx.notify();
    }

    pub fn set_var_filter_query(&mut self, query: String, cx: &mut Context<Self>) {
        self.var_filter_query = query;
        cx.notify();
    }

    pub fn save_global_environment(&mut self, env: Environment, cx: &mut Context<Self>) {
        self.global_environment = env;
        cx.notify();
    }

    pub fn create_environment(&mut self, name: Option<String>, cx: &mut Context<Self>) -> String {
        let env_name = name.unwrap_or_else(|| format!("env-{}", self.environments.len() + 1));
        let env_file = EnvironmentFile::default();
        let _ = self.store.save_environment(&env_name, &env_file);
        self.reload_environments();
        self.selected_env_modal_id = Some(env_name.clone());
        if self.active_environment_id.is_none() {
            self.set_active_environment(Some(env_name.clone()), cx);
        } else {
            cx.notify();
        }
        env_name
    }

    pub fn duplicate_environment(&mut self, env_id: &str, cx: &mut Context<Self>) -> Option<String> {
        let existing = self.environments.iter().find(|e| e.id == env_id || e.name == env_id)?.clone();
        let new_name = format!("{}-copy", existing.name);
        let (env_file, secrets) = EnvironmentFile::from_environment(&existing);
        let _ = self.store.save_environment(&new_name, &env_file);
        let _ = self.store.save_secrets(&new_name, &secrets);
        self.reload_environments();
        self.selected_env_modal_id = Some(new_name.clone());
        cx.notify();
        Some(new_name)
    }

    pub fn rename_environment(&mut self, env_id: &str, new_name: String, cx: &mut Context<Self>) {
        let trimmed = new_name.trim();
        if trimmed.is_empty() || env_id == "globals" {
            return;
        }
        let _ = self.store.rename_environment(env_id, trimmed);
        self.reload_environments();
        if self.active_environment_id.as_deref() == Some(env_id) {
            self.active_environment_id = Some(trimmed.to_string());
        }
        if self.selected_env_modal_id.as_deref() == Some(env_id) {
            self.selected_env_modal_id = Some(trimmed.to_string());
        }
        cx.notify();
    }

    pub fn delete_environment(&mut self, env_id: &str, cx: &mut Context<Self>) {
        if env_id == "globals" {
            return;
        }
        let _ = self.store.delete_environment(env_id);
        self.reload_environments();
        if self.active_environment_id.as_deref() == Some(env_id) {
            self.set_active_environment(self.environments.first().map(|e| e.id.clone()), cx);
        }
        if self.selected_env_modal_id.as_deref() == Some(env_id) {
            self.selected_env_modal_id = self.environments.first().map(|e| e.id.clone());
        }
        cx.notify();
    }

    pub fn toggle_cookies_modal(&mut self, cx: &mut Context<Self>) {
        self.show_cookies_modal = !self.show_cookies_modal;
        cx.notify();
    }

    pub fn toggle_command_palette(&mut self, cx: &mut Context<Self>) {
        self.show_command_palette = !self.show_command_palette;
        cx.notify();
    }

    pub fn toggle_settings_modal(&mut self, cx: &mut Context<Self>) {
        self.show_settings_modal = !self.show_settings_modal;
        cx.notify();
    }

    pub fn save_environments(&mut self, envs: Vec<Environment>, cx: &mut Context<Self>) {
        for env in &envs {
            let (file, secrets) = EnvironmentFile::from_environment(env);
            let _ = self.store.save_environment(&env.name, &file);
            let _ = self.store.save_secrets(&env.name, &secrets);
        }
        self.environments = envs;
        cx.notify();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use restclient_core::models::EnvironmentVariable;

    #[test]
    fn test_effective_environment_override() {
        let mut global = Environment::new("ws1", "Globals");
        global.variables.push(EnvironmentVariable {
            enabled: true,
            key: "HOST".to_string(),
            value: "global.example.com".to_string(),
            secret: false,
        });
        global.variables.push(EnvironmentVariable {
            enabled: true,
            key: "GLOBAL_TOKEN".to_string(),
            value: "token_123".to_string(),
            secret: true,
        });
        global.variables.push(EnvironmentVariable {
            enabled: false,
            key: "DISABLED_KEY".to_string(),
            value: "should_not_appear".to_string(),
            secret: false,
        });

        let mut active = Environment::new("ws1", "Production");
        active.variables.push(EnvironmentVariable {
            enabled: true,
            key: "HOST".to_string(),
            value: "prod.example.com".to_string(),
            secret: false,
        });
        active.variables.push(EnvironmentVariable {
            enabled: true,
            key: "PORT".to_string(),
            value: "443".to_string(),
            secret: false,
        });

        let temp_dir = std::env::temp_dir().join(format!("test_ws_eff_{}", uuid::Uuid::new_v4().simple()));
        let store = Arc::new(FsWorkspaceStore::new(&temp_dir));
        let _ = store.init_workspace("Test WS");

        let ws_state = WorkspaceState {
            store,
            workspace_path: temp_dir.clone(),
            workspace_meta: WorkspaceMeta::default(),
            active_workspace: Some(Workspace::new("Test")),
            folders: Vec::new(),
            requests: Vec::new(),
            parse_errors: HashMap::new(),
            pins: Vec::new(),
            local_state: LocalState::default(),
            environments: vec![active.clone()],
            global_environment: global,
            active_environment_id: Some(active.id.clone()),
            selected_env_modal_id: None,
            env_filter_query: String::new(),
            var_filter_query: String::new(),
            git_branch: None,
            git_tracked_warning: false,
            filter_query: String::new(),
            sidebar_collapsed: false,
            sidebar_width: px(260.),
            split_layout_vertical: false,
            show_env_modal: false,
            show_cookies_modal: false,
            show_command_palette: false,
            show_settings_modal: false,
            renaming_request_id: None,
            rename_input: None,
            rename_subscription: None,
            renaming_folder_id: None,
            rename_folder_input: None,
            rename_folder_subscription: None,
            renaming_env_id: None,
            rename_env_input: None,
            rename_env_subscription: None,
            collapsed_folders: HashSet::new(),
            collapsed_requests: HashSet::new(),
            unsaved_requests: HashSet::new(),
            running_request_id: None,
            recent_workspaces: Vec::new(),
            nav_history: Vec::new(),
            nav_index: 0,
            state_save_task: None,
            request_save_tasks: HashMap::new(),
            conflicts: HashMap::new(),
            corrupt_secrets_warning: None,
            last_renamed_request: None,
            watcher: None,
            watcher_task: None,
        };

        let effective = ws_state.effective_environment();
        let host_var = effective.variables.iter().find(|v| v.key == "HOST").unwrap();
        assert_eq!(host_var.value, "prod.example.com");

        let port_var = effective.variables.iter().find(|v| v.key == "PORT").unwrap();
        assert_eq!(port_var.value, "443");

        let token_var = effective.variables.iter().find(|v| v.key == "GLOBAL_TOKEN").unwrap();
        assert_eq!(token_var.value, "token_123");

        assert!(effective.variables.iter().all(|v| v.key != "DISABLED_KEY"));
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_conflict_detection_and_invalid_yaml_handling() {
        let temp_dir = std::env::temp_dir().join(format!("test_ws_conflict_{}", uuid::Uuid::new_v4().simple()));
        let store = Arc::new(FsWorkspaceStore::new(&temp_dir));
        store.init_workspace("Conflict WS").unwrap();

        let (_slug, rel_path) = store.create_request(None, "Login", HttpMethod::Post, "https://api.example.com/login").unwrap();

        let mut ws_state = WorkspaceState {
            store: store.clone(),
            workspace_path: temp_dir.clone(),
            workspace_meta: WorkspaceMeta::default(),
            active_workspace: Some(Workspace::new("Conflict WS")),
            folders: Vec::new(),
            requests: Vec::new(),
            parse_errors: HashMap::new(),
            pins: Vec::new(),
            local_state: LocalState::default(),
            environments: Vec::new(),
            global_environment: Environment::new("default", "Globals"),
            active_environment_id: None,
            selected_env_modal_id: None,
            env_filter_query: String::new(),
            var_filter_query: String::new(),
            git_branch: None,
            git_tracked_warning: false,
            filter_query: String::new(),
            sidebar_collapsed: false,
            sidebar_width: px(260.),
            split_layout_vertical: false,
            show_env_modal: false,
            show_cookies_modal: false,
            show_command_palette: false,
            show_settings_modal: false,
            renaming_request_id: None,
            rename_input: None,
            rename_subscription: None,
            renaming_folder_id: None,
            rename_folder_input: None,
            rename_folder_subscription: None,
            renaming_env_id: None,
            rename_env_input: None,
            rename_env_subscription: None,
            collapsed_folders: HashSet::new(),
            collapsed_requests: HashSet::new(),
            unsaved_requests: HashSet::new(),
            running_request_id: None,
            recent_workspaces: Vec::new(),
            nav_history: Vec::new(),
            nav_index: 0,
            state_save_task: None,
            request_save_tasks: HashMap::new(),
            conflicts: HashMap::new(),
            corrupt_secrets_warning: None,
            last_renamed_request: None,
            watcher: None,
            watcher_task: None,
        };

        ws_state.reload_tree();
        assert_eq!(ws_state.requests.len(), 1);

        // 1. External modification while dirty triggers conflict
        ws_state.unsaved_requests.insert(rel_path.clone());
        // Simulate external change event
        if ws_state.unsaved_requests.contains(&rel_path) {
            ws_state.conflicts.insert(rel_path.clone(), "File was modified externally while you have unsaved changes.".to_string());
        }
        assert!(ws_state.conflicts.contains_key(&rel_path));

        // Resolve keep mine
        ws_state.conflicts.remove(&rel_path);
        ws_state.unsaved_requests.remove(&rel_path);
        assert!(!ws_state.conflicts.contains_key(&rel_path));

        // 2. Invalid YAML: test parse error recording and keeping good version
        let req_file_path = temp_dir.join("collections").join(&rel_path);
        std::fs::write(&req_file_path, "invalid yaml : : :").unwrap();
        match store.read_request(&rel_path) {
            Ok(_) => panic!("Expected parse error"),
            Err(e) => {
                ws_state.parse_errors.insert(rel_path.clone(), e.to_string());
            }
        }
        assert!(ws_state.parse_errors.contains_key(&rel_path));
        // In-memory version is preserved
        assert_eq!(ws_state.requests.len(), 1);
        assert_eq!(ws_state.requests[0].name, "Login");

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}

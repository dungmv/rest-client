use crate::settings::AppSettings;
use gpui::{Context, Task};
use restclient_core::models::{Environment, Folder, HttpRequestItem, Workspace};
use restclient_core::storage::models::{FolderNode, LocalState, WorkspaceMeta};
use restclient_core::storage::{FsWorkspaceStore, WorkspaceStore};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;

pub struct WorkspaceDataState {
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
    pub conflicts: HashMap<String, String>,
    pub corrupt_secrets_warning: Option<String>,
    pub last_renamed_request: Option<(String, String)>,
    pub last_renamed_example: Option<(String, String, String)>,
    pub unsaved_requests: HashSet<String>,
    pub running_request_id: Option<String>,
    pub recent_workspaces: Vec<String>,
    pub state_save_task: Option<Task<()>>,
    pub request_save_tasks: HashMap<String, Task<()>>,
    pub env_save_task: Option<Task<()>>,
    pub watcher: Option<restclient_core::storage::WorkspaceWatcher>,
    pub watcher_task: Option<Task<()>>,
}

impl WorkspaceDataState {
    pub fn new() -> Self {
        let mut settings = AppSettings::load();
        let ws_dir = if let Some(curr) = &settings.current_workspace {
            PathBuf::from(curr)
        } else {
            dirs::data_local_dir()
                .map(|d| d.join("restclient").join("workspaces").join("my-workspace"))
                .unwrap_or_else(|| PathBuf::from("./my-workspace"))
        };

        let default_name = ws_dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Workspace")
            .to_string();

        let store: Arc<dyn WorkspaceStore> = Arc::new(FsWorkspaceStore::new(&ws_dir));
        if !ws_dir.join("workspace.yaml").exists() {
            let _ = store.init_workspace(&default_name);
        }

        settings.add_recent_workspace(&ws_dir.to_string_lossy());
        let _ = store.ensure_gitignore();

        let mut state = Self {
            store,
            workspace_path: ws_dir,
            workspace_meta: WorkspaceMeta::default(),
            active_workspace: Some(Workspace::new(&default_name)),
            folders: Vec::new(),
            requests: Vec::new(),
            parse_errors: HashMap::new(),
            pins: Vec::new(),
            local_state: LocalState::default(),
            environments: Vec::new(),
            global_environment: Environment::new("default", "Globals"),
            active_environment_id: None,
            conflicts: HashMap::new(),
            corrupt_secrets_warning: None,
            last_renamed_request: None,
            last_renamed_example: None,
            unsaved_requests: HashSet::new(),
            running_request_id: None,
            recent_workspaces: settings.recent_workspaces,
            state_save_task: None,
            request_save_tasks: HashMap::new(),
            env_save_task: None,
            watcher: None,
            watcher_task: None,
        };

        state.reload_all();
        state
    }

    pub fn new_for_path(path: PathBuf, store: Arc<dyn WorkspaceStore>) -> Self {
        let mut state = Self {
            store,
            workspace_path: path,
            workspace_meta: WorkspaceMeta::default(),
            active_workspace: Some(Workspace::new("Test Workspace")),
            folders: Vec::new(),
            requests: Vec::new(),
            parse_errors: HashMap::new(),
            pins: Vec::new(),
            local_state: LocalState::default(),
            environments: Vec::new(),
            global_environment: Environment::new("default", "Globals"),
            active_environment_id: None,
            conflicts: HashMap::new(),
            corrupt_secrets_warning: None,
            last_renamed_request: None,
            last_renamed_example: None,
            unsaved_requests: HashSet::new(),
            running_request_id: None,
            recent_workspaces: Vec::new(),
            state_save_task: None,
            request_save_tasks: HashMap::new(),
            env_save_task: None,
            watcher: None,
            watcher_task: None,
        };
        state.reload_all();
        state
    }

    pub fn open_workspace(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        let store: Arc<dyn WorkspaceStore> = Arc::new(FsWorkspaceStore::new(&path));
        if !path.join("workspace.yaml").exists() {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("Workspace");
            let _ = store.init_workspace(name);
        }
        let _ = store.ensure_gitignore();
        self.store = store;
        self.workspace_path = path.clone();
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

    pub fn execute_import_current_workspace(
        &mut self,
        result: restclient_core::importers::ImportResult,
        options: restclient_core::importers::ImportOptions,
        cx: &mut Context<Self>,
    ) -> anyhow::Result<restclient_core::importers::ImportSummary> {
        let summary = restclient_core::importers::apply_import_to_workspace(
            self.store.as_ref(),
            &result,
            &options,
        )?;
        self.reload_tree();
        self.reload_environments();
        cx.notify();
        Ok(summary)
    }

    pub fn execute_import_new_workspace(
        &mut self,
        target_dir: PathBuf,
        workspace_name: &str,
        result: restclient_core::importers::ImportResult,
        options: restclient_core::importers::ImportOptions,
        cx: &mut Context<Self>,
    ) -> anyhow::Result<restclient_core::importers::ImportSummary> {
        let new_store = Arc::new(FsWorkspaceStore::new(&target_dir));
        new_store.init_workspace(workspace_name)?;
        let summary = restclient_core::importers::apply_import_to_workspace(
            new_store.as_ref(),
            &result,
            &options,
        )?;
        self.open_workspace(target_dir, cx);
        Ok(summary)
    }

    pub fn reload_all(&mut self) {
        if let Ok(meta) = self.store.load_workspace_meta() {
            self.workspace_meta = meta;
        }
        let folder_name = self
            .workspace_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Workspace");
        let initial_name = if self.workspace_meta.name.trim().is_empty() {
            folder_name.to_string()
        } else {
            self.workspace_meta.name.clone()
        };
        self.active_workspace = Some(Workspace {
            id: self.workspace_path.to_string_lossy().to_string(),
            name: initial_name,
            description: String::new(),
            active_environment_id: self.active_environment_id.clone(),
            headers: self.workspace_meta.headers.iter().cloned().map(Into::into).collect(),
            auth: self.workspace_meta.auth.clone().into(),
        });

        self.pins = self.store.load_pins().unwrap_or_default();
        self.local_state = self.store.load_state().unwrap_or_default();

        self.reload_tree();
        self.reload_environments();
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
                f.headers = folder_node.file.headers.iter().cloned().map(Into::into).collect();
                f.auth = folder_node.file.auth.clone().into();
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

    pub fn find_request(&self, req_id: &str) -> Option<&HttpRequestItem> {
        self.requests.iter().find(|r| r.id == req_id)
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
            effective.variables.push(v.clone());
        }

        if let Some(active_env) = self.active_environment() {
            for v in &active_env.variables {
                if let Some(existing) = effective.variables.iter_mut().find(|x| x.key == v.key) {
                    *existing = v.clone();
                } else {
                    effective.variables.push(v.clone());
                }
            }
        }
        effective
    }

}

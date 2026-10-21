use crate::platform::detect_git_branch;
use crate::settings::AppSettings;
use crate::state::EditorState;
use gpui::{AppContext as _, Context, Entity, Pixels, Task, Window, px};
use gpui_component::input::InputState;
use restclient_core::models::{
    Auth, Environment, Folder, HttpMethod, HttpRequestItem, KeyValuePair, Workspace,
};
use restclient_core::storage::models::{
    EnvironmentFile, FolderNode, LocalState, RequestFile, WorkspaceMeta,
};
use restclient_core::storage::{FsWorkspaceStore, WorkspaceStore};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GitTab {
    #[default]
    Changes,
    History,
    Branches,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitActiveModal {
    Commit,
    History,
    NewBranch,
    Remotes,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScopeSettingsTarget {
    Workspace,
    Folder(String),
}

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
    pub scope_settings_target: Option<ScopeSettingsTarget>,
    pub theme_preference: crate::theme::ThemePreference,
    pub show_git_modal: bool,
    pub show_rename_workspace_modal: bool,
    pub show_import_modal: bool,
    pub import_target_folder_rel: Option<String>,
    pub active_git_modal: Option<GitActiveModal>,
    pub git_tab: GitTab,
    pub git_status: Option<restclient_core::git::GitStatusResult>,
    pub git_history: Vec<restclient_core::git::GitCommitInfo>,
    pub git_branches: Vec<restclient_core::git::GitBranchInfo>,
    pub git_remotes: Vec<restclient_core::git::GitRemote>,
    pub git_selected_staged_files: HashSet<String>,
    pub git_selected_unstaged_files: HashSet<String>,
    pub git_selected_file: Option<(String, bool)>,
    pub git_diff_content: Option<String>,
    pub git_selected_commit: Option<restclient_core::git::GitCommitDetail>,
    pub git_expanded_history_files: HashSet<String>,
    pub git_commit_message: String,
    pub git_error: Option<String>,
    pub git_loading: bool,
    pub git_is_syncing: bool,
    pub renaming_request_id: Option<String>,
    pub rename_input: Option<Entity<InputState>>,
    pub rename_subscription: Option<gpui::Subscription>,
    pub renaming_folder_id: Option<String>,
    pub rename_folder_input: Option<Entity<InputState>>,
    pub rename_folder_subscription: Option<gpui::Subscription>,
    pub renaming_env_id: Option<String>,
    pub rename_env_input: Option<Entity<InputState>>,
    pub rename_env_subscription: Option<gpui::Subscription>,
    pub renaming_example: Option<(String, String)>,
    pub rename_example_input: Option<Entity<InputState>>,
    pub rename_example_subscription: Option<gpui::Subscription>,
    pub rename_workspace_input: Option<Entity<InputState>>,
    pub rename_workspace_subscription: Option<gpui::Subscription>,
    pub last_renamed_example: Option<(String, String, String)>,
    pub collapsed_folders: HashSet<String>,
    pub collapsed_requests: HashSet<String>,
    pub unsaved_requests: HashSet<String>,
    pub running_request_id: Option<String>,
    pub recent_workspaces: Vec<String>,
    pub nav_history: Vec<String>,
    pub nav_index: usize,
    pub state_save_task: Option<Task<()>>,
    pub request_save_tasks: HashMap<String, Task<()>>,
    pub env_save_task: Option<Task<()>>,
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
        let git_branch = detect_git_branch(Some(&ws_dir));
        let git_tracked_warning = store.ensure_gitignore().unwrap_or(false);

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
            scope_settings_target: None,
            theme_preference: crate::theme::ThemePreference::from_str_opt(settings.theme.as_deref()),
            show_git_modal: false,
            show_rename_workspace_modal: false,
            show_import_modal: false,
            import_target_folder_rel: None,
            active_git_modal: None,
            git_tab: GitTab::Changes,
            git_status: None,
            git_history: Vec::new(),
            git_branches: Vec::new(),
            git_remotes: Vec::new(),
            git_selected_staged_files: HashSet::new(),
            git_selected_unstaged_files: HashSet::new(),
            git_selected_file: None,
            git_diff_content: None,
            git_selected_commit: None,
            git_expanded_history_files: HashSet::new(),
            git_commit_message: String::new(),
            git_error: None,
            git_loading: false,
            git_is_syncing: false,
            renaming_request_id: None,
            rename_input: None,
            rename_subscription: None,
            renaming_folder_id: None,
            rename_folder_input: None,
            rename_folder_subscription: None,
            renaming_env_id: None,
            rename_env_input: None,
            rename_env_subscription: None,
            renaming_example: None,
            rename_example_input: None,
            rename_example_subscription: None,
            rename_workspace_input: None,
            rename_workspace_subscription: None,
            last_renamed_example: None,
            collapsed_folders: HashSet::new(),
            collapsed_requests: HashSet::new(),
            unsaved_requests: HashSet::new(),
            running_request_id: None,
            recent_workspaces: settings.recent_workspaces,
            nav_history: Vec::new(),
            nav_index: 0,
            state_save_task: None,
            request_save_tasks: HashMap::new(),
            env_save_task: None,
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
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("Workspace");
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

    pub fn open_import_modal(&mut self, target_folder_rel: Option<String>, cx: &mut Context<Self>) {
        self.show_import_modal = true;
        self.import_target_folder_rel = target_folder_rel;
        cx.notify();
    }

    pub fn close_import_modal(&mut self, cx: &mut Context<Self>) {
        self.show_import_modal = false;
        self.import_target_folder_rel = None;
        cx.notify();
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
        self.refresh_git_status(cx);
        self.close_import_modal(cx);
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
        self.close_import_modal(cx);
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
        self.refresh_git_status(cx);
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
        self.refresh_git_status(cx);
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
        self.refresh_git_status(cx);
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
                self.refresh_git_status(cx);
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

    pub fn start_renaming_example(
        &mut self,
        req_id: String,
        example_name: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.cancel_renaming(cx);
        self.cancel_renaming_folder(cx);

        let input = cx.new(|cx| {
            let mut s = InputState::new(window, cx);
            s.set_value(example_name.clone(), window, cx);
            s.select_all(window, cx);
            s.focus(window, cx);
            s
        });

        let sub = cx.subscribe(&input, move |this, input, event: &gpui_component::input::InputEvent, cx| {
            match event {
                gpui_component::input::InputEvent::PressEnter { .. } => {
                    let val = input.read(cx).value().to_string();
                    this.finish_renaming_example(val, cx);
                }
                gpui_component::input::InputEvent::Blur => {
                    let val = input.read(cx).value().to_string();
                    this.finish_renaming_example(val, cx);
                }
                _ => {}
            }
        });

        self.renaming_example = Some((req_id, example_name));
        self.rename_example_input = Some(input);
        self.rename_example_subscription = Some(sub);
        cx.notify();
    }

    pub fn rename_example_internal(&mut self, req_id: &str, old_name: &str, new_name: &str) -> bool {
        let trimmed = new_name.trim();
        if !trimmed.is_empty() && trimmed != old_name {
            if let Some(mut req_file) = self.get_request_file(req_id) {
                let mut unique_name = trimmed.to_string();
                let mut idx = 2;
                while req_file.examples.iter().any(|e| e.name == unique_name && e.name != old_name) {
                    unique_name = format!("{}-{}", trimmed, idx);
                    idx += 1;
                }
                if let Some(ex) = req_file.examples.iter_mut().find(|e| e.name == old_name) {
                    ex.name = unique_name.clone();
                }
                self.last_renamed_example = Some((req_id.to_string(), old_name.to_string(), unique_name));
                let _ = self.store.write_request(req_id, &req_file);
                self.reload_tree();
                return true;
            }
        }
        false
    }

    pub fn finish_renaming_example(&mut self, new_name: String, cx: &mut Context<Self>) {
        let Some((req_id, old_name)) = self.renaming_example.take() else {
            return;
        };
        self.rename_example_input = None;
        self.rename_example_subscription = None;
        if self.rename_example_internal(&req_id, &old_name, &new_name) {
            cx.notify();
        }
    }

    pub fn cancel_renaming_example(&mut self, cx: &mut Context<Self>) {
        if self.renaming_example.is_some() {
            self.renaming_example = None;
            self.rename_example_input = None;
            self.rename_example_subscription = None;
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
                    ws.refresh_git_status(cx);
                    cx.notify();
                });
            }
        });
        self.request_save_tasks.insert(req.id, task);
    }

    /// Updates the in-memory request and schedules persistence without notifying the
    /// root view. Use this for ordinary text edits where the input component already
    /// redraws itself and the request's surrounding layout has not changed.
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
                    ws.refresh_git_status(cx);
                    cx.notify();
                });
            }
        });
        self.request_save_tasks.insert(req_id, task);
    }

    /// Debounced auto-save for typing in active request editors.
    /// Does NOT clone the request or write to disk on every keystroke.
    /// Cancels previous timer and only saves 500ms after the user stops typing.
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
                    ws.refresh_git_status(cx);
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
        self.refresh_git_status(cx);
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
        self.refresh_git_status(cx);
        self.refresh_git_branches(cx);
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

    pub fn create_folder(&mut self, parent_id: Option<String>, name: String, cx: &mut Context<Self>) -> Option<String> {
        let trimmed = name.trim();
        if !trimmed.is_empty() {
            if let Some(ref p_id) = parent_id {
                self.collapsed_folders.remove(p_id);
            }
            if let Ok((_dir_name, rel_path)) = self.store.create_group(parent_id.as_deref(), trimmed) {
                self.reload_tree();
                self.refresh_git_status(cx);
                cx.notify();
                return Some(rel_path);
            }
            self.reload_tree();
            self.refresh_git_status(cx);
            cx.notify();
        }
        None
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
        self.refresh_git_status(cx);
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
        self.refresh_git_status(cx);
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
        } else if !self.show_env_modal {
            self.flush_environments();
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

    pub fn save_global_environment_silently(&mut self, env: Environment) {
        self.global_environment = env;
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

    pub fn start_renaming_workspace(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let curr_name = self
            .active_workspace
            .as_ref()
            .map(|w| w.name.clone())
            .unwrap_or_else(|| self.workspace_meta.name.clone());
        let input = cx.new(|cx| {
            let mut s = InputState::new(window, cx);
            s.set_value(curr_name, window, cx);
            s.select_all(window, cx);
            s.focus(window, cx);
            s
        });

        let sub = cx.subscribe(&input, move |this, input, event: &gpui_component::input::InputEvent, cx| {
            match event {
                gpui_component::input::InputEvent::PressEnter { .. } => {
                    let val = input.read(cx).value().to_string();
                    this.finish_renaming_workspace(val, cx);
                }
                _ => {}
            }
        });

        self.show_rename_workspace_modal = true;
        self.rename_workspace_input = Some(input);
        self.rename_workspace_subscription = Some(sub);
        cx.notify();
    }

    pub fn finish_renaming_workspace(&mut self, new_name: String, cx: &mut Context<Self>) {
        self.show_rename_workspace_modal = false;
        self.rename_workspace_input = None;
        self.rename_workspace_subscription = None;
        let trimmed = new_name.trim();
        if !trimmed.is_empty() {
            self.rename_workspace(trimmed.to_string(), cx);
        } else {
            cx.notify();
        }
    }

    pub fn cancel_renaming_workspace(&mut self, cx: &mut Context<Self>) {
        self.show_rename_workspace_modal = false;
        self.rename_workspace_input = None;
        self.rename_workspace_subscription = None;
        cx.notify();
    }

    pub fn rename_workspace_internal(&mut self, new_name: &str) -> bool {
        let trimmed = new_name.trim();
        if trimmed.is_empty() {
            return false;
        }
        self.workspace_meta.name = trimmed.to_string();
        let _ = self.store.save_workspace_meta(&self.workspace_meta);
        if let Some(ws) = &mut self.active_workspace {
            ws.name = trimmed.to_string();
        }
        true
    }

    pub fn rename_workspace(&mut self, new_name: String, cx: &mut Context<Self>) {
        if self.rename_workspace_internal(&new_name) {
            cx.notify();
        }
    }

    pub fn open_workspace_folder(&self) {
        let path = &self.workspace_path;
        #[cfg(target_os = "windows")]
        let _ = std::process::Command::new("explorer").arg(path).spawn();
        #[cfg(target_os = "macos")]
        let _ = std::process::Command::new("open").arg(path).spawn();
        #[cfg(target_os = "linux")]
        let _ = std::process::Command::new("xdg-open").arg(path).spawn();
    }

    pub fn clean_history_internal(&mut self) {
        self.recent_workspaces.clear();
        let mut settings = AppSettings::load();
        settings.recent_workspaces.clear();
        let _ = settings.save();
        self.nav_history.clear();
        self.nav_index = 0;
    }

    pub fn clean_history(&mut self, cx: &mut Context<Self>) {
        self.clean_history_internal();
        cx.notify();
    }

    pub fn clear_cookies(&mut self, cx: &mut Context<Self>) {
        let _ = self.store.save_cookies(&serde_json::json!({}));
        cx.notify();
    }

    pub fn toggle_command_palette(&mut self, cx: &mut Context<Self>) {
        self.show_command_palette = !self.show_command_palette;
        cx.notify();
    }

    pub fn dismiss_top_modal(&mut self, cx: &mut Context<Self>) -> bool {
        if self.show_command_palette {
            self.show_command_palette = false;
            cx.notify();
            return true;
        }
        if self.show_rename_workspace_modal {
            self.cancel_renaming_workspace(cx);
            return true;
        }
        if self.show_git_modal {
            self.close_git_modal(cx);
            return true;
        }
        if self.show_import_modal {
            self.show_import_modal = false;
            self.import_target_folder_rel = None;
            cx.notify();
            return true;
        }
        if self.show_cookies_modal {
            self.show_cookies_modal = false;
            cx.notify();
            return true;
        }
        if self.show_settings_modal {
            self.show_settings_modal = false;
            cx.notify();
            return true;
        }
        if self.show_env_modal {
            self.show_env_modal = false;
            cx.notify();
            return true;
        }
        if self.renaming_request_id.is_some() {
            self.cancel_renaming(cx);
            return true;
        }
        if self.renaming_folder_id.is_some() {
            self.cancel_renaming_folder(cx);
            return true;
        }
        if self.renaming_env_id.is_some() {
            self.cancel_renaming_environment(cx);
            return true;
        }
        if self.renaming_example.is_some() {
            self.cancel_renaming_example(cx);
            return true;
        }
        false
    }

    pub fn toggle_settings_modal(&mut self, cx: &mut Context<Self>) {
        self.show_settings_modal = !self.show_settings_modal;
        cx.notify();
    }

    pub fn open_workspace_settings(&mut self, cx: &mut Context<Self>) {
        self.scope_settings_target = Some(ScopeSettingsTarget::Workspace);
        cx.notify();
    }

    pub fn open_folder_settings(&mut self, folder_id: String, cx: &mut Context<Self>) {
        self.scope_settings_target = Some(ScopeSettingsTarget::Folder(folder_id));
        cx.notify();
    }

    pub fn close_scope_settings(&mut self, cx: &mut Context<Self>) {
        self.scope_settings_target = None;
        cx.notify();
    }

    pub fn save_workspace_scope_settings(
        &mut self,
        headers: Vec<KeyValuePair>,
        auth: Auth,
        cx: &mut Context<Self>,
    ) {
        self.workspace_meta.headers = headers.iter().cloned().map(Into::into).collect();
        self.workspace_meta.auth = auth.clone().into();
        if let Some(ws) = &mut self.active_workspace {
            ws.headers = headers;
            ws.auth = auth;
        }
        let _ = self.store.save_workspace_meta(&self.workspace_meta);
        cx.notify();
    }

    pub fn save_folder_scope_settings(
        &mut self,
        folder_id: &str,
        headers: Vec<KeyValuePair>,
        auth: Auth,
        cx: &mut Context<Self>,
    ) {
        if let Some(f) = self.folders.iter_mut().find(|f| f.id == folder_id) {
            f.headers = headers.clone();
            f.auth = auth.clone();
        }
        if let Ok(mut meta) = self.store.load_folder_meta(folder_id) {
            meta.headers = headers.into_iter().map(Into::into).collect();
            meta.auth = auth.into();
            let _ = self.store.save_folder_meta(folder_id, &meta);
        }
        cx.notify();
    }

    /// Traces folder hierarchy from root folder to immediate parent folder containing the request
    pub fn get_folder_hierarchy(&self, folder_id: Option<&str>) -> Vec<&Folder> {
        let mut chain = Vec::new();
        let mut cur_id = folder_id.map(|s| s.to_string());
        while let Some(id) = cur_id {
            if let Some(folder) = self.folders.iter().find(|f| f.id == id) {
                chain.push(folder);
                cur_id = folder.parent_id.clone();
            } else {
                break;
            }
        }
        chain.reverse();
        chain
    }

    /// Resolves effective headers and auth for a request, taking Workspace & Folder hierarchy into account
    pub fn resolve_request_hierarchy(&self, req: &HttpRequestItem) -> (Vec<KeyValuePair>, Auth) {
        let ws_headers = self.active_workspace.as_ref().map(|w| w.headers.as_slice()).unwrap_or(&[]);
        let ws_auth = self.active_workspace.as_ref().map(|w| &w.auth).unwrap_or(&Auth::None);

        let folder_chain_refs = self.get_folder_hierarchy(req.folder_id.as_deref());
        let folder_chain: Vec<(&[KeyValuePair], &Auth)> = folder_chain_refs
            .iter()
            .map(|f| (f.headers.as_slice(), &f.auth))
            .collect();

        restclient_core::http::resolve_hierarchical_headers_and_auth(
            ws_headers,
            ws_auth,
            &folder_chain,
            &req.headers,
            &req.auth,
        )
    }

    /// Gets preview description of inherited auth for a request or folder
    pub fn get_inherited_auth_description(&self, folder_id: Option<&str>) -> String {
        let folder_chain_refs = self.get_folder_hierarchy(folder_id);
        for f in folder_chain_refs.iter().rev() {
            if !f.auth.is_inherit() {
                let kind = match &f.auth {
                    Auth::None => "No Auth",
                    Auth::Bearer { .. } => "Bearer Token",
                    Auth::Basic { .. } => "Basic Auth",
                    Auth::ApiKey { .. } => "API Key",
                    Auth::Inherit => "Inherit",
                };
                return format!("Inherited from folder \"{}\" ({})", f.name, kind);
            }
        }
        let ws_auth = self.active_workspace.as_ref().map(|w| &w.auth).unwrap_or(&Auth::None);
        let ws_name = self.active_workspace.as_ref().map(|w| w.name.as_str()).unwrap_or("Workspace");
        match ws_auth {
            Auth::None => format!("Inherited from workspace \"{}\" (No Auth)", ws_name),
            Auth::Bearer { .. } => format!("Inherited from workspace \"{}\" (Bearer Token)", ws_name),
            Auth::Basic { .. } => format!("Inherited from workspace \"{}\" (Basic Auth)", ws_name),
            Auth::ApiKey { .. } => format!("Inherited from workspace \"{}\" (API Key)", ws_name),
            Auth::Inherit => "No auth configured in parent hierarchy".to_string(),
        }
    }

    pub fn set_theme_preference(&mut self, pref: crate::theme::ThemePreference, cx: &mut Context<Self>) {
        self.theme_preference = pref;
        let appearance = cx.window_appearance();
        let mode = crate::theme::resolve_theme_mode(pref, appearance);
        crate::theme::set_theme(pref, mode);

        let gpui_mode = match mode {
            crate::theme::ThemeMode::Dark => gpui_component::ThemeMode::Dark,
            crate::theme::ThemeMode::Light => gpui_component::ThemeMode::Light,
        };
        gpui_component::Theme::change(gpui_mode, None, cx);

        let mut settings = AppSettings::load();
        settings.theme = Some(pref.as_str().to_string());
        let _ = settings.save();

        cx.refresh_windows();
        cx.notify();
    }

    pub fn open_git_modal(&mut self, modal: GitActiveModal, cx: &mut Context<Self>) {
        self.active_git_modal = Some(modal);
        self.show_git_modal = true;
        self.git_error = None;
        match modal {
            GitActiveModal::Commit => {
                self.git_selected_unstaged_files.clear();
                self.git_selected_staged_files.clear();
                self.refresh_git_status(cx);
            }
            GitActiveModal::History => {
                self.git_expanded_history_files.clear();
                self.refresh_git_history(cx);
            }
            GitActiveModal::NewBranch => {
                self.refresh_git_branches(cx);
            }
            GitActiveModal::Remotes => {
                self.refresh_git_remotes(cx);
            }
        }
        cx.notify();
    }

    pub fn close_git_modal(&mut self, cx: &mut Context<Self>) {
        self.active_git_modal = None;
        self.show_git_modal = false;
        self.git_error = None;
        self.git_selected_unstaged_files.clear();
        self.git_selected_staged_files.clear();
        self.git_expanded_history_files.clear();
        cx.notify();
    }

    pub fn toggle_git_modal(&mut self, cx: &mut Context<Self>) {
        if self.active_git_modal.is_some() || self.show_git_modal {
            self.close_git_modal(cx);
        } else {
            self.open_git_modal(GitActiveModal::Commit, cx);
        }
    }

    pub fn set_git_tab(&mut self, tab: GitTab, cx: &mut Context<Self>) {
        self.git_tab = tab;
        match tab {
            GitTab::Changes => self.refresh_git_status(cx),
            GitTab::History => self.refresh_git_history(cx),
            GitTab::Branches => self.refresh_git_branches(cx),
        }
        cx.notify();
    }

    pub fn refresh_git_all(&mut self, cx: &mut Context<Self>) {
        self.refresh_git_status(cx);
        self.refresh_git_history(cx);
        self.refresh_git_branches(cx);
        self.refresh_git_remotes(cx);
    }

    pub fn refresh_git_remotes(&mut self, cx: &mut Context<Self>) {
        let ws_path = self.workspace_path.clone();
        cx.spawn(async move |this, cx| {
            let remotes = cx.background_executor().spawn(async move {
                restclient_core::git::git_get_remotes(&ws_path).ok()
            }).await;
            if let Some(remotes) = remotes {
                if let Some(this) = this.upgrade() {
                    this.update(cx, |ws, cx| {
                        ws.git_remotes = remotes;
                        cx.notify();
                    });
                }
            }
        }).detach();
    }

    pub fn git_add_remote(&mut self, name: &str, url: &str, cx: &mut Context<Self>) -> Result<(), String> {
        match restclient_core::git::git_add_remote(&self.workspace_path, name, url) {
            Ok(_) => {
                self.git_error = None;
                self.refresh_git_remotes(cx);
                Ok(())
            }
            Err(e) => {
                let err_str = e.to_string();
                self.git_error = Some(err_str.clone());
                cx.notify();
                Err(err_str)
            }
        }
    }

    pub fn git_remove_remote(&mut self, name: &str, cx: &mut Context<Self>) -> Result<(), String> {
        match restclient_core::git::git_remove_remote(&self.workspace_path, name) {
            Ok(_) => {
                self.git_error = None;
                self.refresh_git_remotes(cx);
                Ok(())
            }
            Err(e) => {
                let err_str = e.to_string();
                self.git_error = Some(err_str.clone());
                cx.notify();
                Err(err_str)
            }
        }
    }

    pub fn git_delete_branch(&mut self, name: &str, cx: &mut Context<Self>) -> Result<(), String> {
        match restclient_core::git::git_delete_branch(&self.workspace_path, name, false) {
            Ok(_) => {
                self.git_error = None;
                self.refresh_git_branches(cx);
                Ok(())
            }
            Err(e) => {
                let err_str = e.to_string();
                self.git_error = Some(err_str.clone());
                cx.notify();
                Err(err_str)
            }
        }
    }

    pub fn git_fetch(&mut self, cx: &mut Context<Self>) -> Result<String, String> {
        match restclient_core::git::git_fetch(&self.workspace_path) {
            Ok(msg) => {
                self.git_error = None;
                self.refresh_git_status(cx);
                Ok(msg)
            }
            Err(e) => {
                let err_str = e.to_string();
                self.git_error = Some(err_str.clone());
                cx.notify();
                Err(err_str)
            }
        }
    }

    pub fn discard_request_changes(
        &mut self,
        req_id: &str,
        editor: &Entity<EditorState>,
        cx: &mut Context<Self>,
    ) {
        let norm_req = req_id.replace('\\', "/");
        let coll_rel = format!("collections/{}", norm_req);
        let path_to_discard = if let Some(git) = &self.git_status {
            let matches = |p: &str| -> bool {
                let p = p.replace('\\', "/");
                p == norm_req
                    || p == coll_rel
                    || p.ends_with(&format!("/{}", norm_req))
                    || p.ends_with(&format!("/{}", coll_rel))
            };
            git.untracked_files.iter().find(|i| matches(&i.path))
                .or_else(|| git.unstaged_files.iter().find(|i| matches(&i.path)))
                .or_else(|| git.staged_files.iter().find(|i| matches(&i.path)))
                .map(|i| i.path.clone())
                .unwrap_or(coll_rel)
        } else {
            coll_rel
        };

        let _ = restclient_core::git::git_discard_file(&self.workspace_path, &path_to_discard);
        self.unsaved_requests.remove(req_id);
        self.reload_all();
        self.refresh_git_status(cx);

        let active_id = editor.read(cx).active_request.as_ref().map(|r| r.id.clone());
        if active_id.as_deref() == Some(req_id) {
            if let Some(reloaded_req) = self.find_request(req_id) {
                editor.update(cx, |ed, cx| {
                    ed.set_active_request(reloaded_req.clone(), false, cx);
                });
            } else {
                editor.update(cx, |ed, cx| {
                    ed.close_tab(req_id, cx);
                });
            }
        }
    }

    pub fn refresh_git_status(&mut self, cx: &mut Context<Self>) {
        let ws_path = self.workspace_path.clone();
        let show_modal = self.show_git_modal;
        let selected_file = self.git_selected_file.clone();

        cx.spawn(async move |this, cx| {
            let ws_status = ws_path.clone();
            let status_res = cx.background_executor().spawn(async move {
                restclient_core::git::git_status(&ws_status)
            }).await;

            let Ok(st) = status_res else { return; };

            let diff_result = if show_modal {
                let valid_staged: HashSet<String> = st.staged_files.iter().map(|f| f.path.clone()).collect();
                let valid_unstaged: HashSet<String> = st.unstaged_files.iter().chain(&st.untracked_files).map(|f| f.path.clone()).collect();

                let file_and_staged = if let Some((path, staged)) = selected_file {
                    if valid_staged.contains(&path) {
                        Some((path, true))
                    } else if valid_unstaged.contains(&path) {
                        Some((path, false))
                    } else {
                        Some((path, staged))
                    }
                } else if let Some(first) = st.staged_files.first() {
                    Some((first.path.clone(), true))
                } else if let Some(first) = st.unstaged_files.first() {
                    Some((first.path.clone(), false))
                } else if let Some(first) = st.untracked_files.first() {
                    Some((first.path.clone(), false))
                } else {
                    None
                };

                if let Some((path, staged)) = file_and_staged {
                    let ws_diff = ws_path.clone();
                    let p = path.clone();
                    let diff_content = cx.background_executor().spawn(async move {
                        match restclient_core::git::git_diff(&ws_diff, Some(&p), staged) {
                            Ok(d) => Some(d),
                            Err(e) => Some(format!("Error loading diff: {}", e)),
                        }
                    }).await;
                    Some((Some((path, staged)), diff_content))
                } else {
                    Some((None, None))
                }
            } else {
                None
            };

            let branches_res = if let Some(this_ent) = this.upgrade() {
                let need_branches = this_ent.read_with(cx, |this, _| this.git_branches.is_empty());
                if need_branches {
                    let ws_b = ws_path.clone();
                    cx.background_executor().spawn(async move {
                        restclient_core::git::git_branches(&ws_b).ok()
                    }).await
                } else {
                    None
                }
            } else {
                None
            };

            if let Some(this_ent) = this.upgrade() {
                this_ent.update(cx, |ws, cx| {
                    if st.is_repo && !st.branch.is_empty() {
                        ws.git_branch = Some(st.branch.clone());
                    }
                    let valid_unstaged: HashSet<String> = st.unstaged_files.iter().chain(&st.untracked_files).map(|f| f.path.clone()).collect();
                    ws.git_selected_unstaged_files.retain(|p| valid_unstaged.contains(p));

                    let valid_staged: HashSet<String> = st.staged_files.iter().map(|f| f.path.clone()).collect();
                    ws.git_selected_staged_files.retain(|p| valid_staged.contains(p));

                    if let Some((file_sel, diff_content)) = diff_result {
                        ws.git_selected_file = file_sel;
                        ws.git_diff_content = diff_content;
                    }

                    if let Some(branches) = branches_res {
                        ws.git_branches = branches;
                    }
                    ws.git_status = Some(st);
                    cx.notify();
                });
            }
        }).detach();
    }

    pub fn load_git_diff(&mut self, file_path: &str, staged: bool, cx: &mut Context<Self>) {
        let ws_path = self.workspace_path.clone();
        let path = file_path.to_string();
        cx.spawn(async move |this, cx| {
            let res = cx.background_executor().spawn(async move {
                restclient_core::git::git_diff(&ws_path, Some(&path), staged)
            }).await;
            if let Some(this) = this.upgrade() {
                this.update(cx, |ws, cx| {
                    match res {
                        Ok(d) => ws.git_diff_content = Some(d),
                        Err(e) => ws.git_diff_content = Some(format!("Error loading diff: {}", e)),
                    }
                    cx.notify();
                });
            }
        }).detach();
    }

    pub fn refresh_git_history(&mut self, cx: &mut Context<Self>) {
        let ws_path = self.workspace_path.clone();
        let needs_first = self.git_selected_commit.is_none();
        cx.spawn(async move |this, cx| {
            let ws_hist = ws_path.clone();
            let commits = cx.background_executor().spawn(async move {
                restclient_core::git::git_history(&ws_hist, 50).ok()
            }).await;
            if let Some(commits) = commits {
                let first_detail = if needs_first {
                    if let Some(first) = commits.first() {
                        let ws_p = ws_path.clone();
                        let h = first.hash.clone();
                        cx.background_executor().spawn(async move {
                            restclient_core::git::git_commit_detail(&ws_p, &h).ok()
                        }).await
                    } else {
                        None
                    }
                } else {
                    None
                };

                if let Some(this) = this.upgrade() {
                    this.update(cx, |ws, cx| {
                        if let Some(detail) = first_detail {
                            ws.git_selected_commit = Some(detail);
                        }
                        ws.git_history = commits;
                        cx.notify();
                    });
                }
            }
        }).detach();
    }

    pub fn refresh_git_branches(&mut self, cx: &mut Context<Self>) {
        let ws_path = self.workspace_path.clone();
        cx.spawn(async move |this, cx| {
            let branches = cx.background_executor().spawn(async move {
                restclient_core::git::git_branches(&ws_path).ok()
            }).await;
            if let Some(branches) = branches {
                if let Some(this) = this.upgrade() {
                    this.update(cx, |ws, cx| {
                        ws.git_branches = branches;
                        cx.notify();
                    });
                }
            }
        }).detach();
    }

    pub fn select_git_file(&mut self, path: String, staged: bool, cx: &mut Context<Self>) {
        self.git_selected_file = Some((path.clone(), staged));
        self.load_git_diff(&path, staged, cx);
    }

    pub fn select_git_commit(&mut self, hash: &str, cx: &mut Context<Self>) {
        self.git_expanded_history_files.clear();
        let ws_path = self.workspace_path.clone();
        let h = hash.to_string();
        cx.spawn(async move |this, cx| {
            let detail = cx.background_executor().spawn(async move {
                restclient_core::git::git_commit_detail(&ws_path, &h).ok()
            }).await;
            if let Some(detail) = detail {
                if let Some(this) = this.upgrade() {
                    this.update(cx, |ws, cx| {
                        ws.git_selected_commit = Some(detail);
                        cx.notify();
                    });
                }
            }
        }).detach();
    }

    pub fn toggle_git_history_file_expansion(&mut self, file_path: &str, cx: &mut Context<Self>) {
        if self.git_expanded_history_files.contains(file_path) {
            self.git_expanded_history_files.remove(file_path);
        } else {
            self.git_expanded_history_files.insert(file_path.to_string());
        }
        cx.notify();
    }

    pub fn git_stage_file(&mut self, path: &str, cx: &mut Context<Self>) {
        let ws_path = self.workspace_path.clone();
        let p = path.to_string();
        cx.spawn(async move |this, cx| {
            let _ = cx.background_executor().spawn(async move {
                restclient_core::git::git_stage_file(&ws_path, &p)
            }).await;
            if let Some(this) = this.upgrade() {
                this.update(cx, |ws, cx| {
                    ws.refresh_git_status(cx);
                });
            }
        }).detach();
    }

    pub fn git_unstage_file(&mut self, path: &str, cx: &mut Context<Self>) {
        let ws_path = self.workspace_path.clone();
        let p = path.to_string();
        cx.spawn(async move |this, cx| {
            let _ = cx.background_executor().spawn(async move {
                restclient_core::git::git_unstage_file(&ws_path, &p)
            }).await;
            if let Some(this) = this.upgrade() {
                this.update(cx, |ws, cx| {
                    ws.refresh_git_status(cx);
                });
            }
        }).detach();
    }

    pub fn git_stage_all(&mut self, cx: &mut Context<Self>) {
        let ws_path = self.workspace_path.clone();
        cx.spawn(async move |this, cx| {
            let _ = cx.background_executor().spawn(async move {
                restclient_core::git::git_stage_all(&ws_path)
            }).await;
            if let Some(this) = this.upgrade() {
                this.update(cx, |ws, cx| {
                    ws.refresh_git_status(cx);
                });
            }
        }).detach();
    }

    pub fn git_unstage_all(&mut self, cx: &mut Context<Self>) {
        let ws_path = self.workspace_path.clone();
        cx.spawn(async move |this, cx| {
            let _ = cx.background_executor().spawn(async move {
                restclient_core::git::git_unstage_all(&ws_path)
            }).await;
            if let Some(this) = this.upgrade() {
                this.update(cx, |ws, cx| {
                    ws.refresh_git_status(cx);
                });
            }
        }).detach();
    }

    pub fn toggle_git_unstaged_selection(&mut self, path: &str, cx: &mut Context<Self>) {
        if self.git_selected_unstaged_files.contains(path) {
            self.git_selected_unstaged_files.remove(path);
        } else {
            self.git_selected_unstaged_files.insert(path.to_string());
        }
        cx.notify();
    }

    pub fn toggle_all_unstaged_selection(&mut self, select: bool, cx: &mut Context<Self>) {
        if let Some(st) = &self.git_status {
            if select {
                self.git_selected_unstaged_files = st.unstaged_files.iter().chain(&st.untracked_files).map(|f| f.path.clone()).collect();
            } else {
                self.git_selected_unstaged_files.clear();
            }
        }
        cx.notify();
    }

    pub fn toggle_git_staged_selection(&mut self, path: &str, cx: &mut Context<Self>) {
        if self.git_selected_staged_files.contains(path) {
            self.git_selected_staged_files.remove(path);
        } else {
            self.git_selected_staged_files.insert(path.to_string());
        }
        cx.notify();
    }

    pub fn toggle_all_staged_selection(&mut self, select: bool, cx: &mut Context<Self>) {
        if let Some(st) = &self.git_status {
            if select {
                self.git_selected_staged_files = st.staged_files.iter().map(|f| f.path.clone()).collect();
            } else {
                self.git_selected_staged_files.clear();
            }
        }
        cx.notify();
    }

    pub fn git_stage_selected(&mut self, cx: &mut Context<Self>) {
        let paths: Vec<String> = self.git_selected_unstaged_files.iter().cloned().collect();
        if paths.is_empty() {
            return;
        }
        self.git_selected_unstaged_files.clear();
        let ws_path = self.workspace_path.clone();
        cx.spawn(async move |this, cx| {
            let _ = cx.background_executor().spawn(async move {
                let str_paths: Vec<&str> = paths.iter().map(|s| s.as_str()).collect();
                restclient_core::git::git_stage_files(&ws_path, &str_paths)
            }).await;
            if let Some(this) = this.upgrade() {
                this.update(cx, |ws, cx| {
                    ws.refresh_git_status(cx);
                });
            }
        }).detach();
    }

    pub fn git_unstage_selected(&mut self, cx: &mut Context<Self>) {
        let paths: Vec<String> = self.git_selected_staged_files.iter().cloned().collect();
        if paths.is_empty() {
            return;
        }
        self.git_selected_staged_files.clear();
        let ws_path = self.workspace_path.clone();
        cx.spawn(async move |this, cx| {
            let _ = cx.background_executor().spawn(async move {
                let str_paths: Vec<&str> = paths.iter().map(|s| s.as_str()).collect();
                restclient_core::git::git_unstage_files(&ws_path, &str_paths)
            }).await;
            if let Some(this) = this.upgrade() {
                this.update(cx, |ws, cx| {
                    ws.refresh_git_status(cx);
                });
            }
        }).detach();
    }

    pub fn git_discard_file(&mut self, path: &str, cx: &mut Context<Self>) {
        let _ = restclient_core::git::git_discard_file(&self.workspace_path, path);
        if let Some((sel, _)) = &self.git_selected_file {
            if sel == path {
                self.git_selected_file = None;
                self.git_diff_content = None;
            }
        }
        self.reload_all();
        self.refresh_git_status(cx);
    }

    pub fn git_commit(&mut self, message: &str, cx: &mut Context<Self>) -> Result<(), String> {
        match restclient_core::git::git_commit(&self.workspace_path, message) {
            Ok(_) => {
                self.git_commit_message.clear();
                self.git_error = None;
                self.git_selected_file = None;
                self.git_diff_content = None;
                self.refresh_git_all(cx);
                Ok(())
            }
            Err(e) => {
                let err_str = e.to_string();
                self.git_error = Some(err_str.clone());
                cx.notify();
                Err(err_str)
            }
        }
    }

    pub fn git_checkout(&mut self, branch: &str, cx: &mut Context<Self>) -> Result<(), String> {
        match restclient_core::git::git_checkout(&self.workspace_path, branch) {
            Ok(_) => {
                self.git_branch = Some(branch.to_string());
                self.git_error = None;
                self.reload_all();
                self.refresh_git_all(cx);
                Ok(())
            }
            Err(e) => {
                let err_str = e.to_string();
                self.git_error = Some(err_str.clone());
                cx.notify();
                Err(err_str)
            }
        }
    }

    pub fn git_create_branch(&mut self, name: &str, cx: &mut Context<Self>) -> Result<(), String> {
        match restclient_core::git::git_create_branch(&self.workspace_path, name) {
            Ok(_) => {
                self.git_branch = Some(name.to_string());
                self.git_error = None;
                self.refresh_git_all(cx);
                Ok(())
            }
            Err(e) => {
                let err_str = e.to_string();
                self.git_error = Some(err_str.clone());
                cx.notify();
                Err(err_str)
            }
        }
    }

    pub fn git_pull(&mut self, cx: &mut Context<Self>) -> Result<String, String> {
        self.git_is_syncing = true;
        cx.notify();
        let res = restclient_core::git::git_pull(&self.workspace_path);
        self.git_is_syncing = false;
        match res {
            Ok(msg) => {
                self.git_error = None;
                self.reload_all();
                self.refresh_git_all(cx);
                Ok(msg)
            }
            Err(e) => {
                let err_str = e.to_string();
                self.git_error = Some(err_str.clone());
                cx.notify();
                Err(err_str)
            }
        }
    }

    pub fn fetch_in_background(&mut self, cx: &mut Context<Self>) {
        let ws_path = self.workspace_path.clone();
        cx.spawn(async move |this, cx| {
            let res = cx.background_executor().spawn(async move {
                restclient_core::git::git_fetch(&ws_path)
            }).await;
            if res.is_ok() {
                if let Some(this) = this.upgrade() {
                    this.update(cx, |ws, cx| {
                        ws.refresh_git_status(cx);
                    });
                }
            }
        }).detach();
    }

    pub fn git_push(&mut self, cx: &mut Context<Self>) -> Result<String, String> {
        match restclient_core::git::git_push(&self.workspace_path) {
            Ok(msg) => {
                self.git_error = None;
                self.refresh_git_status(cx);
                Ok(msg)
            }
            Err(e) => {
                let err_str = e.to_string();
                self.git_error = Some(err_str.clone());
                cx.notify();
                Err(err_str)
            }
        }
    }

    pub fn git_init_repo(&mut self, cx: &mut Context<Self>) -> Result<(), String> {
        match restclient_core::git::git_init(&self.workspace_path) {
            Ok(_) => {
                self.git_error = None;
                self.refresh_git_all(cx);
                Ok(())
            }
            Err(e) => {
                let err_str = e.to_string();
                self.git_error = Some(err_str.clone());
                cx.notify();
                Err(err_str)
            }
        }
    }

    pub fn save_environments(&mut self, envs: Vec<Environment>, cx: &mut Context<Self>) {
        self.env_save_task = None;
        for env in &envs {
            let (file, secrets) = EnvironmentFile::from_environment(env);
            let _ = self.store.save_environment(&env.name, &file);
            let _ = self.store.save_secrets(&env.name, &secrets);
        }
        self.environments = envs;
        cx.notify();
    }

    pub fn save_environments_debounced(&mut self, envs: Vec<Environment>, cx: &mut Context<Self>) {
        self.environments = envs;

        let store = self.store.clone();
        let task = cx.spawn(async move |this, cx| {
            cx.background_executor().timer(std::time::Duration::from_millis(500)).await;

            let Some(this_ent) = this.upgrade() else { return; };
            let envs = this_ent.read_with(cx, |this, _| this.environments.clone());

            let store_clone = store.clone();
            cx.background_executor().spawn(async move {
                for env in &envs {
                    let (file, secrets) = EnvironmentFile::from_environment(env);
                    let _ = store_clone.save_environment(&env.name, &file);
                    let _ = store_clone.save_secrets(&env.name, &secrets);
                }
            }).await;

            if let Some(this_ent) = this.upgrade() {
                this_ent.update(cx, |ws, _cx| {
                    ws.env_save_task = None;
                });
            }
        });
        self.env_save_task = Some(task);
    }

    pub fn flush_environments(&mut self) {
        if self.env_save_task.take().is_some() {
            for env in &self.environments {
                let (file, secrets) = EnvironmentFile::from_environment(env);
                let _ = self.store.save_environment(&env.name, &file);
                let _ = self.store.save_secrets(&env.name, &secrets);
            }
        }
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
            key: "HOST".to_string(),
            initial_value: "global.example.com".to_string(),
            current_value: "".to_string(),
            secret: false,
        });
        global.variables.push(EnvironmentVariable {
            key: "GLOBAL_TOKEN".to_string(),
            initial_value: "token_123".to_string(),
            current_value: "".to_string(),
            secret: true,
        });
        global.variables.push(EnvironmentVariable {
            key: "ONLY_GLOBAL".to_string(),
            initial_value: "val_only_global".to_string(),
            current_value: "".to_string(),
            secret: false,
        });

        let mut active = Environment::new("ws1", "Production");
        active.variables.push(EnvironmentVariable {
            key: "HOST".to_string(),
            initial_value: "prod.example.com".to_string(),
            current_value: "".to_string(),
            secret: false,
        });
        active.variables.push(EnvironmentVariable {
            key: "PORT".to_string(),
            initial_value: "443".to_string(),
            current_value: "".to_string(),
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
            scope_settings_target: None,
            theme_preference: Default::default(),
            show_git_modal: false,
            show_rename_workspace_modal: false,
            show_import_modal: false,
            import_target_folder_rel: None,
            active_git_modal: None,
            git_tab: GitTab::Changes,
            git_status: None,
            git_history: Vec::new(),
            git_branches: Vec::new(),
            git_remotes: Vec::new(),
            git_selected_staged_files: HashSet::new(),
            git_selected_unstaged_files: HashSet::new(),
            git_selected_file: None,
            git_diff_content: None,
            git_selected_commit: None,
            git_expanded_history_files: HashSet::new(),
            git_commit_message: String::new(),
            git_error: None,
            git_loading: false,
            git_is_syncing: false,
            renaming_request_id: None,
            rename_input: None,
            rename_subscription: None,
            renaming_folder_id: None,
            rename_folder_input: None,
            rename_folder_subscription: None,
            renaming_env_id: None,
            rename_env_input: None,
            rename_env_subscription: None,
            renaming_example: None,
            rename_example_input: None,
            rename_example_subscription: None,
            rename_workspace_input: None,
            rename_workspace_subscription: None,
            last_renamed_example: None,
            collapsed_folders: HashSet::new(),
            collapsed_requests: HashSet::new(),
            unsaved_requests: HashSet::new(),
            running_request_id: None,
            recent_workspaces: Vec::new(),
            nav_history: Vec::new(),
            nav_index: 0,
            state_save_task: None,
            request_save_tasks: HashMap::new(),
            env_save_task: None,
            conflicts: HashMap::new(),
            corrupt_secrets_warning: None,
            last_renamed_request: None,
            watcher: None,
            watcher_task: None,
        };

        let effective = ws_state.effective_environment();
        let host_var = effective.variables.iter().find(|v| v.key == "HOST").unwrap();
        assert_eq!(host_var.resolved_value(), "prod.example.com");

        let port_var = effective.variables.iter().find(|v| v.key == "PORT").unwrap();
        assert_eq!(port_var.resolved_value(), "443");

        let token_var = effective.variables.iter().find(|v| v.key == "GLOBAL_TOKEN").unwrap();
        assert_eq!(token_var.resolved_value(), "token_123");

        assert!(effective.variables.iter().any(|v| v.key == "ONLY_GLOBAL"));
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
            scope_settings_target: None,
            theme_preference: Default::default(),
            show_git_modal: false,
            show_rename_workspace_modal: false,
            show_import_modal: false,
            import_target_folder_rel: None,
            active_git_modal: None,
            git_tab: GitTab::Changes,
            git_status: None,
            git_history: Vec::new(),
            git_branches: Vec::new(),
            git_remotes: Vec::new(),
            git_selected_staged_files: HashSet::new(),
            git_selected_unstaged_files: HashSet::new(),
            git_selected_file: None,
            git_diff_content: None,
            git_selected_commit: None,
            git_expanded_history_files: HashSet::new(),
            git_commit_message: String::new(),
            git_error: None,
            git_loading: false,
            git_is_syncing: false,
            renaming_request_id: None,
            rename_input: None,
            rename_subscription: None,
            renaming_folder_id: None,
            rename_folder_input: None,
            rename_folder_subscription: None,
            renaming_env_id: None,
            rename_env_input: None,
            rename_env_subscription: None,
            renaming_example: None,
            rename_example_input: None,
            rename_example_subscription: None,
            rename_workspace_input: None,
            rename_workspace_subscription: None,
            last_renamed_example: None,
            collapsed_folders: HashSet::new(),
            collapsed_requests: HashSet::new(),
            unsaved_requests: HashSet::new(),
            running_request_id: None,
            recent_workspaces: Vec::new(),
            nav_history: Vec::new(),
            nav_index: 0,
            state_save_task: None,
            request_save_tasks: HashMap::new(),
            env_save_task: None,
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

    #[test]
    fn test_rename_example() {
        let temp_dir = std::env::temp_dir().join(format!("test_ws_rename_ex_{}", uuid::Uuid::new_v4().simple()));
        let store = Arc::new(FsWorkspaceStore::new(&temp_dir));
        store.init_workspace("Rename Ex WS").unwrap();

        let (_slug, rel_path) = store.create_request(None, "Auth", HttpMethod::Post, "https://api.example.com/auth").unwrap();
        let mut req_file = store.read_request(&rel_path).unwrap();
        req_file.examples.push(restclient_core::storage::models::RequestExampleFile {
            name: "Success".to_string(),
            status: 200,
            headers: Vec::new(),
            body: Some("{}".to_string()),
            extra: std::collections::BTreeMap::new(),
        });
        req_file.examples.push(restclient_core::storage::models::RequestExampleFile {
            name: "Error".to_string(),
            status: 400,
            headers: Vec::new(),
            body: Some("{}".to_string()),
            extra: std::collections::BTreeMap::new(),
        });
        store.write_request(&rel_path, &req_file).unwrap();

        let mut ws_state = WorkspaceState {
            store: store.clone(),
            workspace_path: temp_dir.clone(),
            workspace_meta: WorkspaceMeta::default(),
            active_workspace: Some(Workspace::new("Rename Ex WS")),
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
            scope_settings_target: None,
            theme_preference: Default::default(),
            show_git_modal: false,
            show_rename_workspace_modal: false,
            show_import_modal: false,
            import_target_folder_rel: None,
            active_git_modal: None,
            git_tab: GitTab::Changes,
            git_status: None,
            git_history: Vec::new(),
            git_branches: Vec::new(),
            git_remotes: Vec::new(),
            git_selected_staged_files: HashSet::new(),
            git_selected_unstaged_files: HashSet::new(),
            git_selected_file: None,
            git_diff_content: None,
            git_selected_commit: None,
            git_expanded_history_files: HashSet::new(),
            git_commit_message: String::new(),
            git_error: None,
            git_loading: false,
            git_is_syncing: false,
            renaming_request_id: None,
            rename_input: None,
            rename_subscription: None,
            renaming_folder_id: None,
            rename_folder_input: None,
            rename_folder_subscription: None,
            renaming_env_id: None,
            rename_env_input: None,
            rename_env_subscription: None,
            renaming_example: None,
            rename_example_input: None,
            rename_example_subscription: None,
            rename_workspace_input: None,
            rename_workspace_subscription: None,
            last_renamed_example: None,
            collapsed_folders: HashSet::new(),
            collapsed_requests: HashSet::new(),
            unsaved_requests: HashSet::new(),
            running_request_id: None,
            recent_workspaces: Vec::new(),
            nav_history: Vec::new(),
            nav_index: 0,
            state_save_task: None,
            request_save_tasks: HashMap::new(),
            env_save_task: None,
            conflicts: HashMap::new(),
            corrupt_secrets_warning: None,
            last_renamed_request: None,
            watcher: None,
            watcher_task: None,
        };

        ws_state.reload_tree();
        assert_eq!(ws_state.requests.len(), 1);
        assert_eq!(ws_state.requests[0].examples.len(), 2);

        // 1. Rename "Success" -> "Auth OK"
        let renamed = ws_state.rename_example_internal(&rel_path, "Success", "Auth OK");
        assert!(renamed);
        assert_eq!(ws_state.requests[0].examples[0].name, "Auth OK");
        assert_eq!(ws_state.last_renamed_example, Some((rel_path.clone(), "Success".to_string(), "Auth OK".to_string())));

        // Verify persisted on disk
        let disk_req = store.read_request(&rel_path).unwrap();
        assert_eq!(disk_req.examples[0].name, "Auth OK");

        // 2. Collision test: Rename "Error" -> "Auth OK", should become "Auth OK-2"
        let renamed2 = ws_state.rename_example_internal(&rel_path, "Error", "Auth OK");
        assert!(renamed2);
        assert_eq!(ws_state.requests[0].examples[1].name, "Auth OK-2");
        assert_eq!(ws_state.last_renamed_example, Some((rel_path.clone(), "Error".to_string(), "Auth OK-2".to_string())));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_create_and_reload_subfolder() {
        let temp_dir = std::env::temp_dir().join(format!("restclient-test-subfolder-{}", uuid::Uuid::new_v4().simple()));
        let store = Arc::new(FsWorkspaceStore::new(&temp_dir));
        store.init_workspace("Test Workspace").unwrap();

        let (_parent_slug, parent_rel) = store.create_group(None, "Parent Folder").unwrap();
        let (_child_slug, child_rel) = store.create_group(Some(&parent_rel), "Child Folder").unwrap();

        let mut ws_state = WorkspaceState {
            store: store.clone(),
            workspace_path: temp_dir.clone(),
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
            scope_settings_target: None,
            theme_preference: Default::default(),
            show_git_modal: false,
            show_rename_workspace_modal: false,
            show_import_modal: false,
            import_target_folder_rel: None,
            active_git_modal: None,
            git_tab: GitTab::Changes,
            git_status: None,
            git_history: Vec::new(),
            git_branches: Vec::new(),
            git_remotes: Vec::new(),
            git_selected_staged_files: HashSet::new(),
            git_selected_unstaged_files: HashSet::new(),
            git_selected_file: None,
            git_diff_content: None,
            git_selected_commit: None,
            git_expanded_history_files: HashSet::new(),
            git_commit_message: String::new(),
            git_error: None,
            git_loading: false,
            git_is_syncing: false,
            renaming_request_id: None,
            rename_input: None,
            rename_subscription: None,
            renaming_folder_id: None,
            rename_folder_input: None,
            rename_folder_subscription: None,
            renaming_env_id: None,
            rename_env_input: None,
            rename_env_subscription: None,
            renaming_example: None,
            rename_example_input: None,
            rename_example_subscription: None,
            rename_workspace_input: None,
            rename_workspace_subscription: None,
            last_renamed_example: None,
            collapsed_folders: HashSet::new(),
            collapsed_requests: HashSet::new(),
            unsaved_requests: HashSet::new(),
            running_request_id: None,
            recent_workspaces: Vec::new(),
            nav_history: Vec::new(),
            nav_index: 0,
            state_save_task: None,
            request_save_tasks: HashMap::new(),
            env_save_task: None,
            conflicts: HashMap::new(),
            corrupt_secrets_warning: None,
            last_renamed_request: None,
            watcher: None,
            watcher_task: None,
        };

        ws_state.reload_tree();

        assert_eq!(ws_state.folders.len(), 2);
        let parent = ws_state.folders.iter().find(|f| f.id == parent_rel).expect("Parent folder should exist");
        assert_eq!(parent.name, "Parent Folder");
        assert!(parent.parent_id.is_none());

        let child = ws_state.folders.iter().find(|f| f.id == child_rel).expect("Child folder should exist");
        assert_eq!(child.name, "Child Folder");
        assert_eq!(child.parent_id.as_deref(), Some(parent_rel.as_str()));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_flush_environments() {
        let temp_dir = std::env::temp_dir().join(format!("restclient_test_flush_env_{}", uuid::Uuid::new_v4()));
        let store: Arc<dyn WorkspaceStore> = Arc::new(FsWorkspaceStore::new(&temp_dir));
        let _ = store.init_workspace("Test WS");

        let mut ws_state = WorkspaceState {
            store: store.clone(),
            workspace_path: temp_dir.clone(),
            workspace_meta: WorkspaceMeta::default(),
            active_workspace: Some(Workspace::new("Test WS")),
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
            scope_settings_target: None,
            theme_preference: Default::default(),
            show_git_modal: false,
            show_rename_workspace_modal: false,
            show_import_modal: false,
            import_target_folder_rel: None,
            active_git_modal: None,
            git_tab: GitTab::Changes,
            git_status: None,
            git_history: Vec::new(),
            git_branches: Vec::new(),
            git_remotes: Vec::new(),
            git_selected_staged_files: HashSet::new(),
            git_selected_unstaged_files: HashSet::new(),
            git_selected_file: None,
            git_diff_content: None,
            git_selected_commit: None,
            git_expanded_history_files: HashSet::new(),
            git_commit_message: String::new(),
            git_error: None,
            git_loading: false,
            git_is_syncing: false,
            renaming_request_id: None,
            rename_input: None,
            rename_subscription: None,
            renaming_folder_id: None,
            rename_folder_input: None,
            rename_folder_subscription: None,
            renaming_env_id: None,
            rename_env_input: None,
            rename_env_subscription: None,
            renaming_example: None,
            rename_example_input: None,
            rename_example_subscription: None,
            rename_workspace_input: None,
            rename_workspace_subscription: None,
            last_renamed_example: None,
            collapsed_folders: HashSet::new(),
            collapsed_requests: HashSet::new(),
            unsaved_requests: HashSet::new(),
            running_request_id: None,
            recent_workspaces: Vec::new(),
            nav_history: Vec::new(),
            nav_index: 0,
            state_save_task: None,
            request_save_tasks: HashMap::new(),
            env_save_task: None,
            conflicts: HashMap::new(),
            corrupt_secrets_warning: None,
            last_renamed_request: None,
            watcher: None,
            watcher_task: None,
        };

        let mut test_env = Environment::new("ws-test", "Staging");
        test_env.variables.push(EnvironmentVariable {
            key: "API_URL".to_string(),
            initial_value: "https://staging.api.com".to_string(),
            current_value: "http://localhost:3000".to_string(),
            secret: false,
        });

        ws_state.environments.push(test_env);
        // Direct save environments to disk
        for env in &ws_state.environments {
            let (file, secrets) = EnvironmentFile::from_environment(env);
            let _ = ws_state.store.save_environment(&env.name, &file);
            let _ = ws_state.store.save_secrets(&env.name, &secrets);
        }

        // Verify files exist in storage
        let loaded = store.list_environments().unwrap();
        assert!(loaded.contains(&"Staging".to_string()));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_rename_workspace() {
        let temp_dir = std::env::temp_dir().join(format!("test_ws_rename_{}", uuid::Uuid::new_v4().simple()));
        let store = Arc::new(FsWorkspaceStore::new(&temp_dir));
        store.init_workspace("Original Name").unwrap();

        let mut ws_state = WorkspaceState {
            store: store.clone(),
            workspace_path: temp_dir.clone(),
            workspace_meta: WorkspaceMeta {
                name: "Original Name".to_string(),
                ..Default::default()
            },
            active_workspace: Some(Workspace::new("Original Name")),
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
            scope_settings_target: None,
            theme_preference: Default::default(),
            show_git_modal: false,
            show_rename_workspace_modal: false,
            show_import_modal: false,
            import_target_folder_rel: None,
            active_git_modal: None,
            git_tab: GitTab::Changes,
            git_status: None,
            git_history: Vec::new(),
            git_branches: Vec::new(),
            git_remotes: Vec::new(),
            git_selected_staged_files: HashSet::new(),
            git_selected_unstaged_files: HashSet::new(),
            git_selected_file: None,
            git_diff_content: None,
            git_selected_commit: None,
            git_expanded_history_files: HashSet::new(),
            git_commit_message: String::new(),
            git_error: None,
            git_loading: false,
            git_is_syncing: false,
            renaming_request_id: None,
            rename_input: None,
            rename_subscription: None,
            renaming_folder_id: None,
            rename_folder_input: None,
            rename_folder_subscription: None,
            renaming_env_id: None,
            rename_env_input: None,
            rename_env_subscription: None,
            renaming_example: None,
            rename_example_input: None,
            rename_example_subscription: None,
            rename_workspace_input: None,
            rename_workspace_subscription: None,
            last_renamed_example: None,
            collapsed_folders: HashSet::new(),
            collapsed_requests: HashSet::new(),
            unsaved_requests: HashSet::new(),
            running_request_id: None,
            recent_workspaces: vec!["/path/one".to_string(), "/path/two".to_string()],
            nav_history: vec!["req-1".to_string()],
            nav_index: 0,
            state_save_task: None,
            request_save_tasks: HashMap::new(),
            env_save_task: None,
            conflicts: HashMap::new(),
            corrupt_secrets_warning: None,
            last_renamed_request: None,
            watcher: None,
            watcher_task: None,
        };

        // Rename workspace
        let success = ws_state.rename_workspace_internal("Production API");
        assert!(success);
        assert_eq!(ws_state.workspace_meta.name, "Production API");
        assert_eq!(ws_state.active_workspace.as_ref().unwrap().name, "Production API");

        // Verify persisted to storage
        let meta = store.load_workspace_meta().unwrap();
        assert_eq!(meta.name, "Production API");

        // Blank name should fail
        assert!(!ws_state.rename_workspace_internal("   "));
        assert_eq!(ws_state.workspace_meta.name, "Production API");

        // Clean history
        ws_state.clean_history_internal();
        assert!(ws_state.recent_workspaces.is_empty());
        assert!(ws_state.nav_history.is_empty());
        assert_eq!(ws_state.nav_index, 0);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_workspace_and_folder_scope_hierarchy() {
        let temp_dir = std::env::temp_dir().join(format!("test_ws_scope_{}", uuid::Uuid::new_v4().simple()));
        let store = Arc::new(FsWorkspaceStore::new(&temp_dir));
        let _ = store.init_workspace("Hierarchy Test WS");

        let mut ws = Workspace::new("Hierarchy Test WS");
        ws.headers = vec![KeyValuePair::new("X-Workspace", "ws1")];
        ws.auth = Auth::Bearer { token: "ws-token".to_string() };

        let mut folder_users = Folder::new("ws", "Users");
        folder_users.id = "users".to_string();
        folder_users.headers = vec![KeyValuePair::new("X-Folder", "users")];
        folder_users.auth = Auth::Inherit;

        let mut folder_admin = Folder::new("ws", "Admin");
        folder_admin.id = "users/admin".to_string();
        folder_admin.parent_id = Some("users".to_string());
        folder_admin.headers = vec![KeyValuePair::new("X-Admin", "true")];
        folder_admin.auth = Auth::Basic { username: "admin".to_string(), password: "secret".to_string() };

        let ws_state = WorkspaceState {
            store,
            workspace_path: temp_dir.clone(),
            workspace_meta: WorkspaceMeta::default(),
            active_workspace: Some(ws),
            folders: vec![folder_users, folder_admin],
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
            scope_settings_target: None,
            theme_preference: Default::default(),
            show_git_modal: false,
            show_rename_workspace_modal: false,
            show_import_modal: false,
            import_target_folder_rel: None,
            active_git_modal: None,
            git_tab: GitTab::Changes,
            git_status: None,
            git_history: Vec::new(),
            git_branches: Vec::new(),
            git_remotes: Vec::new(),
            git_selected_staged_files: HashSet::new(),
            git_selected_unstaged_files: HashSet::new(),
            git_selected_file: None,
            git_diff_content: None,
            git_selected_commit: None,
            git_expanded_history_files: HashSet::new(),
            git_commit_message: String::new(),
            git_error: None,
            git_loading: false,
            git_is_syncing: false,
            renaming_request_id: None,
            rename_input: None,
            rename_subscription: None,
            renaming_folder_id: None,
            rename_folder_input: None,
            rename_folder_subscription: None,
            renaming_env_id: None,
            rename_env_input: None,
            rename_env_subscription: None,
            renaming_example: None,
            rename_example_input: None,
            rename_example_subscription: None,
            rename_workspace_input: None,
            rename_workspace_subscription: None,
            last_renamed_example: None,
            collapsed_folders: HashSet::new(),
            collapsed_requests: HashSet::new(),
            unsaved_requests: HashSet::new(),
            running_request_id: None,
            recent_workspaces: Vec::new(),
            nav_history: Vec::new(),
            nav_index: 0,
            state_save_task: None,
            request_save_tasks: HashMap::new(),
            env_save_task: None,
            conflicts: HashMap::new(),
            corrupt_secrets_warning: None,
            last_renamed_request: None,
            watcher: None,
            watcher_task: None,
        };

        // Case 1: Request in subfolder (users/admin) with Inherit auth
        let mut req1 = HttpRequestItem::new("ws", "Admin Req", HttpMethod::Get, "https://api.com/admin");
        req1.folder_id = Some("users/admin".to_string());
        req1.headers = vec![KeyValuePair::new("X-Req", "1")];
        req1.auth = Auth::Inherit;

        let (headers1, auth1) = ws_state.resolve_request_hierarchy(&req1);
        assert_eq!(headers1.len(), 4); // X-Workspace, X-Folder, X-Admin, X-Req
        assert_eq!(auth1, Auth::Basic { username: "admin".to_string(), password: "secret".to_string() });

        // Case 2: Request in top folder (users) with Inherit auth
        let mut req2 = HttpRequestItem::new("ws", "Users Req", HttpMethod::Get, "https://api.com/users");
        req2.folder_id = Some("users".to_string());
        req2.auth = Auth::Inherit;

        let (headers2, auth2) = ws_state.resolve_request_hierarchy(&req2);
        assert_eq!(headers2.len(), 2); // X-Workspace, X-Folder
        assert_eq!(auth2, Auth::Bearer { token: "ws-token".to_string() }); // Inherited from Workspace!

        // Case 3: Root request (no folder) with Inherit auth
        let mut req3 = HttpRequestItem::new("ws", "Root Req", HttpMethod::Get, "https://api.com/root");
        req3.auth = Auth::Inherit;

        let (headers3, auth3) = ws_state.resolve_request_hierarchy(&req3);
        assert_eq!(headers3.len(), 1); // X-Workspace
        assert_eq!(auth3, Auth::Bearer { token: "ws-token".to_string() });

        // Case 4: Request in subfolder with explicit None auth
        let mut req4 = HttpRequestItem::new("ws", "Public Req", HttpMethod::Get, "https://api.com/public");
        req4.folder_id = Some("users/admin".to_string());
        req4.auth = Auth::None;

        let (_, auth4) = ws_state.resolve_request_hierarchy(&req4);
        assert_eq!(auth4, Auth::None);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}


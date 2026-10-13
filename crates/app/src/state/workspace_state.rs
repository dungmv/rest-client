use crate::platform::detect_git_branch;
use gpui::{AppContext as _, Context, Entity, Window};
use gpui_component::input::InputState;
use restclient_core::models::{Environment, Folder, HttpMethod, HttpRequestItem, Workspace};
use restclient_core::storage::{FsStorage, HistoryStorage};
use std::sync::Arc;

pub struct WorkspaceState {
    pub fs_storage: Arc<FsStorage>,
    pub history_storage: Arc<HistoryStorage>,
    pub workspaces: Vec<Workspace>,
    pub active_workspace: Option<Workspace>,
    pub folders: Vec<Folder>,
    pub requests: Vec<HttpRequestItem>,
    pub environments: Vec<Environment>,
    pub active_environment_id: Option<String>,
    pub git_branch: Option<String>,
    pub filter_query: String,
    pub sidebar_collapsed: bool,
    pub split_layout_vertical: bool,
    pub show_env_modal: bool,
    pub show_cookies_modal: bool,
    pub show_command_palette: bool,
    pub show_settings_modal: bool,
    pub renaming_request_id: Option<String>,
    pub rename_input: Option<Entity<InputState>>,
    pub rename_subscription: Option<gpui::Subscription>,
}

impl WorkspaceState {
    pub fn new() -> Self {
        let storage_dir = FsStorage::default_storage_dir();
        let fs_storage = Arc::new(FsStorage::new(&storage_dir));
        let history_path = storage_dir.join("history.db");
        let history_storage = Arc::new(
            HistoryStorage::new(&history_path)
                .unwrap_or_else(|_| HistoryStorage::new_in_memory().unwrap()),
        );

        let git_branch = detect_git_branch(None);

        let mut state = Self {
            fs_storage,
            history_storage,
            workspaces: Vec::new(),
            active_workspace: None,
            folders: Vec::new(),
            requests: Vec::new(),
            environments: Vec::new(),
            active_environment_id: None,
            git_branch,
            filter_query: String::new(),
            sidebar_collapsed: false,
            split_layout_vertical: false,
            show_env_modal: false,
            show_cookies_modal: false,
            show_command_palette: false,
            show_settings_modal: false,
            renaming_request_id: None,
            rename_input: None,
            rename_subscription: None,
        };

        state.load_initial();
        state
    }

    fn load_initial(&mut self) {
        let mut ws_list = self.fs_storage.list_workspaces().unwrap_or_default();
        if ws_list.is_empty() {
            let default_ws = Workspace::new("My Workspace");
            let _ = self.fs_storage.init_workspace(&default_ws);
            ws_list = vec![default_ws];
        }

        self.workspaces = ws_list.clone();
        if let Some(first_ws) = ws_list.first() {
            self.switch_workspace(&first_ws.id);
        }
    }

    pub fn switch_workspace(&mut self, ws_id: &str) {
        if let Ok(ws) = self.fs_storage.load_workspace(ws_id) {
            self.folders = self.fs_storage.load_folders(ws_id).unwrap_or_default();
            self.requests = self.fs_storage.list_requests(ws_id).unwrap_or_default();
            self.environments = self.fs_storage.load_environments(ws_id).unwrap_or_default();
            self.active_environment_id = ws.active_environment_id.clone().or_else(|| {
                self.environments.first().map(|e| e.id.clone())
            });
            self.active_workspace = Some(ws);
        }
    }

    pub fn active_environment(&self) -> Option<&Environment> {
        if let Some(env_id) = &self.active_environment_id {
            self.environments.iter().find(|e| &e.id == env_id)
        } else {
            None
        }
    }

    pub fn set_active_environment(&mut self, env_id: Option<String>, cx: &mut Context<Self>) {
        self.active_environment_id = env_id.clone();
        if let Some(ws) = &mut self.active_workspace {
            ws.active_environment_id = env_id;
            let _ = self.fs_storage.save_workspace(ws);
        }
        cx.notify();
    }

    pub fn create_request(&mut self, folder_id: Option<String>, cx: &mut Context<Self>) -> HttpRequestItem {
        let ws_id = self.active_workspace.as_ref().map(|w| w.id.as_str()).unwrap_or("default");
        let mut req = HttpRequestItem::new(ws_id, "New Request", HttpMethod::Get, "");
        req.folder_id = folder_id;
        let _ = self.fs_storage.save_request(&req);
        self.requests.push(req.clone());
        cx.notify();
        req
    }

    pub fn duplicate_request(&mut self, req_id: &str, cx: &mut Context<Self>) -> Option<HttpRequestItem> {
        let existing = self.requests.iter().find(|r| r.id == req_id)?.clone();
        let ws_id = self.active_workspace.as_ref().map(|w| w.id.as_str()).unwrap_or("default");
        let mut dup = existing;
        dup.id = uuid::Uuid::new_v4().to_string();
        dup.workspace_id = ws_id.to_string();
        dup.name = format!("{} (Copy)", dup.name);
        dup.last_status_code = None;
        let _ = self.fs_storage.save_request(&dup);
        self.requests.push(dup.clone());
        cx.notify();
        Some(dup)
    }

    pub fn delete_request(&mut self, req_id: &str, cx: &mut Context<Self>) {
        if self.renaming_request_id.as_deref() == Some(req_id) {
            self.renaming_request_id = None;
            self.rename_input = None;
            self.rename_subscription = None;
        }
        let ws_id = self.active_workspace.as_ref().map(|w| w.id.as_str()).unwrap_or("default");
        let _ = self.fs_storage.delete_request(ws_id, req_id);
        self.requests.retain(|r| r.id != req_id);
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
            if let Some(req) = self.requests.iter_mut().find(|r| r.id == req_id) {
                req.name = trimmed.to_string();
                let _ = self.fs_storage.save_request(req);
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
        if req.workspace_id.is_empty() || req.workspace_id == "scratch" {
            if let Some(ws) = &self.active_workspace {
                req.workspace_id = ws.id.clone();
            }
        }
        let _ = self.fs_storage.save_request(&req);
        if let Some(idx) = self.requests.iter().position(|r| r.id == req.id) {
            self.requests[idx] = req;
        } else {
            self.requests.push(req);
        }
        cx.notify();
    }

    pub fn create_folder(&mut self, name: String, cx: &mut Context<Self>) {
        let ws_id = self.active_workspace.as_ref().map(|w| w.id.as_str()).unwrap_or("default");
        let folder = Folder::new(ws_id, name);
        self.folders.push(folder);
        let _ = self.fs_storage.save_folders(ws_id, &self.folders);
        cx.notify();
    }

    pub fn delete_folder(&mut self, folder_id: &str, cx: &mut Context<Self>) {
        let ws_id = self.active_workspace.as_ref().map(|w| w.id.as_str()).unwrap_or("default");
        self.folders.retain(|f| f.id != folder_id);
        for req in &mut self.requests {
            if req.folder_id.as_deref() == Some(folder_id) {
                req.folder_id = None;
                let _ = self.fs_storage.save_request(req);
            }
        }
        let _ = self.fs_storage.save_folders(ws_id, &self.folders);
        cx.notify();
    }

    pub fn set_filter_query(&mut self, query: String, cx: &mut Context<Self>) {
        self.filter_query = query;
        cx.notify();
    }

    pub fn toggle_sidebar(&mut self, cx: &mut Context<Self>) {
        self.sidebar_collapsed = !self.sidebar_collapsed;
        cx.notify();
    }

    pub fn toggle_split_layout(&mut self, cx: &mut Context<Self>) {
        self.split_layout_vertical = !self.split_layout_vertical;
        cx.notify();
    }

    pub fn toggle_env_modal(&mut self, cx: &mut Context<Self>) {
        self.show_env_modal = !self.show_env_modal;
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
        let ws_id = self.active_workspace.as_ref().map(|w| w.id.as_str()).unwrap_or("default");
        self.environments = envs.clone();
        let _ = self.fs_storage.save_environments(ws_id, &envs);
        cx.notify();
    }
}

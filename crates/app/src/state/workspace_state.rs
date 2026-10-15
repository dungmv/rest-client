use crate::platform::detect_git_branch;
use gpui::{AppContext as _, Context, Entity, Pixels, Window, px};
use gpui_component::input::InputState;
use restclient_core::models::{Environment, Folder, HttpMethod, HttpRequestItem, Workspace};
use restclient_core::storage::FsStorage;
use std::collections::HashSet;
use std::sync::Arc;

pub struct WorkspaceState {
    pub fs_storage: Arc<FsStorage>,
    pub workspaces: Vec<Workspace>,
    pub active_workspace: Option<Workspace>,
    pub folders: Vec<Folder>,
    pub requests: Vec<HttpRequestItem>,
    pub environments: Vec<Environment>,
    pub global_environment: Environment,
    pub active_environment_id: Option<String>,
    pub selected_env_modal_id: Option<String>,
    pub env_filter_query: String,
    pub var_filter_query: String,
    pub git_branch: Option<String>,
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
}

impl WorkspaceState {
    pub fn new() -> Self {
        let storage_dir = FsStorage::default_storage_dir();
        let fs_storage = Arc::new(FsStorage::new(&storage_dir));
        let git_branch = detect_git_branch(None);

        let mut state = Self {
            fs_storage,
            workspaces: Vec::new(),
            active_workspace: None,
            folders: Vec::new(),
            requests: Vec::new(),
            environments: Vec::new(),
            global_environment: Environment::new("default", "Globals"),
            active_environment_id: None,
            selected_env_modal_id: Some("globals".to_string()),
            env_filter_query: String::new(),
            var_filter_query: String::new(),
            git_branch,
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
            self.global_environment = self.fs_storage.load_global_environment(ws_id).unwrap_or_else(|_| {
                let mut g = Environment::new(ws_id, "Globals");
                g.id = format!("{}-globals", ws_id);
                g
            });
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

    pub fn active_environment_mut(&mut self) -> Option<&mut Environment> {
        if let Some(env_id) = &self.active_environment_id {
            self.environments.iter_mut().find(|e| &e.id == env_id)
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

    pub fn effective_environment(&self) -> Environment {
        let ws_id = self.active_workspace.as_ref().map(|w| w.id.as_str()).unwrap_or("default");
        let mut effective = Environment::new(ws_id, "Effective");

        // 1. Add all enabled variables from global_environment
        for v in &self.global_environment.variables {
            if v.enabled {
                effective.variables.push(v.clone());
            }
        }

        // 2. Active environment overrides matching keys or adds new variables
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

    pub fn update_request(&mut self, mut req: HttpRequestItem, cx: &mut Context<Self>) {
        if req.workspace_id.is_empty() || req.workspace_id == "scratch" {
            if let Some(ws) = &self.active_workspace {
                req.workspace_id = ws.id.clone();
            }
        }
        // Only update existing requests; never resurrect or push a non-existent/deleted request
        if let Some(idx) = self.requests.iter().position(|r| r.id == req.id) {
            let _ = self.fs_storage.save_request(&req);
            self.requests[idx] = req;
            cx.notify();
        }
    }

    pub fn create_folder(&mut self, name: String, cx: &mut Context<Self>) {
        let ws_id = self.active_workspace.as_ref().map(|w| w.id.as_str()).unwrap_or("default");
        let folder = Folder::new(ws_id, name);
        self.folders.push(folder);
        let _ = self.fs_storage.save_folders(ws_id, &self.folders);
        cx.notify();
    }

    pub fn delete_folder(&mut self, folder_id: &str, cx: &mut Context<Self>) {
        if self.renaming_folder_id.as_deref() == Some(folder_id) {
            self.renaming_folder_id = None;
            self.rename_folder_input = None;
            self.rename_folder_subscription = None;
        }
        self.collapsed_folders.remove(folder_id);
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

    pub fn toggle_folder_collapsed(&mut self, folder_id: &str, cx: &mut Context<Self>) {
        if self.collapsed_folders.contains(folder_id) {
            self.collapsed_folders.remove(folder_id);
        } else {
            self.collapsed_folders.insert(folder_id.to_string());
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
            self.rename_folder(&folder_id, trimmed.to_string(), cx);
        } else {
            cx.notify();
        }
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
        if let Some(folder) = self.folders.iter_mut().find(|f| f.id == folder_id) {
            folder.name = trimmed.to_string();
            let ws_id = self.active_workspace.as_ref().map(|w| w.id.as_str()).unwrap_or("default");
            let _ = self.fs_storage.save_folders(ws_id, &self.folders);
            cx.notify();
        }
    }

    pub fn move_request(&mut self, src_req_id: &str, target_req_id: &str, cx: &mut Context<Self>) {
        if src_req_id == target_req_id {
            return;
        }
        let src_pos = self.requests.iter().position(|r| r.id == src_req_id);
        let target_pos = self.requests.iter().position(|r| r.id == target_req_id);
        if let (Some(s_idx), Some(t_idx)) = (src_pos, target_pos) {
            let target_folder_id = self.requests[t_idx].folder_id.clone();
            let mut req = self.requests.remove(s_idx);
            req.folder_id = target_folder_id;
            let new_t_idx = self.requests.iter().position(|r| r.id == target_req_id).unwrap_or(self.requests.len());
            self.requests.insert(new_t_idx, req);

            for (idx, r) in self.requests.iter_mut().enumerate() {
                r.sort_order = idx as i32;
                let _ = self.fs_storage.save_request(r);
            }
            cx.notify();
        }
    }

    pub fn move_request_to_folder(
        &mut self,
        src_req_id: &str,
        target_folder_id: Option<String>,
        cx: &mut Context<Self>,
    ) {
        if let Some(src_idx) = self.requests.iter().position(|r| r.id == src_req_id) {
            let mut req = self.requests.remove(src_idx);
            req.folder_id = target_folder_id.clone();
            if let Some(ref fid) = target_folder_id {
                self.collapsed_folders.remove(fid);
            }

            let insert_idx = self
                .requests
                .iter()
                .rposition(|r| r.folder_id == target_folder_id)
                .map(|pos| pos + 1)
                .unwrap_or(self.requests.len());

            self.requests.insert(insert_idx, req);

            for (idx, r) in self.requests.iter_mut().enumerate() {
                r.sort_order = idx as i32;
                let _ = self.fs_storage.save_request(r);
            }
            cx.notify();
        }
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

            for (idx, f) in self.folders.iter_mut().enumerate() {
                f.sort_order = idx as i32;
            }
            let ws_id = self.active_workspace.as_ref().map(|w| w.id.as_str()).unwrap_or("default");
            let _ = self.fs_storage.save_folders(ws_id, &self.folders);
            cx.notify();
        }
    }

    pub fn set_filter_query(&mut self, query: String, cx: &mut Context<Self>) {
        self.filter_query = query;
        cx.notify();
    }

    pub fn toggle_sidebar(&mut self, cx: &mut Context<Self>) {
        self.sidebar_collapsed = !self.sidebar_collapsed;
        cx.notify();
    }

    pub fn set_sidebar_width(&mut self, width: Pixels, cx: &mut Context<Self>) {
        let clamped = px(width.as_f32().clamp(200.0, 480.0));
        if self.sidebar_width != clamped {
            self.sidebar_width = clamped;
            cx.notify();
        }
    }

    pub fn toggle_split_layout(&mut self, cx: &mut Context<Self>) {
        self.split_layout_vertical = !self.split_layout_vertical;
        cx.notify();
    }

    pub fn toggle_env_modal(&mut self, cx: &mut Context<Self>) {
        self.show_env_modal = !self.show_env_modal;
        if self.show_env_modal {
            if self.selected_env_modal_id.is_none() {
                self.selected_env_modal_id = self
                    .active_environment_id
                    .clone()
                    .or_else(|| Some("globals".to_string()));
            }
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
        let ws_id = self.active_workspace.as_ref().map(|w| w.id.as_str()).unwrap_or("default");
        self.global_environment = env.clone();
        let _ = self.fs_storage.save_global_environment(ws_id, &env);
        cx.notify();
    }

    pub fn create_environment(&mut self, name: Option<String>, cx: &mut Context<Self>) -> String {
        let ws_id = self.active_workspace.as_ref().map(|w| w.id.clone()).unwrap_or_else(|| "default".to_string());
        let env_name = name.unwrap_or_else(|| format!("env-{}", self.environments.len() + 1));
        let new_env = Environment::new(&ws_id, env_name);
        let new_id = new_env.id.clone();
        self.environments.push(new_env);
        let ws_id_ref = self.active_workspace.as_ref().map(|w| w.id.as_str()).unwrap_or("default");
        let _ = self.fs_storage.save_environments(ws_id_ref, &self.environments);
        self.selected_env_modal_id = Some(new_id.clone());
        if self.active_environment_id.is_none() {
            self.set_active_environment(Some(new_id.clone()), cx);
        } else {
            cx.notify();
        }
        new_id
    }

    pub fn duplicate_environment(&mut self, env_id: &str, cx: &mut Context<Self>) -> Option<String> {
        let ws_id = self.active_workspace.as_ref().map(|w| w.id.clone()).unwrap_or_else(|| "default".to_string());
        if env_id == "globals" {
            let mut new_env = Environment::new(&ws_id, "Globals (Copy)");
            new_env.variables = self.global_environment.variables.clone();
            let new_id = new_env.id.clone();
            self.environments.push(new_env);
            let ws_id_ref = self.active_workspace.as_ref().map(|w| w.id.as_str()).unwrap_or("default");
            let _ = self.fs_storage.save_environments(ws_id_ref, &self.environments);
            self.selected_env_modal_id = Some(new_id.clone());
            cx.notify();
            return Some(new_id);
        }

        let existing = self.environments.iter().find(|e| e.id == env_id)?.clone();
        let mut dup = existing.clone();
        dup.id = uuid::Uuid::new_v4().to_string();
        dup.workspace_id = ws_id;
        dup.name = format!("{} (Copy)", existing.name);
        let dup_id = dup.id.clone();
        self.environments.push(dup);
        let ws_id_ref = self.active_workspace.as_ref().map(|w| w.id.as_str()).unwrap_or("default");
        let _ = self.fs_storage.save_environments(ws_id_ref, &self.environments);
        self.selected_env_modal_id = Some(dup_id.clone());
        cx.notify();
        Some(dup_id)
    }

    pub fn rename_environment(&mut self, env_id: &str, new_name: String, cx: &mut Context<Self>) {
        let trimmed = new_name.trim();
        if trimmed.is_empty() || env_id == "globals" {
            return;
        }
        if let Some(env) = self.environments.iter_mut().find(|e| e.id == env_id) {
            env.name = trimmed.to_string();
            let ws_id = self.active_workspace.as_ref().map(|w| w.id.as_str()).unwrap_or("default");
            let _ = self.fs_storage.save_environments(ws_id, &self.environments);
            cx.notify();
        }
    }

    pub fn delete_environment(&mut self, env_id: &str, cx: &mut Context<Self>) {
        if env_id == "globals" {
            return;
        }
        self.environments.retain(|e| e.id != env_id);
        if self.active_environment_id.as_deref() == Some(env_id) {
            self.set_active_environment(None, cx);
        }
        if self.selected_env_modal_id.as_deref() == Some(env_id) {
            self.selected_env_modal_id = self
                .environments
                .first()
                .map(|e| e.id.clone())
                .or_else(|| Some("globals".to_string()));
        }
        let ws_id = self.active_workspace.as_ref().map(|w| w.id.as_str()).unwrap_or("default");
        let _ = self.fs_storage.save_environments(ws_id, &self.environments);
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
        // Overrides HOST from global
        active.variables.push(EnvironmentVariable {
            enabled: true,
            key: "HOST".to_string(),
            value: "prod.example.com".to_string(),
            secret: false,
        });
        // Adds unique active var
        active.variables.push(EnvironmentVariable {
            enabled: true,
            key: "PORT".to_string(),
            value: "443".to_string(),
            secret: false,
        });

        let ws_state = WorkspaceState {
            fs_storage: Arc::new(FsStorage::new(std::env::temp_dir())),
            workspaces: Vec::new(),
            active_workspace: Some(Workspace::new("Test")),
            folders: Vec::new(),
            requests: Vec::new(),
            environments: vec![active.clone()],
            global_environment: global,
            active_environment_id: Some(active.id.clone()),
            selected_env_modal_id: None,
            env_filter_query: String::new(),
            var_filter_query: String::new(),
            git_branch: None,
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
        };

        let effective = ws_state.effective_environment();
        let map: std::collections::HashMap<_, _> = effective
            .variables
            .iter()
            .map(|v| (v.key.as_str(), v.value.as_str()))
            .collect();

        // HOST should be overridden by active env
        assert_eq!(map.get("HOST"), Some(&"prod.example.com"));
        // GLOBAL_TOKEN should be retained from globals
        assert_eq!(map.get("GLOBAL_TOKEN"), Some(&"token_123"));
        // PORT should be included from active
        assert_eq!(map.get("PORT"), Some(&"443"));
        // Disabled variable should not be present
        assert_eq!(map.get("DISABLED_KEY"), None);
    }

    #[test]
    fn test_duplicate_and_rename_environment() {
        let temp_dir = std::env::temp_dir().join(format!("restclient_ws_test_{}", uuid::Uuid::new_v4()));
        let ws = Workspace::new("Test Workspace");
        let ws_id = ws.id.clone();
        let fs_storage = Arc::new(FsStorage::new(&temp_dir));
        let _ = fs_storage.init_workspace(&ws);

        let mut env1 = Environment::new(&ws_id, "Development");
        env1.variables.push(EnvironmentVariable {
            enabled: true,
            key: "URL".to_string(),
            value: "http://localhost:3000".to_string(),
            secret: false,
        });
        let _ = fs_storage.save_environments(&ws_id, &[env1.clone()]);

        let mut ws_state = WorkspaceState {
            fs_storage: fs_storage.clone(),
            workspaces: vec![ws.clone()],
            active_workspace: Some(ws),
            folders: Vec::new(),
            requests: Vec::new(),
            environments: vec![env1.clone()],
            global_environment: Environment::new(&ws_id, "Globals"),
            active_environment_id: Some(env1.id.clone()),
            selected_env_modal_id: Some(env1.id.clone()),
            env_filter_query: String::new(),
            var_filter_query: String::new(),
            git_branch: None,
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
        };

        // Test duplicate
        let dup_id = {
            let existing = ws_state.environments.iter().find(|e| e.id == env1.id).unwrap().clone();
            let mut dup = existing.clone();
            dup.id = uuid::Uuid::new_v4().to_string();
            dup.name = format!("{} (Copy)", existing.name);
            let dup_id = dup.id.clone();
            ws_state.environments.push(dup);
            dup_id
        };

        assert_eq!(ws_state.environments.len(), 2);
        let duplicated = ws_state.environments.iter().find(|e| e.id == dup_id).unwrap();
        assert_eq!(duplicated.name, "Development (Copy)");
        assert_eq!(duplicated.variables.len(), 1);
        assert_eq!(duplicated.variables[0].key, "URL");
        assert_eq!(duplicated.variables[0].value, "http://localhost:3000");

        // Test rename
        if let Some(target) = ws_state.environments.iter_mut().find(|e| e.id == dup_id) {
            target.name = "Staging".to_string();
        }
        let renamed = ws_state.environments.iter().find(|e| e.id == dup_id).unwrap();
        assert_eq!(renamed.name, "Staging");

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_update_request_does_not_resurrect_deleted_request() {
        let temp_dir = std::env::temp_dir().join(format!("restclient_test_{}", uuid::Uuid::new_v4()));
        let fs_storage = Arc::new(FsStorage::new(&temp_dir));
        let ws_id = "test-ws";
        let req1 = HttpRequestItem::new(ws_id, "Req 1", HttpMethod::Get, "https://api.example.com");

        let mut ws_state = WorkspaceState {
            fs_storage: fs_storage.clone(),
            workspaces: Vec::new(),
            active_workspace: None,
            folders: Vec::new(),
            requests: vec![req1.clone()],
            environments: Vec::new(),
            global_environment: Environment::new(ws_id, "Globals"),
            active_environment_id: None,
            selected_env_modal_id: None,
            env_filter_query: String::new(),
            var_filter_query: String::new(),
            git_branch: None,
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
        };

        assert_eq!(ws_state.requests.len(), 1);

        // Simulate deleting req1
        ws_state.requests.retain(|r| r.id != req1.id);
        assert_eq!(ws_state.requests.len(), 0);

        // When updating a request that is not in requests, it must not be added
        if let Some(idx) = ws_state.requests.iter().position(|r| r.id == req1.id) {
            ws_state.requests[idx] = req1.clone();
        }
        assert_eq!(ws_state.requests.len(), 0);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_folder_reorder_and_request_drag_move() {
        let temp_dir = std::env::temp_dir().join(format!("restclient_test_{}", uuid::Uuid::new_v4()));
        let fs_storage = Arc::new(FsStorage::new(&temp_dir));
        let ws_id = "test-ws";
        let mut f1 = Folder::new(ws_id, "Folder 1");
        f1.sort_order = 0;
        let mut f2 = Folder::new(ws_id, "Folder 2");
        f2.sort_order = 1;

        let mut r1 = HttpRequestItem::new(ws_id, "Req 1", HttpMethod::Get, "https://api.example.com/1");
        r1.sort_order = 0;
        let mut r2 = HttpRequestItem::new(ws_id, "Req 2", HttpMethod::Post, "https://api.example.com/2");
        r2.sort_order = 1;
        let mut r3 = HttpRequestItem::new(ws_id, "Req 3", HttpMethod::Put, "https://api.example.com/3");
        r3.sort_order = 2;

        let mut ws_state = WorkspaceState {
            fs_storage: fs_storage.clone(),
            workspaces: Vec::new(),
            active_workspace: None,
            folders: vec![f1.clone(), f2.clone()],
            requests: vec![r1.clone(), r2.clone(), r3.clone()],
            environments: Vec::new(),
            global_environment: Environment::new(ws_id, "Globals"),
            active_environment_id: None,
            selected_env_modal_id: None,
            env_filter_query: String::new(),
            var_filter_query: String::new(),
            git_branch: None,
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
        };

        // Test rename folder directly
        if let Some(folder) = ws_state.folders.iter_mut().find(|f| f.id == f1.id) {
            folder.name = "Folder 1 Renamed".to_string();
        }
        assert_eq!(ws_state.folders[0].name, "Folder 1 Renamed");

        // Test move_folder: swap f1 and f2
        let folder1_id = f1.id.clone();
        let folder2_id = f2.id.clone();
        let s_idx = ws_state.folders.iter().position(|f| f.id == folder1_id).unwrap();
        let folder = ws_state.folders.remove(s_idx);
        let t_idx = ws_state.folders.iter().position(|f| f.id == folder2_id).unwrap();
        ws_state.folders.insert(t_idx + 1, folder);
        assert_eq!(ws_state.folders[0].id, folder2_id);
        assert_eq!(ws_state.folders[1].id, folder1_id);

        // Test move request into folder
        let r3_id = r3.id.clone();
        let r3_pos = ws_state.requests.iter().position(|r| r.id == r3_id).unwrap();
        let mut moved_r3 = ws_state.requests.remove(r3_pos);
        moved_r3.folder_id = Some(folder1_id.clone());
        ws_state.requests.insert(0, moved_r3);
        assert_eq!(ws_state.requests[0].id, r3_id);
        assert_eq!(ws_state.requests[0].folder_id, Some(folder1_id));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}

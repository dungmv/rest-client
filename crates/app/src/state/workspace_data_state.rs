use crate::settings::AppSettings;
use gpui::Context;
use restclient_core::models::{Auth, Folder, HttpRequestItem, KeyValuePair, Workspace};
use restclient_core::storage::models::{FolderNode, WorkspaceMeta};
use restclient_core::storage::{FsWorkspaceStore, WorkspaceStore};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

pub struct WorkspaceDataState {
    pub store: Arc<dyn WorkspaceStore>,
    pub workspace_path: PathBuf,
    pub workspace_meta: WorkspaceMeta,
    pub active_workspace: Option<Workspace>,
    pub folders: Vec<Folder>,
    pub requests: Vec<HttpRequestItem>,
    pub scratch_request: HttpRequestItem,
    pub parse_errors: HashMap<String, String>,
    pub pins: Vec<String>,
    pub last_renamed_request: Option<(String, String)>,
    pub last_renamed_example: Option<(String, String, String)>,
    pub recent_workspaces: Vec<String>,
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
            scratch_request: HttpRequestItem::new(
                "scratch",
                "Untitled Request",
                restclient_core::models::HttpMethod::Get,
                "",
            ),
            parse_errors: HashMap::new(),
            pins: Vec::new(),
            last_renamed_request: None,
            last_renamed_example: None,
            recent_workspaces: settings.recent_workspaces,
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
            scratch_request: HttpRequestItem::new(
                "scratch",
                "Untitled Request",
                restclient_core::models::HttpMethod::Get,
                "",
            ),
            parse_errors: HashMap::new(),
            pins: Vec::new(),
            last_renamed_request: None,
            last_renamed_example: None,
            recent_workspaces: Vec::new(),
        };
        state.reload_all();
        state
    }

    pub fn open_workspace(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        let store: Arc<dyn WorkspaceStore> = Arc::new(FsWorkspaceStore::new(&path));
        if !path.join("workspace.yaml").exists() {
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("Workspace");
            let _ = store.init_workspace(name);
        }
        let _ = store.ensure_gitignore();
        self.store = store;
        self.workspace_path = path.clone();

        let mut settings = AppSettings::load();
        settings.add_recent_workspace(&path.to_string_lossy());
        self.recent_workspaces = settings.recent_workspaces;

        self.reload_all();
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
            active_environment_id: None,
            headers: self.workspace_meta.headers.clone(),
            auth: self.workspace_meta.auth.clone(),
        });

        self.pins = self.store.load_pins().unwrap_or_default();
        self.reload_tree();
    }

    pub fn reload_tree(&mut self) {
        if let Ok(tree) = self.store.scan_tree() {
            let ws_name = self.workspace_meta.name.clone();
            let mut all_requests = Vec::new();
            let mut all_folders = Vec::new();
            let mut errors = HashMap::new();

            for req_node in tree.root_requests {
                let mut item = req_node.file.into_item(&req_node.rel_path, &ws_name, None);
                item.last_status_code = req_node.last_status_code;
                all_requests.push(item);
                if let Some(err) = req_node.parse_error {
                    errors.insert(req_node.rel_path, err);
                }
            }

            fn process_folder(
                folder_node: FolderNode,
                parent_id: Option<String>,
                ws_name: &str,
                all_folders: &mut Vec<Folder>,
                all_requests: &mut Vec<HttpRequestItem>,
                errors: &mut HashMap<String, String>,
            ) {
                let mut f = Folder::new(ws_name, &folder_node.file.name);
                f.id = folder_node.rel_path.clone();
                f.parent_id = parent_id;
                f.headers = folder_node.file.headers;
                f.auth = folder_node.file.auth;
                all_folders.push(f);

                let folder_rel = folder_node.rel_path;
                for req_node in folder_node.requests {
                    let mut item = req_node.file.into_item(
                        &req_node.rel_path,
                        ws_name,
                        Some(folder_rel.clone()),
                    );
                    item.last_status_code = req_node.last_status_code;
                    all_requests.push(item);
                    if let Some(err) = req_node.parse_error {
                        errors.insert(req_node.rel_path, err);
                    }
                }

                for sub in folder_node.subfolders {
                    process_folder(
                        sub,
                        Some(folder_rel.clone()),
                        ws_name,
                        all_folders,
                        all_requests,
                        errors,
                    );
                }
            }

            for f_node in tree.folders {
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

    pub fn find_request(&self, req_id: &str) -> Option<&HttpRequestItem> {
        if req_id == "scratch" {
            Some(&self.scratch_request)
        } else {
            self.requests.iter().find(|r| r.id == req_id)
        }
    }

    pub fn find_request_mut(&mut self, req_id: &str) -> Option<&mut HttpRequestItem> {
        if req_id == "scratch" {
            Some(&mut self.scratch_request)
        } else {
            self.requests.iter_mut().find(|r| r.id == req_id)
        }
    }

    pub fn mutate_request<F, R>(&mut self, req_id: &str, f: F) -> Option<R>
    where
        F: FnOnce(&mut HttpRequestItem) -> R,
    {
        self.find_request_mut(req_id).map(f)
    }

    pub fn update_active_request<F>(&mut self, req_id: &str, f: F)
    where
        F: FnOnce(&mut HttpRequestItem),
    {
        if let Some(r) = self.find_request_mut(req_id) {
            f(r);
        }
    }

    pub fn save_workspace_scope_settings(
        &mut self,
        headers: Vec<KeyValuePair>,
        auth: Auth,
        cx: &mut Context<Self>,
    ) {
        self.workspace_meta.headers = headers.clone();
        self.workspace_meta.auth = auth.clone();
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
            meta.headers = headers;
            meta.auth = auth;
            let _ = self.store.save_folder_meta(folder_id, &meta);
        }
        cx.notify();
    }

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

    pub fn resolve_request_hierarchy(&self, req: &HttpRequestItem) -> (Vec<KeyValuePair>, Auth) {
        let ws_headers = self
            .active_workspace
            .as_ref()
            .map(|w| w.headers.as_slice())
            .unwrap_or(&[]);
        let ws_auth = self
            .active_workspace
            .as_ref()
            .map(|w| &w.auth)
            .unwrap_or(&Auth::None);

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
        let ws_auth = self
            .active_workspace
            .as_ref()
            .map(|w| &w.auth)
            .unwrap_or(&Auth::None);
        let ws_name = self
            .active_workspace
            .as_ref()
            .map(|w| w.name.as_str())
            .unwrap_or("Workspace");
        match ws_auth {
            Auth::None => format!("Inherited from workspace \"{}\" (No Auth)", ws_name),
            Auth::Bearer { .. } => {
                format!("Inherited from workspace \"{}\" (Bearer Token)", ws_name)
            }
            Auth::Basic { .. } => format!("Inherited from workspace \"{}\" (Basic Auth)", ws_name),
            Auth::ApiKey { .. } => format!("Inherited from workspace \"{}\" (API Key)", ws_name),
            Auth::Inherit => "No auth configured in parent hierarchy".to_string(),
        }
    }
}

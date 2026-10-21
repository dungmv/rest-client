use super::WorkspaceDataState;
use gpui::Context;
use restclient_core::models::{Auth, Environment, Folder, HttpRequestItem, KeyValuePair};
use restclient_core::storage::models::EnvironmentFile;

impl WorkspaceDataState {
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

    pub fn save_environments(&mut self, envs: Vec<Environment>, cx: &mut Context<Self>) {
        self.env_save_task = None;
        for env in &envs {
            let (file, secrets) = EnvironmentFile::from_environment(env);
            let _ = self.store.save_environment(&env.name, &file);
            let _ = self.store.save_secrets(&env.name, &secrets);
        }
        self.reload_environments();
        cx.notify();
    }

    pub fn save_environments_debounced(&mut self, envs: Vec<Environment>, cx: &mut Context<Self>) {
        let store = self.store.clone();
        let envs_clone = envs.clone();
        let task = cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(std::time::Duration::from_millis(500))
                .await;
            cx.background_executor()
                .spawn(async move {
                    for env in &envs_clone {
                        let (file, secrets) = EnvironmentFile::from_environment(env);
                        let _ = store.save_environment(&env.name, &file);
                        let _ = store.save_secrets(&env.name, &secrets);
                    }
                })
                .await;
            if let Some(this) = this.upgrade() {
                this.update(cx, |ws, cx| {
                    ws.reload_environments();
                    cx.notify();
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
            self.reload_environments();
        }
    }
}

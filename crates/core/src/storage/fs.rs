use crate::models::{Environment, Folder, HttpRequestItem, Workspace};
use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

pub struct FsStorage {
    root_dir: PathBuf,
}

impl FsStorage {
    pub fn new(root_dir: impl Into<PathBuf>) -> Self {
        let root_dir = root_dir.into();
        Self { root_dir }
    }

    pub fn default_storage_dir() -> PathBuf {
        if let Some(user_dirs) = dirs::data_local_dir() {
            user_dirs.join("restclient").join("workspaces")
        } else {
            PathBuf::from("./data/workspaces")
        }
    }

    pub fn workspace_dir(&self, workspace_id: &str) -> PathBuf {
        self.root_dir.join(workspace_id)
    }

    pub fn requests_dir(&self, workspace_id: &str) -> PathBuf {
        self.workspace_dir(workspace_id).join("requests")
    }

    pub fn init_workspace(&self, workspace: &Workspace) -> Result<()> {
        let ws_dir = self.workspace_dir(&workspace.id);
        fs::create_dir_all(&ws_dir)?;
        fs::create_dir_all(self.requests_dir(&workspace.id))?;

        self.save_workspace(workspace)?;
        Ok(())
    }

    pub fn save_workspace(&self, workspace: &Workspace) -> Result<()> {
        let ws_dir = self.workspace_dir(&workspace.id);
        fs::create_dir_all(&ws_dir)?;
        let path = ws_dir.join("workspace.json");
        let json = serde_json::to_string_pretty(workspace)?;
        fs::write(path, json)?;
        Ok(())
    }

    pub fn load_workspace(&self, workspace_id: &str) -> Result<Workspace> {
        let path = self.workspace_dir(workspace_id).join("workspace.json");
        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read workspace file at {:?}", path))?;
        let ws: Workspace = serde_json::from_str(&content)?;
        Ok(ws)
    }

    pub fn list_workspaces(&self) -> Result<Vec<Workspace>> {
        if !self.root_dir.exists() {
            fs::create_dir_all(&self.root_dir)?;
            return Ok(Vec::new());
        }

        let mut workspaces = Vec::new();
        for entry in fs::read_dir(&self.root_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                let ws_file = entry.path().join("workspace.json");
                if ws_file.exists() {
                    if let Ok(content) = fs::read_to_string(&ws_file) {
                        if let Ok(ws) = serde_json::from_str::<Workspace>(&content) {
                            workspaces.push(ws);
                        }
                    }
                }
            }
        }
        Ok(workspaces)
    }

    pub fn save_folders(&self, workspace_id: &str, folders: &[Folder]) -> Result<()> {
        let path = self.workspace_dir(workspace_id).join("folders.json");
        let json = serde_json::to_string_pretty(folders)?;
        fs::write(path, json)?;
        Ok(())
    }

    pub fn load_folders(&self, workspace_id: &str) -> Result<Vec<Folder>> {
        let path = self.workspace_dir(workspace_id).join("folders.json");
        if !path.exists() {
            return Ok(Vec::new());
        }
        let content = fs::read_to_string(path)?;
        let folders: Vec<Folder> = serde_json::from_str(&content)?;
        Ok(folders)
    }

    pub fn save_environments(&self, workspace_id: &str, environments: &[Environment]) -> Result<()> {
        let path = self.workspace_dir(workspace_id).join("environments.json");
        let json = serde_json::to_string_pretty(environments)?;
        fs::write(path, json)?;
        Ok(())
    }

    pub fn load_environments(&self, workspace_id: &str) -> Result<Vec<Environment>> {
        let path = self.workspace_dir(workspace_id).join("environments.json");
        if !path.exists() {
            return Ok(Vec::new());
        }
        let content = fs::read_to_string(path)?;
        let envs: Vec<Environment> = serde_json::from_str(&content)?;
        Ok(envs)
    }

    pub fn save_global_environment(&self, workspace_id: &str, environment: &Environment) -> Result<()> {
        let ws_dir = self.workspace_dir(workspace_id);
        fs::create_dir_all(&ws_dir)?;
        let path = ws_dir.join("globals.json");
        let json = serde_json::to_string_pretty(environment)?;
        fs::write(path, json)?;
        Ok(())
    }

    pub fn load_global_environment(&self, workspace_id: &str) -> Result<Environment> {
        let path = self.workspace_dir(workspace_id).join("globals.json");
        if !path.exists() {
            let mut global = Environment::new(workspace_id, "Globals");
            global.id = format!("{}-globals", workspace_id);
            return Ok(global);
        }
        let content = fs::read_to_string(path)?;
        let env: Environment = serde_json::from_str(&content)?;
        Ok(env)
    }

    pub fn save_request(&self, request: &HttpRequestItem) -> Result<()> {
        let req_dir = self.requests_dir(&request.workspace_id);
        fs::create_dir_all(&req_dir)?;
        let path = req_dir.join(format!("{}.json", request.id));
        let json = serde_json::to_string_pretty(request)?;
        fs::write(path, json)?;
        Ok(())
    }

    pub fn load_request(&self, workspace_id: &str, request_id: &str) -> Result<HttpRequestItem> {
        let path = self.requests_dir(workspace_id).join(format!("{}.json", request_id));
        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read request file at {:?}", path))?;
        let req: HttpRequestItem = serde_json::from_str(&content)?;
        Ok(req)
    }

    pub fn delete_request(&self, workspace_id: &str, request_id: &str) -> Result<()> {
        let path = self.requests_dir(workspace_id).join(format!("{}.json", request_id));
        if path.exists() {
            fs::remove_file(path)?;
        }
        Ok(())
    }

    pub fn list_requests(&self, workspace_id: &str) -> Result<Vec<HttpRequestItem>> {
        let req_dir = self.requests_dir(workspace_id);
        if !req_dir.exists() {
            return Ok(Vec::new());
        }

        let mut requests = Vec::new();
        for entry in fs::read_dir(req_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(req) = serde_json::from_str::<HttpRequestItem>(&content) {
                        requests.push(req);
                    }
                }
            }
        }
        requests.sort_by_key(|r| r.sort_order);
        Ok(requests)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::EnvironmentVariable;

    #[test]
    fn test_save_and_load_global_environment() {
        let temp_dir = std::env::temp_dir().join(format!("restclient_test_{}", uuid::Uuid::new_v4()));
        let storage = FsStorage::new(&temp_dir);
        let ws_id = "test-workspace";

        // When globals.json does not exist yet
        let default_global = storage.load_global_environment(ws_id).unwrap();
        assert_eq!(default_global.name, "Globals");
        assert_eq!(default_global.id, format!("{}-globals", ws_id));
        assert!(default_global.variables.is_empty());

        // Save global variables
        let mut global = default_global;
        global.variables.push(EnvironmentVariable {
            key: "GLOBAL_API_KEY".to_string(),
            initial_value: "secret_123".to_string(),
            current_value: "".to_string(),
            secret: true,
        });
        storage.save_global_environment(ws_id, &global).unwrap();

        // Load again
        let loaded = storage.load_global_environment(ws_id).unwrap();
        assert_eq!(loaded.variables.len(), 1);
        assert_eq!(loaded.variables[0].key, "GLOBAL_API_KEY");
        assert_eq!(loaded.variables[0].initial_value, "secret_123");
        assert!(loaded.variables[0].secret);

        let _ = fs::remove_dir_all(&temp_dir);
    }
}

use super::WorkspaceDataState;
use crate::settings::AppSettings;
use gpui::Context;
use restclient_core::models::{Folder, HttpMethod, HttpRequestItem};
use restclient_core::storage::models::RequestFile;
use std::path::PathBuf;

impl WorkspaceDataState {
    pub fn create_request_internal(&mut self, folder_rel: Option<String>) -> HttpRequestItem {
        let (_slug, rel_path) = self
            .store
            .create_request(folder_rel.as_deref(), "New Request", HttpMethod::Get, "")
            .unwrap_or_else(|_| (format!("req-{}", uuid::Uuid::new_v4().simple()), "new-request.yaml".to_string()));

        let ws_name = self.workspace_meta.name.clone();
        let mut item = HttpRequestItem::new(&ws_name, "New Request", HttpMethod::Get, "");
        item.id = rel_path.clone();
        item.folder_id = folder_rel;

        self.requests.push(item.clone());
        item
    }

    pub fn create_request(&mut self, folder_rel: Option<String>, cx: &mut Context<Self>) -> HttpRequestItem {
        let item = self.create_request_internal(folder_rel);
        cx.notify();
        item
    }

    pub fn duplicate_request_internal(&mut self, req_id: &str) -> Option<HttpRequestItem> {
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
        let ws_name = self.workspace_meta.name.clone();
        let mut dup = req_file.to_item(&rel_path, &ws_name, folder_rel);
        dup.last_status_code = existing.last_status_code;
        self.requests.push(dup.clone());
        Some(dup)
    }

    pub fn duplicate_request(&mut self, req_id: &str, cx: &mut Context<Self>) -> Option<HttpRequestItem> {
        let res = self.duplicate_request_internal(req_id);
        if res.is_some() {
            cx.notify();
        }
        res
    }

    pub fn delete_request_internal(&mut self, req_id: &str) {
        let _ = self.store.delete_request(req_id);
        self.requests.retain(|r| r.id != req_id);
        self.parse_errors.remove(req_id);
        self.pins.retain(|p| p != req_id);
        let _ = self.store.save_pins(&self.pins);
    }

    pub fn delete_request(&mut self, req_id: &str, cx: &mut Context<Self>) {
        self.delete_request_internal(req_id);
        cx.notify();
    }

    pub fn rename_request_internal(&mut self, req_id: &str, new_name: &str) -> Option<String> {
        let trimmed = new_name.trim();
        if trimmed.is_empty() {
            return None;
        }
        if let Ok(new_rel_path) = self.store.rename_request(req_id, trimmed) {
            self.last_renamed_request = Some((req_id.to_string(), new_rel_path.clone()));

            if let Some(r) = self.requests.iter_mut().find(|r| r.id == req_id) {
                r.id = new_rel_path.clone();
                r.name = trimmed.to_string();
            }

            if let Some(pos) = self.pins.iter().position(|p| p == req_id) {
                self.pins[pos] = new_rel_path.clone();
                let _ = self.store.save_pins(&self.pins);
            }

            if let Some(err) = self.parse_errors.remove(req_id) {
                self.parse_errors.insert(new_rel_path.clone(), err);
            }

            Some(new_rel_path)
        } else {
            None
        }
    }

    pub fn rename_request(&mut self, req_id: &str, new_name: &str, cx: &mut Context<Self>) -> Option<String> {
        let res = self.rename_request_internal(req_id, new_name);
        if res.is_some() {
            cx.notify();
        }
        res
    }

    pub fn create_folder_internal(&mut self, parent_id: Option<String>, name: String) -> Option<String> {
        let trimmed = name.trim();
        if !trimmed.is_empty() {
            if let Ok((_dir_name, rel_path)) = self.store.create_group(parent_id.as_deref(), trimmed) {
                let ws_name = self.workspace_meta.name.clone();
                let mut f = Folder::new(&ws_name, trimmed);
                f.id = rel_path.clone();
                f.parent_id = parent_id;
                self.folders.push(f);
                return Some(rel_path);
            }
        }
        None
    }

    pub fn create_folder(&mut self, parent_id: Option<String>, name: String, cx: &mut Context<Self>) -> Option<String> {
        let res = self.create_folder_internal(parent_id, name);
        if res.is_some() {
            cx.notify();
        }
        res
    }

    pub fn delete_folder_internal(&mut self, folder_id: &str) {
        let _ = self.store.delete_group(folder_id);
        let prefix = format!("{}/", folder_id);
        self.folders.retain(|f| f.id != folder_id && !f.id.starts_with(&prefix));
        self.requests.retain(|r| {
            if let Some(fid) = &r.folder_id {
                fid != folder_id && !fid.starts_with(&prefix)
            } else {
                !r.id.starts_with(&prefix)
            }
        });
        self.pins.retain(|p| p != folder_id && !p.starts_with(&prefix));
        let _ = self.store.save_pins(&self.pins);
    }

    pub fn delete_folder(&mut self, folder_id: &str, cx: &mut Context<Self>) {
        self.delete_folder_internal(folder_id);
        cx.notify();
    }

    pub fn rename_folder_internal(&mut self, folder_id: &str, new_name: &str) -> Option<String> {
        let trimmed = new_name.trim();
        if trimmed.is_empty() {
            return None;
        }
        if let Ok(new_rel_path) = self.store.rename_group(folder_id, trimmed) {
            let old_prefix = format!("{}/", folder_id);
            let new_prefix = format!("{}/", new_rel_path);

            if let Some(f) = self.folders.iter_mut().find(|f| f.id == folder_id) {
                f.id = new_rel_path.clone();
                f.name = trimmed.to_string();
            }

            for f in self.folders.iter_mut() {
                if f.parent_id.as_deref() == Some(folder_id) {
                    f.parent_id = Some(new_rel_path.clone());
                }
                if f.id.starts_with(&old_prefix) {
                    f.id = format!("{}{}", new_prefix, &f.id[old_prefix.len()..]);
                }
            }

            for r in self.requests.iter_mut() {
                if r.folder_id.as_deref() == Some(folder_id) {
                    r.folder_id = Some(new_rel_path.clone());
                } else if let Some(fid) = &r.folder_id {
                    if fid.starts_with(&old_prefix) {
                        r.folder_id = Some(format!("{}{}", new_prefix, &fid[old_prefix.len()..]));
                    }
                }
                if r.id.starts_with(&old_prefix) {
                    r.id = format!("{}{}", new_prefix, &r.id[old_prefix.len()..]);
                }
            }

            for p in self.pins.iter_mut() {
                if p.starts_with(&old_prefix) {
                    *p = format!("{}{}", new_prefix, &p[old_prefix.len()..]);
                }
            }
            let _ = self.store.save_pins(&self.pins);

            Some(new_rel_path)
        } else {
            None
        }
    }

    pub fn rename_folder(&mut self, folder_id: &str, new_name: String, cx: &mut Context<Self>) {
        if self.rename_folder_internal(folder_id, &new_name).is_some() {
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
            cx.notify();
        }
    }

    pub fn move_request(&mut self, src_req_id: &str, target_req_id: &str, cx: &mut Context<Self>) {
        if src_req_id == target_req_id {
            return;
        }
        let src_req = self.requests.iter().find(|r| r.id == src_req_id).cloned();
        let target_req = self.requests.iter().find(|r| r.id == target_req_id).cloned();

        if let (Some(src), Some(target)) = (src_req, target_req) {
            if src.folder_id != target.folder_id {
                if let Ok(new_rel_path) = self.store.move_request(src_req_id, target.folder_id.as_deref()) {
                    if let Some(r) = self.requests.iter_mut().find(|r| r.id == src_req_id) {
                        r.id = new_rel_path.clone();
                        r.folder_id = target.folder_id.clone();
                    }
                    if let Some(pos) = self.pins.iter().position(|p| p == src_req_id) {
                        self.pins[pos] = new_rel_path;
                        let _ = self.store.save_pins(&self.pins);
                    }
                }
            } else {
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
                    let src_idx = self.requests.iter().position(|r| r.id == src_req_id);
                    let target_idx = self.requests.iter().position(|r| r.id == target_req_id);
                    if let (Some(si), Some(ti)) = (src_idx, target_idx) {
                        let item = self.requests.remove(si);
                        self.requests.insert(ti, item);
                    }
                }
            }
        }
        cx.notify();
    }

    pub fn move_request_to_folder_internal(
        &mut self,
        src_req_id: &str,
        target_folder_id: Option<String>,
    ) -> Option<String> {
        if let Ok(new_rel_path) = self.store.move_request(src_req_id, target_folder_id.as_deref()) {
            if let Some(r) = self.requests.iter_mut().find(|r| r.id == src_req_id) {
                r.id = new_rel_path.clone();
                r.folder_id = target_folder_id;
            }
            if let Some(pos) = self.pins.iter().position(|p| p == src_req_id) {
                self.pins[pos] = new_rel_path.clone();
                let _ = self.store.save_pins(&self.pins);
            }
            Some(new_rel_path)
        } else {
            None
        }
    }

    pub fn move_request_to_folder(
        &mut self,
        src_req_id: &str,
        target_folder_id: Option<String>,
        cx: &mut Context<Self>,
    ) {
        if self.move_request_to_folder_internal(src_req_id, target_folder_id).is_some() {
            cx.notify();
        }
    }

    pub fn rename_example_internal(&mut self, req_id: &str, old_name: &str, new_name: &str) -> bool {
        let trimmed = new_name.trim();
        if trimmed.is_empty() || trimmed == old_name {
            return false;
        }

        if let Ok(mut req_file) = self.store.read_request(req_id) {
            let mut unique_name = trimmed.to_string();
            let mut idx = 2;
            while req_file.examples.iter().any(|e| e.name == unique_name && e.name != old_name) {
                unique_name = format!("{}-{}", trimmed, idx);
                idx += 1;
            }
            if let Some(ex) = req_file.examples.iter_mut().find(|e| e.name == old_name) {
                ex.name = unique_name.clone();
            }
            self.last_renamed_example = Some((req_id.to_string(), old_name.to_string(), unique_name.clone()));
            let _ = self.store.write_request(req_id, &req_file);
            if let Some(r) = self.requests.iter_mut().find(|r| r.id == req_id) {
                if let Some(ex) = r.examples.iter_mut().find(|e| e.name == old_name) {
                    ex.name = unique_name;
                }
            }
            return true;
        }
        false
    }

    pub fn rename_example(&mut self, req_id: &str, old_name: &str, new_name: &str, cx: &mut Context<Self>) {
        if self.rename_example_internal(req_id, old_name, new_name) {
            cx.notify();
        }
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
    }

    pub fn clean_history(&mut self, cx: &mut Context<Self>) {
        self.clean_history_internal();
        cx.notify();
    }

    pub fn clear_cookies(&mut self, cx: &mut Context<Self>) {
        let _ = self.store.save_cookies(&serde_json::json!({}));
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

    pub fn get_request_file(&self, req_id: &str) -> Option<RequestFile> {
        self.store.read_request(req_id).ok()
    }

    pub fn save_request_file_internal(&mut self, req_id: &str, req_file: &RequestFile) {
        let _ = self.store.write_request(req_id, req_file);
        if let Some(r) = self.requests.iter_mut().find(|r| r.id == req_id) {
            let prev_status = r.last_status_code;
            let folder_id = r.folder_id.clone();
            *r = req_file.to_item(req_id, &self.workspace_meta.name, folder_id);
            r.last_status_code = prev_status;
        }
    }

    pub fn save_request_file(&mut self, req_id: &str, req_file: &RequestFile, cx: &mut Context<Self>) {
        self.save_request_file_internal(req_id, req_file);
        cx.notify();
    }
}

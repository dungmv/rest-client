use super::GitState;
use gpui::Context;

impl GitState {
    pub fn stage_file(&mut self, path: &str, cx: &mut Context<Self>) {
        let ws_path = self.repo_path.clone();
        let p = path.to_string();
        cx.spawn(async move |this, cx| {
            let _ = cx.background_executor().spawn(async move {
                restclient_core::git::git_stage_file(&ws_path, &p)
            }).await;
            if let Some(this) = this.upgrade() {
                this.update(cx, |git, cx| {
                    git.refresh_status(cx);
                });
            }
        }).detach();
    }

    pub fn unstage_file(&mut self, path: &str, cx: &mut Context<Self>) {
        let ws_path = self.repo_path.clone();
        let p = path.to_string();
        cx.spawn(async move |this, cx| {
            let _ = cx.background_executor().spawn(async move {
                restclient_core::git::git_unstage_file(&ws_path, &p)
            }).await;
            if let Some(this) = this.upgrade() {
                this.update(cx, |git, cx| {
                    git.refresh_status(cx);
                });
            }
        }).detach();
    }

    pub fn stage_all(&mut self, cx: &mut Context<Self>) {
        let ws_path = self.repo_path.clone();
        cx.spawn(async move |this, cx| {
            let _ = cx.background_executor().spawn(async move {
                restclient_core::git::git_stage_all(&ws_path)
            }).await;
            if let Some(this) = this.upgrade() {
                this.update(cx, |git, cx| {
                    git.refresh_status(cx);
                });
            }
        }).detach();
    }

    pub fn unstage_all(&mut self, cx: &mut Context<Self>) {
        let ws_path = self.repo_path.clone();
        cx.spawn(async move |this, cx| {
            let _ = cx.background_executor().spawn(async move {
                restclient_core::git::git_unstage_all(&ws_path)
            }).await;
            if let Some(this) = this.upgrade() {
                this.update(cx, |git, cx| {
                    git.refresh_status(cx);
                });
            }
        }).detach();
    }

    pub fn toggle_unstaged_selection(&mut self, path: &str, cx: &mut Context<Self>) {
        if self.selected_unstaged_files.contains(path) {
            self.selected_unstaged_files.remove(path);
        } else {
            self.selected_unstaged_files.insert(path.to_string());
        }
        cx.notify();
    }

    pub fn toggle_all_unstaged_selection(&mut self, select: bool, cx: &mut Context<Self>) {
        if let Some(st) = &self.status {
            if select {
                self.selected_unstaged_files = st.unstaged_files.iter().chain(&st.untracked_files).map(|f| f.path.clone()).collect();
            } else {
                self.selected_unstaged_files.clear();
            }
        }
        cx.notify();
    }

    pub fn toggle_staged_selection(&mut self, path: &str, cx: &mut Context<Self>) {
        if self.selected_staged_files.contains(path) {
            self.selected_staged_files.remove(path);
        } else {
            self.selected_staged_files.insert(path.to_string());
        }
        cx.notify();
    }

    pub fn toggle_all_staged_selection(&mut self, select: bool, cx: &mut Context<Self>) {
        if let Some(st) = &self.status {
            if select {
                self.selected_staged_files = st.staged_files.iter().map(|f| f.path.clone()).collect();
            } else {
                self.selected_staged_files.clear();
            }
        }
        cx.notify();
    }

    pub fn stage_selected(&mut self, cx: &mut Context<Self>) {
        let paths: Vec<String> = self.selected_unstaged_files.iter().cloned().collect();
        if paths.is_empty() {
            return;
        }
        self.selected_unstaged_files.clear();
        let ws_path = self.repo_path.clone();
        cx.spawn(async move |this, cx| {
            let _ = cx.background_executor().spawn(async move {
                let str_paths: Vec<&str> = paths.iter().map(|s| s.as_str()).collect();
                restclient_core::git::git_stage_files(&ws_path, &str_paths)
            }).await;
            if let Some(this) = this.upgrade() {
                this.update(cx, |git, cx| {
                    git.refresh_status(cx);
                });
            }
        }).detach();
    }

    pub fn unstage_selected(&mut self, cx: &mut Context<Self>) {
        let paths: Vec<String> = self.selected_staged_files.iter().cloned().collect();
        if paths.is_empty() {
            return;
        }
        self.selected_staged_files.clear();
        let ws_path = self.repo_path.clone();
        cx.spawn(async move |this, cx| {
            let _ = cx.background_executor().spawn(async move {
                let str_paths: Vec<&str> = paths.iter().map(|s| s.as_str()).collect();
                restclient_core::git::git_unstage_files(&ws_path, &str_paths)
            }).await;
            if let Some(this) = this.upgrade() {
                this.update(cx, |git, cx| {
                    git.refresh_status(cx);
                });
            }
        }).detach();
    }

    pub fn discard_file(&mut self, path: &str, cx: &mut Context<Self>) {
        let _ = restclient_core::git::git_discard_file(&self.repo_path, path);
        if let Some((sel, _)) = &self.selected_file {
            if sel == path {
                self.selected_file = None;
                self.diff_content = None;
            }
        }
        self.refresh_status(cx);
    }

    pub fn commit(&mut self, message: &str, cx: &mut Context<Self>) -> Result<(), String> {
        match restclient_core::git::git_commit(&self.repo_path, message) {
            Ok(_) => {
                self.commit_message.clear();
                self.error = None;
                self.selected_file = None;
                self.diff_content = None;
                self.refresh_all(cx);
                Ok(())
            }
            Err(e) => {
                let err_str = e.to_string();
                self.error = Some(err_str.clone());
                cx.notify();
                Err(err_str)
            }
        }
    }

    pub fn checkout(&mut self, branch: &str, cx: &mut Context<Self>) -> Result<(), String> {
        match restclient_core::git::git_checkout(&self.repo_path, branch) {
            Ok(_) => {
                self.branch = Some(branch.to_string());
                self.error = None;
                self.refresh_all(cx);
                Ok(())
            }
            Err(e) => {
                let err_str = e.to_string();
                self.error = Some(err_str.clone());
                cx.notify();
                Err(err_str)
            }
        }
    }

    pub fn create_branch(&mut self, name: &str, cx: &mut Context<Self>) -> Result<(), String> {
        match restclient_core::git::git_create_branch(&self.repo_path, name) {
            Ok(_) => {
                self.branch = Some(name.to_string());
                self.error = None;
                self.refresh_all(cx);
                Ok(())
            }
            Err(e) => {
                let err_str = e.to_string();
                self.error = Some(err_str.clone());
                cx.notify();
                Err(err_str)
            }
        }
    }

    pub fn pull(&mut self, cx: &mut Context<Self>) -> Result<String, String> {
        self.is_syncing = true;
        cx.notify();
        let res = restclient_core::git::git_pull(&self.repo_path);
        self.is_syncing = false;
        match res {
            Ok(msg) => {
                self.error = None;
                self.refresh_all(cx);
                Ok(msg)
            }
            Err(e) => {
                let err_str = e.to_string();
                self.error = Some(err_str.clone());
                cx.notify();
                Err(err_str)
            }
        }
    }

    pub fn fetch_in_background(&mut self, cx: &mut Context<Self>) {
        let ws_path = self.repo_path.clone();
        cx.spawn(async move |this, cx| {
            let res = cx.background_executor().spawn(async move {
                restclient_core::git::git_fetch(&ws_path)
            }).await;
            if res.is_ok() {
                if let Some(this) = this.upgrade() {
                    this.update(cx, |git, cx| {
                        git.refresh_status(cx);
                    });
                }
            }
        }).detach();
    }

    pub fn push(&mut self, cx: &mut Context<Self>) -> Result<String, String> {
        match restclient_core::git::git_push(&self.repo_path) {
            Ok(msg) => {
                self.error = None;
                self.refresh_status(cx);
                Ok(msg)
            }
            Err(e) => {
                let err_str = e.to_string();
                self.error = Some(err_str.clone());
                cx.notify();
                Err(err_str)
            }
        }
    }

    pub fn init_repo(&mut self, cx: &mut Context<Self>) -> Result<(), String> {
        match restclient_core::git::git_init(&self.repo_path) {
            Ok(_) => {
                self.error = None;
                self.refresh_all(cx);
                Ok(())
            }
            Err(e) => {
                let err_str = e.to_string();
                self.error = Some(err_str.clone());
                cx.notify();
                Err(err_str)
            }
        }
    }
}

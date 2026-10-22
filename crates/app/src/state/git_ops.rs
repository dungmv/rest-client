use super::GitState;
use crate::state::toast_state::ToastState;
use gpui::{Context, Entity};

impl GitState {
    pub fn stage_file(&mut self, path: &str, cx: &mut Context<Self>) {
        let ws_path = self.repo_path.clone();
        let p = path.to_string();
        cx.spawn(async move |this, cx| {
            let _ = cx
                .background_executor()
                .spawn(async move { restclient_core::git::git_stage_file(&ws_path, &p) })
                .await;
            if let Some(this) = this.upgrade() {
                this.update(cx, |git, cx| {
                    git.refresh_status(cx);
                });
            }
        })
        .detach();
    }

    pub fn unstage_file(&mut self, path: &str, cx: &mut Context<Self>) {
        let ws_path = self.repo_path.clone();
        let p = path.to_string();
        cx.spawn(async move |this, cx| {
            let _ = cx
                .background_executor()
                .spawn(async move { restclient_core::git::git_unstage_file(&ws_path, &p) })
                .await;
            if let Some(this) = this.upgrade() {
                this.update(cx, |git, cx| {
                    git.refresh_status(cx);
                });
            }
        })
        .detach();
    }

    pub fn stage_all(&mut self, cx: &mut Context<Self>) {
        let ws_path = self.repo_path.clone();
        cx.spawn(async move |this, cx| {
            let _ = cx
                .background_executor()
                .spawn(async move { restclient_core::git::git_stage_all(&ws_path) })
                .await;
            if let Some(this) = this.upgrade() {
                this.update(cx, |git, cx| {
                    git.refresh_status(cx);
                });
            }
        })
        .detach();
    }

    pub fn unstage_all(&mut self, cx: &mut Context<Self>) {
        let ws_path = self.repo_path.clone();
        cx.spawn(async move |this, cx| {
            let _ = cx
                .background_executor()
                .spawn(async move { restclient_core::git::git_unstage_all(&ws_path) })
                .await;
            if let Some(this) = this.upgrade() {
                this.update(cx, |git, cx| {
                    git.refresh_status(cx);
                });
            }
        })
        .detach();
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
                self.selected_unstaged_files = st
                    .unstaged_files
                    .iter()
                    .chain(&st.untracked_files)
                    .map(|f| f.path.clone())
                    .collect();
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
                self.selected_staged_files =
                    st.staged_files.iter().map(|f| f.path.clone()).collect();
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
            let _ = cx
                .background_executor()
                .spawn(async move {
                    let str_paths: Vec<&str> = paths.iter().map(|s| s.as_str()).collect();
                    restclient_core::git::git_stage_files(&ws_path, &str_paths)
                })
                .await;
            if let Some(this) = this.upgrade() {
                this.update(cx, |git, cx| {
                    git.refresh_status(cx);
                });
            }
        })
        .detach();
    }

    pub fn unstage_selected(&mut self, cx: &mut Context<Self>) {
        let paths: Vec<String> = self.selected_staged_files.iter().cloned().collect();
        if paths.is_empty() {
            return;
        }
        self.selected_staged_files.clear();
        let ws_path = self.repo_path.clone();
        cx.spawn(async move |this, cx| {
            let _ = cx
                .background_executor()
                .spawn(async move {
                    let str_paths: Vec<&str> = paths.iter().map(|s| s.as_str()).collect();
                    restclient_core::git::git_unstage_files(&ws_path, &str_paths)
                })
                .await;
            if let Some(this) = this.upgrade() {
                this.update(cx, |git, cx| {
                    git.refresh_status(cx);
                });
            }
        })
        .detach();
    }

    pub fn discard_file(&mut self, path: &str, cx: &mut Context<Self>) {
        let ws_path = self.repo_path.clone();
        let p = path.to_string();
        let p_task = p.clone();
        cx.spawn(async move |this, cx| {
            let _ = cx
                .background_executor()
                .spawn(async move { restclient_core::git::git_discard_file(&ws_path, &p_task) })
                .await;
            if let Some(this) = this.upgrade() {
                this.update(cx, |git, cx| {
                    if let Some((sel, _)) = &git.selected_file {
                        if sel == &p {
                            git.selected_file = None;
                            git.diff_content = None;
                        }
                    }
                    git.refresh_status(cx);
                });
            }
        })
        .detach();
    }

    pub fn commit(&mut self, message: &str, cx: &mut Context<Self>) {
        if self.loading {
            return;
        }
        self.loading = true;
        self.error = None;
        cx.notify();

        let ws_path = self.repo_path.clone();
        let msg = message.to_string();
        cx.spawn(async move |this, cx| {
            let res = cx
                .background_executor()
                .spawn(async move { restclient_core::git::git_commit(&ws_path, &msg) })
                .await;
            if let Some(this) = this.upgrade() {
                this.update(cx, |git, cx| {
                    git.loading = false;
                    match res {
                        Ok(_) => {
                            git.commit_message.clear();
                            git.error = None;
                            git.selected_file = None;
                            git.diff_content = None;
                            git.close_modal(cx);
                            git.refresh_all(cx);
                        }
                        Err(e) => {
                            git.error = Some(e.to_string());
                            cx.notify();
                        }
                    }
                });
            }
        })
        .detach();
    }

    pub fn checkout(
        &mut self,
        branch: &str,
        toast: Option<Entity<ToastState>>,
        cx: &mut Context<Self>,
    ) {
        if self.loading {
            return;
        }
        self.loading = true;
        self.error = None;
        cx.notify();

        let ws_path = self.repo_path.clone();
        let b = branch.to_string();
        let b_task = b.clone();
        cx.spawn(async move |this, cx| {
            let res = cx
                .background_executor()
                .spawn(async move { restclient_core::git::git_checkout(&ws_path, &b_task) })
                .await;
            if let Some(this) = this.upgrade() {
                this.update(cx, |git, cx| {
                    git.loading = false;
                    match res {
                        Ok(_) => {
                            git.branch = Some(b);
                            git.error = None;
                            git.refresh_all(cx);
                            if let Some(toast) = &toast {
                                toast.update(cx, |ts, cx| {
                                    ts.push_success(
                                        "Git Checkout",
                                        format!(
                                            "Switched to branch '{}'",
                                            git.branch.as_deref().unwrap_or("")
                                        ),
                                        cx,
                                    );
                                });
                            }
                        }
                        Err(e) => {
                            let err_str = e.to_string();
                            git.error = Some(err_str.clone());
                            if let Some(toast) = &toast {
                                toast.update(cx, |ts, cx| {
                                    ts.push_error("Git Checkout Failed", err_str, cx);
                                });
                            }
                            cx.notify();
                        }
                    }
                });
            }
        })
        .detach();
    }

    pub fn create_branch(&mut self, name: &str, cx: &mut Context<Self>) {
        if self.loading {
            return;
        }
        self.loading = true;
        self.error = None;
        cx.notify();

        let ws_path = self.repo_path.clone();
        let n = name.to_string();
        let n_task = n.clone();
        cx.spawn(async move |this, cx| {
            let res = cx
                .background_executor()
                .spawn(async move { restclient_core::git::git_create_branch(&ws_path, &n_task) })
                .await;
            if let Some(this) = this.upgrade() {
                this.update(cx, |git, cx| {
                    git.loading = false;
                    match res {
                        Ok(_) => {
                            git.branch = Some(n);
                            git.error = None;
                            git.close_modal(cx);
                            git.refresh_all(cx);
                        }
                        Err(e) => {
                            git.error = Some(e.to_string());
                            cx.notify();
                        }
                    }
                });
            }
        })
        .detach();
    }

    pub fn pull(&mut self, toast: Option<Entity<ToastState>>, cx: &mut Context<Self>) {
        if self.is_syncing {
            return;
        }
        self.is_syncing = true;
        self.error = None;
        cx.notify();

        let ws_path = self.repo_path.clone();
        cx.spawn(async move |this, cx| {
            let res = cx
                .background_executor()
                .spawn(async move { restclient_core::git::git_pull(&ws_path) })
                .await;
            if let Some(this) = this.upgrade() {
                this.update(cx, |git, cx| {
                    git.is_syncing = false;
                    match res {
                        Ok(msg) => {
                            git.error = None;
                            git.refresh_all(cx);
                            if let Some(toast) = &toast {
                                let trimmed = msg.trim();
                                if trimmed.contains("Already up to date") {
                                    toast.update(cx, |ts, cx| {
                                        ts.push_info("Git Sync", "Already up to date.", cx);
                                    });
                                } else {
                                    toast.update(cx, |ts, cx| {
                                        ts.push_success(
                                            "Git Pull",
                                            format!("Successfully pulled: {}", trimmed),
                                            cx,
                                        );
                                    });
                                }
                            }
                        }
                        Err(e) => {
                            let err_str = e.to_string();
                            git.error = Some(err_str.clone());
                            if let Some(toast) = &toast {
                                toast.update(cx, |ts, cx| {
                                    ts.push_error("Git Pull Failed", err_str, cx);
                                });
                            }
                            cx.notify();
                        }
                    }
                });
            }
        })
        .detach();
    }

    pub fn fetch_in_background(&mut self, cx: &mut Context<Self>) {
        self.fetch(None, cx);
    }

    pub fn push(&mut self, toast: Option<Entity<ToastState>>, cx: &mut Context<Self>) {
        if self.is_syncing {
            return;
        }
        self.is_syncing = true;
        self.error = None;
        cx.notify();

        let ws_path = self.repo_path.clone();
        cx.spawn(async move |this, cx| {
            let res = cx
                .background_executor()
                .spawn(async move { restclient_core::git::git_push(&ws_path) })
                .await;
            if let Some(this) = this.upgrade() {
                this.update(cx, |git, cx| {
                    git.is_syncing = false;
                    match res {
                        Ok(msg) => {
                            git.error = None;
                            git.refresh_status(cx);
                            if let Some(toast) = &toast {
                                let trimmed = msg.trim();
                                toast.update(cx, |ts, cx| {
                                    ts.push_success(
                                        "Git Push",
                                        if trimmed.is_empty() {
                                            "Pushed commits to remote.".to_string()
                                        } else {
                                            trimmed.to_string()
                                        },
                                        cx,
                                    );
                                });
                            }
                        }
                        Err(e) => {
                            let err_str = e.to_string();
                            git.error = Some(err_str.clone());
                            if let Some(toast) = &toast {
                                toast.update(cx, |ts, cx| {
                                    ts.push_error("Git Push Failed", err_str, cx);
                                });
                            }
                            cx.notify();
                        }
                    }
                });
            }
        })
        .detach();
    }

    pub fn init_repo(&mut self, cx: &mut Context<Self>) {
        if self.loading {
            return;
        }
        self.loading = true;
        self.error = None;
        cx.notify();

        let ws_path = self.repo_path.clone();
        cx.spawn(async move |this, cx| {
            let res = cx
                .background_executor()
                .spawn(async move { restclient_core::git::git_init(&ws_path) })
                .await;
            if let Some(this) = this.upgrade() {
                this.update(cx, |git, cx| {
                    git.loading = false;
                    match res {
                        Ok(_) => {
                            git.error = None;
                            git.refresh_all(cx);
                        }
                        Err(e) => {
                            git.error = Some(e.to_string());
                            cx.notify();
                        }
                    }
                });
            }
        })
        .detach();
    }
}

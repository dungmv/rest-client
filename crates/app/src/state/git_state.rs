use gpui::Context;
use std::collections::HashSet;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitActiveModal {
    Commit,
    History,
    NewBranch,
    Remotes,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GitTab {
    #[default]
    Changes,
    History,
    Branches,
}

#[derive(Debug, Clone)]
pub struct GitState {
    pub repo_path: PathBuf,
    pub branch: Option<String>,
    pub tracked_warning: bool,
    pub show_modal: bool,
    pub active_modal: Option<GitActiveModal>,
    pub tab: GitTab,
    pub status: Option<restclient_core::git::GitStatusResult>,
    pub history: Vec<restclient_core::git::GitCommitInfo>,
    pub branches: Vec<restclient_core::git::GitBranchInfo>,
    pub remotes: Vec<restclient_core::git::GitRemote>,
    pub selected_staged_files: HashSet<String>,
    pub selected_unstaged_files: HashSet<String>,
    pub selected_file: Option<(String, bool)>,
    pub diff_content: Option<String>,
    pub selected_commit: Option<restclient_core::git::GitCommitDetail>,
    pub expanded_history_files: HashSet<String>,
    pub commit_message: String,
    pub error: Option<String>,
    pub loading: bool,
    pub is_syncing: bool,
}

impl Default for GitState {
    fn default() -> Self {
        Self {
            repo_path: PathBuf::new(),
            branch: None,
            tracked_warning: false,
            show_modal: false,
            active_modal: None,
            tab: GitTab::Changes,
            status: None,
            history: Vec::new(),
            branches: Vec::new(),
            remotes: Vec::new(),
            selected_staged_files: HashSet::new(),
            selected_unstaged_files: HashSet::new(),
            selected_file: None,
            diff_content: None,
            selected_commit: None,
            expanded_history_files: HashSet::new(),
            commit_message: String::new(),
            error: None,
            loading: false,
            is_syncing: false,
        }
    }
}

impl GitState {
    pub fn new(repo_path: PathBuf) -> Self {
        Self {
            repo_path,
            ..Default::default()
        }
    }

    pub fn set_repo_path(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        self.repo_path = path;
        self.refresh_all(cx);
    }

    pub fn open_modal(&mut self, modal: GitActiveModal, cx: &mut Context<Self>) {
        self.active_modal = Some(modal);
        self.show_modal = true;
        self.error = None;
        match modal {
            GitActiveModal::Commit => {
                self.selected_unstaged_files.clear();
                self.selected_staged_files.clear();
                self.refresh_status(cx);
            }
            GitActiveModal::History => {
                self.expanded_history_files.clear();
                self.refresh_history(cx);
            }
            GitActiveModal::NewBranch => {
                self.refresh_branches(cx);
            }
            GitActiveModal::Remotes => {
                self.refresh_remotes(cx);
            }
        }
        cx.notify();
    }

    pub fn close_modal(&mut self, cx: &mut Context<Self>) -> bool {
        if self.show_modal || self.active_modal.is_some() {
            self.active_modal = None;
            self.show_modal = false;
            self.error = None;
            self.selected_unstaged_files.clear();
            self.selected_staged_files.clear();
            self.expanded_history_files.clear();
            cx.notify();
            true
        } else {
            false
        }
    }

    pub fn toggle_modal(&mut self, cx: &mut Context<Self>) {
        if self.show_modal || self.active_modal.is_some() {
            self.close_modal(cx);
        } else {
            self.open_modal(GitActiveModal::Commit, cx);
        }
    }

    pub fn set_tab(&mut self, tab: GitTab, cx: &mut Context<Self>) {
        self.tab = tab;
        match tab {
            GitTab::Changes => self.refresh_status(cx),
            GitTab::History => self.refresh_history(cx),
            GitTab::Branches => self.refresh_branches(cx),
        }
        cx.notify();
    }

    pub fn refresh_all(&mut self, cx: &mut Context<Self>) {
        self.refresh_status(cx);
        self.refresh_history(cx);
        self.refresh_branches(cx);
        self.refresh_remotes(cx);
    }

    pub fn refresh_remotes(&mut self, cx: &mut Context<Self>) {
        let ws_path = self.repo_path.clone();
        cx.spawn(async move |this, cx| {
            let remotes = cx.background_executor().spawn(async move {
                restclient_core::git::git_get_remotes(&ws_path).ok()
            }).await;
            if let Some(remotes) = remotes {
                if let Some(this) = this.upgrade() {
                    this.update(cx, |git, cx| {
                        git.remotes = remotes;
                        cx.notify();
                    });
                }
            }
        }).detach();
    }

    pub fn add_remote(&mut self, name: &str, url: &str, cx: &mut Context<Self>) -> Result<(), String> {
        match restclient_core::git::git_add_remote(&self.repo_path, name, url) {
            Ok(_) => {
                self.error = None;
                self.refresh_remotes(cx);
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

    pub fn remove_remote(&mut self, name: &str, cx: &mut Context<Self>) -> Result<(), String> {
        match restclient_core::git::git_remove_remote(&self.repo_path, name) {
            Ok(_) => {
                self.error = None;
                self.refresh_remotes(cx);
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

    pub fn delete_branch(&mut self, name: &str, cx: &mut Context<Self>) -> Result<(), String> {
        match restclient_core::git::git_delete_branch(&self.repo_path, name, false) {
            Ok(_) => {
                self.error = None;
                self.refresh_branches(cx);
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

    pub fn fetch(&mut self, cx: &mut Context<Self>) -> Result<String, String> {
        match restclient_core::git::git_fetch(&self.repo_path) {
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

    pub fn refresh_status(&mut self, cx: &mut Context<Self>) {
        let ws_path = self.repo_path.clone();
        let show_modal = self.show_modal;
        let selected_file = self.selected_file.clone();

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
                } else {
                    st.untracked_files.first().map(|first| (first.path.clone(), false))
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
                let need_branches = this_ent.read_with(cx, |this, _| this.branches.is_empty());
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
                this_ent.update(cx, |git, cx| {
                    if st.is_repo && !st.branch.is_empty() {
                        git.branch = Some(st.branch.clone());
                    }
                    let valid_unstaged: HashSet<String> = st.unstaged_files.iter().chain(&st.untracked_files).map(|f| f.path.clone()).collect();
                    git.selected_unstaged_files.retain(|p| valid_unstaged.contains(p));

                    let valid_staged: HashSet<String> = st.staged_files.iter().map(|f| f.path.clone()).collect();
                    git.selected_staged_files.retain(|p| valid_staged.contains(p));

                    if let Some((file_sel, diff_content)) = diff_result {
                        git.selected_file = file_sel;
                        git.diff_content = diff_content;
                    }

                    if let Some(branches) = branches_res {
                        git.branches = branches;
                    }
                    git.status = Some(st);
                    cx.notify();
                });
            }
        }).detach();
    }

    pub fn load_diff(&mut self, file_path: &str, staged: bool, cx: &mut Context<Self>) {
        let ws_path = self.repo_path.clone();
        let path = file_path.to_string();
        cx.spawn(async move |this, cx| {
            let res = cx.background_executor().spawn(async move {
                restclient_core::git::git_diff(&ws_path, Some(&path), staged)
            }).await;
            if let Some(this) = this.upgrade() {
                this.update(cx, |git, cx| {
                    match res {
                        Ok(d) => git.diff_content = Some(d),
                        Err(e) => git.diff_content = Some(format!("Error loading diff: {}", e)),
                    }
                    cx.notify();
                });
            }
        }).detach();
    }

    pub fn refresh_history(&mut self, cx: &mut Context<Self>) {
        let ws_path = self.repo_path.clone();
        let needs_first = self.selected_commit.is_none();
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
                    this.update(cx, |git, cx| {
                        if let Some(detail) = first_detail {
                            git.selected_commit = Some(detail);
                        }
                        git.history = commits;
                        cx.notify();
                    });
                }
            }
        }).detach();
    }

    pub fn refresh_branches(&mut self, cx: &mut Context<Self>) {
        let ws_path = self.repo_path.clone();
        cx.spawn(async move |this, cx| {
            let branches = cx.background_executor().spawn(async move {
                restclient_core::git::git_branches(&ws_path).ok()
            }).await;
            if let Some(branches) = branches {
                if let Some(this) = this.upgrade() {
                    this.update(cx, |git, cx| {
                        git.branches = branches;
                        cx.notify();
                    });
                }
            }
        }).detach();
    }

    pub fn select_file(&mut self, path: String, staged: bool, cx: &mut Context<Self>) {
        self.selected_file = Some((path.clone(), staged));
        self.load_diff(&path, staged, cx);
    }

    pub fn select_commit(&mut self, hash: &str, cx: &mut Context<Self>) {
        self.expanded_history_files.clear();
        let ws_path = self.repo_path.clone();
        let h = hash.to_string();
        cx.spawn(async move |this, cx| {
            let detail = cx.background_executor().spawn(async move {
                restclient_core::git::git_commit_detail(&ws_path, &h).ok()
            }).await;
            if let Some(detail) = detail {
                if let Some(this) = this.upgrade() {
                    this.update(cx, |git, cx| {
                        git.selected_commit = Some(detail);
                        cx.notify();
                    });
                }
            }
        }).detach();
    }

    pub fn toggle_history_file_expansion(&mut self, file_path: &str, cx: &mut Context<Self>) {
        if self.expanded_history_files.contains(file_path) {
            self.expanded_history_files.remove(file_path);
        } else {
            self.expanded_history_files.insert(file_path.to_string());
        }
        cx.notify();
    }
}

use super::request_node::SidebarReqItem;
use restclient_core::models::Folder;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarGitStatus {
    New,      // File mới (Untracked '??', Staged Added 'A') -> Green
    Modified, // File có thay đổi ('M', 'R', 'C', '!', or unsaved) -> Yellow
    Clean,    // Đã commit -> Normal (White)
}

pub fn calculate_request_git_status(
    req_id: &str,
    git_status: Option<&restclient_core::git::GitStatusResult>,
    is_unsaved: bool,
) -> SidebarGitStatus {
    if is_unsaved {
        return SidebarGitStatus::Modified;
    }

    let Some(git) = git_status else {
        return SidebarGitStatus::Clean;
    };

    if !git.is_repo {
        return SidebarGitStatus::Clean;
    }

    let norm_req = req_id.replace('\\', "/");
    let coll_rel = format!("collections/{}", norm_req);

    let matches = |p: &str| -> bool {
        let p = p.replace('\\', "/");
        p == norm_req
            || p == coll_rel
            || p.ends_with(&format!("/{}", norm_req))
            || p.ends_with(&format!("/{}", coll_rel))
    };

    // 1. Untracked (new file)
    if git.untracked_files.iter().any(|item| matches(&item.path)) {
        return SidebarGitStatus::New;
    }

    // 2. Staged
    if let Some(item) = git.staged_files.iter().find(|item| matches(&item.path)) {
        return match item.status {
            restclient_core::git::GitFileStatus::Added => SidebarGitStatus::New,
            _ => SidebarGitStatus::Modified,
        };
    }

    // 3. Unstaged (working tree changes)
    if git.unstaged_files.iter().any(|item| matches(&item.path)) {
        return SidebarGitStatus::Modified;
    }

    SidebarGitStatus::Clean
}

pub fn calculate_folder_git_status(
    folder_id: &str,
    folders: &[Folder],
    requests: &[SidebarReqItem],
    git_status: Option<&restclient_core::git::GitStatusResult>,
    unsaved_requests: &std::collections::HashSet<String>,
) -> SidebarGitStatus {
    let mut descendant_folders = std::collections::HashSet::new();
    descendant_folders.insert(folder_id.to_string());

    let mut changed = true;
    while changed {
        changed = false;
        for f in folders {
            if let Some(ref pid) = f.parent_id {
                if descendant_folders.contains(pid) && descendant_folders.insert(f.id.clone()) {
                    changed = true;
                }
            }
        }
    }

    let mut has_new = false;
    for req in requests {
        if let Some(ref fid) = req.folder_id {
            if descendant_folders.contains(fid) {
                let st = calculate_request_git_status(&req.id, git_status, unsaved_requests.contains(&req.id));
                if st == SidebarGitStatus::Modified {
                    return SidebarGitStatus::Modified;
                }
                if st == SidebarGitStatus::New {
                    has_new = true;
                }
            }
        }
    }

    if has_new {
        SidebarGitStatus::New
    } else {
        SidebarGitStatus::Clean
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::sidebar::request_node::SidebarExampleItem;
    use restclient_core::git::{GitFileStatus, GitStatusItem, GitStatusResult};
    use restclient_core::models::HttpMethod;
    use std::collections::HashSet;

    #[test]
    fn test_calculate_request_git_status_untracked_is_new() {
        let status = GitStatusResult {
            is_repo: true,
            untracked_files: vec![GitStatusItem {
                path: "collections/users/get.yaml".to_string(),
                status: GitFileStatus::Untracked,
                staged: false,
            }],
            ..Default::default()
        };

        let result = calculate_request_git_status("users/get.yaml", Some(&status), false);
        assert_eq!(result, SidebarGitStatus::New);
    }

    #[test]
    fn test_calculate_request_git_status_staged_added_is_new() {
        let status = GitStatusResult {
            is_repo: true,
            staged_files: vec![GitStatusItem {
                path: "collections/login.yaml".to_string(),
                status: GitFileStatus::Added,
                staged: true,
            }],
            ..Default::default()
        };

        let result = calculate_request_git_status("login.yaml", Some(&status), false);
        assert_eq!(result, SidebarGitStatus::New);
    }

    #[test]
    fn test_calculate_request_git_status_unstaged_modified_is_modified() {
        let status = GitStatusResult {
            is_repo: true,
            unstaged_files: vec![GitStatusItem {
                path: "collections/auth/login.yaml".to_string(),
                status: GitFileStatus::Modified,
                staged: false,
            }],
            ..Default::default()
        };

        let result = calculate_request_git_status("auth/login.yaml", Some(&status), false);
        assert_eq!(result, SidebarGitStatus::Modified);
    }

    #[test]
    fn test_calculate_request_git_status_staged_modified_is_modified() {
        let status = GitStatusResult {
            is_repo: true,
            staged_files: vec![GitStatusItem {
                path: "collections/checkout.yaml".to_string(),
                status: GitFileStatus::Modified,
                staged: true,
            }],
            ..Default::default()
        };

        let result = calculate_request_git_status("checkout.yaml", Some(&status), false);
        assert_eq!(result, SidebarGitStatus::Modified);
    }

    #[test]
    fn test_calculate_request_git_status_unsaved_is_modified() {
        let status = GitStatusResult {
            is_repo: true,
            ..Default::default()
        };

        let result = calculate_request_git_status("req1.yaml", Some(&status), true);
        assert_eq!(result, SidebarGitStatus::Modified);
    }

    #[test]
    fn test_calculate_request_git_status_clean_is_clean() {
        let status = GitStatusResult {
            is_repo: true,
            staged_files: vec![GitStatusItem {
                path: "collections/other.yaml".to_string(),
                status: GitFileStatus::Modified,
                staged: true,
            }],
            ..Default::default()
        };

        let result = calculate_request_git_status("req_clean.yaml", Some(&status), false);
        assert_eq!(result, SidebarGitStatus::Clean);
    }

    #[test]
    fn test_calculate_request_git_status_non_repo_is_clean() {
        let status = GitStatusResult {
            is_repo: false,
            ..Default::default()
        };

        let result = calculate_request_git_status("req.yaml", Some(&status), false);
        assert_eq!(result, SidebarGitStatus::Clean);
    }

    #[test]
    fn test_calculate_folder_git_status() {
        let folders = vec![
            Folder {
                id: "auth".to_string(),
                workspace_id: "ws".to_string(),
                parent_id: None,
                name: "Auth".to_string(),
                sort_order: 0,
                headers: Vec::new(),
                auth: restclient_core::models::Auth::Inherit,
            },
            Folder {
                id: "auth/sub".to_string(),
                workspace_id: "ws".to_string(),
                parent_id: Some("auth".to_string()),
                name: "Sub".to_string(),
                sort_order: 0,
                headers: Vec::new(),
                auth: restclient_core::models::Auth::Inherit,
            },
        ];

        let requests = vec![
            SidebarReqItem {
                id: "auth/sub/token.yaml".to_string(),
                folder_id: Some("auth/sub".to_string()),
                name: "Token".to_string(),
                display_name: "Token".to_string(),
                method: HttpMethod::Post,
                url: "".to_string(),
                last_status_code: None,
                examples: vec![],
            },
            SidebarReqItem {
                id: "auth/login.yaml".to_string(),
                folder_id: Some("auth".to_string()),
                name: "Login".to_string(),
                display_name: "Login".to_string(),
                method: HttpMethod::Post,
                url: "".to_string(),
                last_status_code: None,
                examples: vec![],
            },
        ];

        let unsaved = HashSet::new();

        // 1. When child is modified, parent folder is modified
        let status_mod = GitStatusResult {
            is_repo: true,
            unstaged_files: vec![GitStatusItem {
                path: "collections/auth/sub/token.yaml".to_string(),
                status: GitFileStatus::Modified,
                staged: false,
            }],
            ..Default::default()
        };
        assert_eq!(
            calculate_folder_git_status("auth", &folders, &requests, Some(&status_mod), &unsaved),
            SidebarGitStatus::Modified
        );

        // 2. When child is new, parent folder is new
        let status_new = GitStatusResult {
            is_repo: true,
            untracked_files: vec![GitStatusItem {
                path: "collections/auth/login.yaml".to_string(),
                status: GitFileStatus::Untracked,
                staged: false,
            }],
            ..Default::default()
        };
        assert_eq!(
            calculate_folder_git_status("auth", &folders, &requests, Some(&status_new), &unsaved),
            SidebarGitStatus::New
        );

        // 3. When all clean, folder is clean
        let status_clean = GitStatusResult {
            is_repo: true,
            ..Default::default()
        };
        assert_eq!(
            calculate_folder_git_status("auth", &folders, &requests, Some(&status_clean), &unsaved),
            SidebarGitStatus::Clean
        );
    }
}

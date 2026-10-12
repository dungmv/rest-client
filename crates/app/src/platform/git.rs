use std::path::Path;

pub fn detect_git_branch(start_dir: Option<&Path>) -> Option<String> {
    let mut current = match start_dir {
        Some(p) => p.to_path_buf(),
        None => std::env::current_dir().ok()?,
    };

    loop {
        let head_path = current.join(".git").join("HEAD");
        if head_path.is_file() {
            if let Ok(content) = std::fs::read_to_string(&head_path) {
                let trimmed = content.trim();
                if let Some(branch) = trimmed.strip_prefix("ref: refs/heads/") {
                    return Some(branch.to_string());
                } else if !trimmed.is_empty() {
                    // Detached HEAD: first 7 characters of hash
                    return Some(trimmed.chars().take(7).collect());
                }
            }
        }

        if !current.pop() {
            break;
        }
    }

    None
}

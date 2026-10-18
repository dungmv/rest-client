use anyhow::Result;
use serde::Serialize;
use std::path::Path;

/// Serialize value to YAML deterministically:
/// - 2-space indent
/// - LF line endings (no CRLF even on Windows)
/// - Trailing newline
/// - No document start prefix (`---`)
pub fn to_yaml_string<T: Serialize>(value: &T) -> Result<String> {
    let mut yaml = serde_yaml::to_string(value)?;
    // Strip document prefix if serde_yaml added it
    if let Some(stripped) = yaml.strip_prefix("---\n") {
        yaml = stripped.to_string();
    } else if let Some(stripped) = yaml.strip_prefix("---") {
        yaml = stripped.trim_start_matches('\n').to_string();
    }
    // Normalize any CRLF to LF
    yaml = yaml.replace("\r\n", "\n");
    if !yaml.ends_with('\n') {
        yaml.push('\n');
    }
    Ok(yaml)
}

/// Windows sharing-violation retry helper: retries 5x with delay (50ms, 100ms, ...).
pub fn with_retry<T, F: FnMut() -> std::io::Result<T>>(mut op: F) -> std::io::Result<T> {
    let mut attempts = 0;
    loop {
        match op() {
            Ok(val) => return Ok(val),
            Err(e) => {
                attempts += 1;
                if attempts >= 5 {
                    return Err(e);
                }
                std::thread::sleep(std::time::Duration::from_millis(50 * attempts as u64));
            }
        }
    }
}

/// Atomic write: on Windows writes directly to avoid sharing violations with file watchers and retry sleeps.
/// On other platforms, writes to temp file in the same directory, then renames.
pub fn atomic_write(path: &Path, content: &[u8]) -> std::io::Result<()> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(parent)?;

    #[cfg(windows)]
    {
        std::fs::write(path, content)
    }

    #[cfg(not(windows))]
    {
        let temp_name = format!(
            ".tmp_{}_{}",
            uuid::Uuid::new_v4().simple(),
            path.file_name().and_then(|n| n.to_str()).unwrap_or("file")
        );
        let temp_path = parent.join(temp_name);

        std::fs::write(&temp_path, content)?;

        let rename_res = with_retry(|| std::fs::rename(&temp_path, path));
        if rename_res.is_err() {
            let _ = std::fs::remove_file(&temp_path);
        }
        rename_res
    }
}

/// Atomic write string (ensuring LF line endings).
pub fn atomic_write_str(path: &Path, content: &str) -> std::io::Result<()> {
    let normalized = content.replace("\r\n", "\n");
    atomic_write(path, normalized.as_bytes())
}

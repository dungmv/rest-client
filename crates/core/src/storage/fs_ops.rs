use crate::storage::models::{FolderFile, RequestFile};
use crate::storage::serialize::{atomic_write_str, to_yaml_string, with_retry};
use crate::storage::slug::{loc_path_for_yaml, slugify, unique_slug};
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

/// Normalize path separators to forward slash for clean git-friendly relative paths
pub fn path_to_forward_slash(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// Create a new request in `collections_dir` (inside `folder_rel` if specified):
/// Writes `<slug>.yaml` atomically.
pub fn create_request(
    collections_dir: &Path,
    folder_rel: Option<&str>,
    name: &str,
    mut req: RequestFile,
) -> Result<(String, PathBuf)> {
    let target_dir = match folder_rel {
        Some(f) if !f.trim().is_empty() => collections_dir.join(f),
        _ => collections_dir.to_path_buf(),
    };
    std::fs::create_dir_all(&target_dir)?;

    let base_slug = slugify(name);
    let slug = unique_slug(&target_dir, &base_slug, "yaml");
    let yaml_file_name = format!("{}.yaml", slug);
    let yaml_path = target_dir.join(&yaml_file_name);

    req.name = name.to_string();
    let content = to_yaml_string(&req)?;
    atomic_write_str(&yaml_path, &content)
        .with_context(|| format!("Failed to atomically write request at {:?}", yaml_path))?;

    let rel_path = yaml_path
        .strip_prefix(collections_dir)
        .unwrap_or(&yaml_path)
        .to_path_buf();

    Ok((slug, rel_path))
}

/// Delete a request: acts on `<slug>.yaml` FIRST (must succeed),
/// then on `<slug>.loc` (best-effort, ignore errors).
pub fn delete_request(collections_dir: &Path, rel_path: &Path) -> Result<()> {
    let yaml_path = collections_dir.join(rel_path);
    if yaml_path.exists() {
        with_retry(|| std::fs::remove_file(&yaml_path))
            .with_context(|| format!("Failed to delete request file at {:?}", yaml_path))?;
    }

    let loc_path = loc_path_for_yaml(&yaml_path);
    if loc_path.exists() {
        let _ = with_retry(|| std::fs::remove_file(&loc_path));
    }

    Ok(())
}

/// Rename a request:
/// Renames `<slug>.yaml` and matching `<slug>.loc` atomically, updating display name in yaml.
pub fn rename_request(
    collections_dir: &Path,
    rel_path: &Path,
    new_name: &str,
) -> Result<PathBuf> {
    let old_yaml_path = collections_dir.join(rel_path);
    if !old_yaml_path.exists() {
        anyhow::bail!("Request file {:?} does not exist", old_yaml_path);
    }

    let parent_dir = old_yaml_path
        .parent()
        .unwrap_or(collections_dir)
        .to_path_buf();

    let old_stem = old_yaml_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("");

    let base_slug = slugify(new_name);
    let new_slug = if base_slug == old_stem {
        base_slug
    } else {
        unique_slug(&parent_dir, &base_slug, "yaml")
    };
    let new_yaml_path = parent_dir.join(format!("{}.yaml", new_slug));
    let new_loc_path = parent_dir.join(format!("{}.loc", new_slug));
    let old_loc_path = loc_path_for_yaml(&old_yaml_path);

    // Read existing yaml content, update name, and write to new path
    let existing_raw = std::fs::read_to_string(&old_yaml_path)?;
    let mut req: RequestFile = serde_yaml::from_str(&existing_raw)
        .unwrap_or_else(|_| RequestFile {
            name: new_name.to_string(),
            ..Default::default()
        });
    req.name = new_name.to_string();
    let updated_content = to_yaml_string(&req)?;

    if old_yaml_path != new_yaml_path {
        // Write new yaml atomically
        atomic_write_str(&new_yaml_path, &updated_content)?;
        // Remove old yaml
        let _ = with_retry(|| std::fs::remove_file(&old_yaml_path));

        // Rename .loc best-effort
        if old_loc_path.exists() {
            let rename_res = with_retry(|| {
                if new_loc_path.exists() {
                    let _ = std::fs::remove_file(&new_loc_path);
                }
                std::fs::rename(&old_loc_path, &new_loc_path)
            });
            if rename_res.is_err()
                && old_loc_path.exists()
                && std::fs::copy(&old_loc_path, &new_loc_path).is_ok()
            {
                let _ = with_retry(|| std::fs::remove_file(&old_loc_path));
            }
        }

        // Update _folder.yaml in parent_dir if present
        let folder_meta_path = parent_dir.join("_folder.yaml");
        if folder_meta_path.exists() {
            if let Ok(raw) = std::fs::read_to_string(&folder_meta_path) {
                if let Ok(mut meta) = serde_yaml::from_str::<FolderFile>(&raw) {
                    let mut changed = false;
                    for stem in &mut meta.order {
                        if stem == old_stem {
                            *stem = new_slug.clone();
                            changed = true;
                        }
                    }
                    if changed {
                        if let Ok(content) = to_yaml_string(&meta) {
                            let _ = atomic_write_str(&folder_meta_path, &content);
                        }
                    }
                }
            }
        }
    } else {
        // Same slug, just write updated content
        atomic_write_str(&new_yaml_path, &updated_content)?;
    }

    let new_rel = new_yaml_path
        .strip_prefix(collections_dir)
        .unwrap_or(&new_yaml_path)
        .to_path_buf();
    Ok(new_rel)
}

/// Move a request into another folder (or root if None):
/// Moves `<slug>.yaml` first (resolving collisions in destination),
/// then moves matching `<slug>.loc` (best-effort).
pub fn move_request(
    collections_dir: &Path,
    rel_path: &Path,
    target_folder: Option<&str>,
) -> Result<PathBuf> {
    let old_yaml_path = collections_dir.join(rel_path);
    if !old_yaml_path.exists() {
        anyhow::bail!("Request file {:?} does not exist", old_yaml_path);
    }

    let target_dir = match target_folder {
        Some(f) if !f.trim().is_empty() => collections_dir.join(f),
        _ => collections_dir.to_path_buf(),
    };
    std::fs::create_dir_all(&target_dir)?;

    let old_stem = old_yaml_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("request");
    let new_slug = unique_slug(&target_dir, old_stem, "yaml");
    let new_yaml_path = target_dir.join(format!("{}.yaml", new_slug));
    let new_loc_path = target_dir.join(format!("{}.loc", new_slug));
    let old_loc_path = loc_path_for_yaml(&old_yaml_path);

    // Atomic move of yaml
    with_retry(|| std::fs::rename(&old_yaml_path, &new_yaml_path))
        .with_context(|| format!("Failed to move request to {:?}", new_yaml_path))?;

    // Best-effort move of loc
    if old_loc_path.exists() {
        let rename_res = with_retry(|| {
            if new_loc_path.exists() {
                let _ = std::fs::remove_file(&new_loc_path);
            }
            std::fs::rename(&old_loc_path, &new_loc_path)
        });
        if rename_res.is_err()
            && old_loc_path.exists()
            && std::fs::copy(&old_loc_path, &new_loc_path).is_ok()
        {
            let _ = with_retry(|| std::fs::remove_file(&old_loc_path));
        }
    }

    let new_rel = new_yaml_path
        .strip_prefix(collections_dir)
        .unwrap_or(&new_yaml_path)
        .to_path_buf();
    Ok(new_rel)
}

/// Create a group (folder) under `collections_dir`:
/// Creates directory and writes `_folder.yaml`.
pub fn create_group(
    collections_dir: &Path,
    parent_folder: Option<&str>,
    name: &str,
) -> Result<(String, PathBuf)> {
    let parent_dir = match parent_folder {
        Some(p) if !p.trim().is_empty() => collections_dir.join(p),
        _ => collections_dir.to_path_buf(),
    };
    std::fs::create_dir_all(&parent_dir)?;

    let base_slug = slugify(name);
    // Find unique directory name
    let mut dir_name = base_slug.clone();
    let mut counter = 2;
    while parent_dir.join(&dir_name).exists() {
        dir_name = format!("{}-{}", base_slug, counter);
        counter += 1;
    }

    let group_dir = parent_dir.join(&dir_name);
    std::fs::create_dir_all(&group_dir)?;

    let folder_meta = FolderFile {
        name: name.to_string(),
        order: Vec::new(),
        headers: Vec::new(),
        auth: crate::storage::models::RequestAuth::Inherit,
        extra: std::collections::BTreeMap::new(),
    };
    let content = to_yaml_string(&folder_meta)?;
    atomic_write_str(&group_dir.join("_folder.yaml"), &content)?;

    let rel_path = group_dir
        .strip_prefix(collections_dir)
        .unwrap_or(&group_dir)
        .to_path_buf();

    Ok((dir_name, rel_path))
}

/// Delete a group: single filesystem operation on the directory.
/// All child requests and .loc files are removed.
pub fn delete_group(collections_dir: &Path, folder_rel: &str) -> Result<()> {
    let group_dir = collections_dir.join(folder_rel);
    if group_dir.exists() {
        with_retry(|| std::fs::remove_dir_all(&group_dir))
            .with_context(|| format!("Failed to delete group at {:?}", group_dir))?;
    }
    Ok(())
}

/// Rename a group: updates display name in `_folder.yaml` and renames directory.
pub fn rename_group(
    collections_dir: &Path,
    folder_rel: &str,
    new_name: &str,
) -> Result<PathBuf> {
    let old_group_dir = collections_dir.join(folder_rel);
    if !old_group_dir.exists() {
        anyhow::bail!("Group {:?} does not exist", old_group_dir);
    }

    let parent_dir = old_group_dir
        .parent()
        .unwrap_or(collections_dir)
        .to_path_buf();

    let base_slug = slugify(new_name);
    let mut new_dir_name = base_slug.clone();
    let mut counter = 2;
    while parent_dir.join(&new_dir_name).exists() && parent_dir.join(&new_dir_name) != old_group_dir {
        new_dir_name = format!("{}-{}", base_slug, counter);
        counter += 1;
    }
    let new_group_dir = parent_dir.join(&new_dir_name);

    // Update _folder.yaml
    let meta_file = old_group_dir.join("_folder.yaml");
    let mut meta = if meta_file.exists() {
        let raw = std::fs::read_to_string(&meta_file)?;
        serde_yaml::from_str::<FolderFile>(&raw).unwrap_or_default()
    } else {
        FolderFile::default()
    };
    meta.name = new_name.to_string();
    let meta_yaml = to_yaml_string(&meta)?;
    atomic_write_str(&meta_file, &meta_yaml)?;

    if old_group_dir != new_group_dir {
        with_retry(|| std::fs::rename(&old_group_dir, &new_group_dir))
            .with_context(|| format!("Failed to rename group dir to {:?}", new_group_dir))?;
    }

    let new_rel = new_group_dir
        .strip_prefix(collections_dir)
        .unwrap_or(&new_group_dir)
        .to_path_buf();
    Ok(new_rel)
}

/// Move a group into a target parent directory.
pub fn move_group(
    collections_dir: &Path,
    folder_rel: &str,
    target_parent_rel: Option<&str>,
) -> Result<PathBuf> {
    let old_group_dir = collections_dir.join(folder_rel);
    if !old_group_dir.exists() {
        anyhow::bail!("Group {:?} does not exist", old_group_dir);
    }

    let target_parent = match target_parent_rel {
        Some(p) if !p.trim().is_empty() => collections_dir.join(p),
        _ => collections_dir.to_path_buf(),
    };
    std::fs::create_dir_all(&target_parent)?;

    let dir_name = old_group_dir
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("folder");

    let mut dest_name = dir_name.to_string();
    let mut counter = 2;
    while target_parent.join(&dest_name).exists() {
        dest_name = format!("{}-{}", dir_name, counter);
        counter += 1;
    }
    let new_group_dir = target_parent.join(&dest_name);

    with_retry(|| std::fs::rename(&old_group_dir, &new_group_dir))
        .with_context(|| format!("Failed to move group to {:?}", new_group_dir))?;

    let new_rel = new_group_dir
        .strip_prefix(collections_dir)
        .unwrap_or(&new_group_dir)
        .to_path_buf();
    Ok(new_rel)
}

/// Reorder requests in a folder by writing the stems to `_folder.yaml`.
pub fn reorder_folder(
    collections_dir: &Path,
    folder_rel: Option<&str>,
    order: &[String],
) -> Result<()> {
    let dir = match folder_rel {
        Some(f) if !f.trim().is_empty() => collections_dir.join(f),
        _ => collections_dir.to_path_buf(),
    };
    std::fs::create_dir_all(&dir)?;

    let meta_file = dir.join("_folder.yaml");
    let mut meta = if meta_file.exists() {
        let raw = std::fs::read_to_string(&meta_file)?;
        serde_yaml::from_str::<FolderFile>(&raw).unwrap_or_default()
    } else {
        let name = dir
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("Collection")
            .to_string();
        FolderFile {
            name,
            order: Vec::new(),
            headers: Vec::new(),
            auth: crate::storage::models::RequestAuth::Inherit,
            extra: std::collections::BTreeMap::new(),
        }
    };

    meta.order = order.to_vec();
    let yaml = to_yaml_string(&meta)?;
    atomic_write_str(&meta_file, &yaml)?;
    Ok(())
}

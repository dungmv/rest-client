use anyhow::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Trait defining storage operations for a workspace.
pub trait WorkspaceStore: Send + Sync {
    fn workspace_root(&self) -> &Path;
}

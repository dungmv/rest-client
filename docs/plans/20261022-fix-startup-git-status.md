# Fix: Footer Git Status Showing "No Git" on Startup

## 1. Goal
Fix the bug where Zest's sidebar footer displays "No Git" upon application launch even when the current workspace directory is a valid Git repository.

## 2. Root Cause Analysis
1. **Missing Initial Refresh**: In `crates/app/src/state/app_state.rs`, `GitState::new(ws_dir)` was instantiated with `status: None` and `branch: None`. Neither `AppState::new` nor `AppView::new` ever called `git.refresh_status(cx)` on startup.
2. **Fallback to "No Git" on `None`**: In `crates/app/src/components/sidebar/footer.rs`, `is_repo` is evaluated as `git_status.map(|s| s.is_repo).unwrap_or(false)`. Since `status` was `None`, `is_repo` evaluated to `false`, rendering `"No Git"` permanently until a git modal was manually opened.
3. **Workspace Switching Desync**: In `crates/app/src/app/subscriptions.rs`, when switching workspaces (`current_ws_path != last_workspace_path`), `git_state` was not informed of the new workspace path via `set_repo_path`.
4. **Git Initial Branch Output Parsing**: In `crates/core/src/git/mod.rs`, `git status --porcelain=v1 -b` on a repository with no commits outputs `## No commits yet on <branch>`, which was incorrectly parsed as branch name `"No"`.

## 3. Implementation Plan
1. **Core Git Enhancement (`crates/core/src/git/mod.rs`)**:
   - Add `detect_git_repo_fast(start_path: &Path) -> Option<(PathBuf, Option<String>)>`: synchronously and cheaply inspect `.git` (or submodule/worktree `gitdir`) and `.git/HEAD` without spawning child processes.
   - Improve `git_status` porcelain parsing to handle `No commits yet on <branch>` and `Initial commit on <branch>`.
2. **GitState Fast Initialization & Initial Status (`crates/app/src/state/git_state.rs`)**:
   - In `GitState::new(repo_path)` and `set_repo_path`: use `detect_git_repo_fast` to immediately know whether it is a git repo and what branch it is on, avoiding a "No Git" flash on frame 0.
   - In `refresh_status`: record errors in `git.error` if `git_status` fails instead of silently returning.
3. **App Startup Trigger (`crates/app/src/state/app_state.rs`)**:
   - Call `git.update(cx, |g, cx| g.refresh_status(cx));` during `AppState::new` initialization so status and branches are queried in the background upon launch.
4. **Workspace Switching Synchronization (`crates/app/src/app/subscriptions.rs`)**:
   - Update `git_state.set_repo_path(current_ws_path, cx)` when workspace path changes.
5. **Footer UX Polish (`crates/app/src/components/sidebar/footer.rs`)**:
   - Distinguish `git_status.is_none()` (still checking / loading) from `Some(status)` with `!status.is_repo` (confirmed not a repo).

## 4. Verification Steps
- Run unit tests: `cargo test`
- Run clippy: `cargo clippy --all-targets -- -D warnings`

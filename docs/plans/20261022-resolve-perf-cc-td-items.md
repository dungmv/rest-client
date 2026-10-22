# Plan: Resolve Technical Debt & Performance Items (PERF-005, CC-002, TD-07/TD-005, PERF-007, PERF-008)

## 1. Goal
Eliminate five critical technical debt and performance bottlenecks identified in `docs/technical-debt.md`:
1. **PERF-005**: Eliminate per-write thread spawn in `FsWorkspaceStore::suppress_path` via a single dedicated background timer worker queue.
2. **CC-002**: Batch file watcher `FsEvent` notifications in `PersistenceState` to prevent render and tree reload cascade thrashing.
3. **TD-07 / TD-005**: Eliminate mutex lock poisoning and option unwraps across core and app components, ensuring safe inner recovery.
4. **PERF-007**: Cache formatted responses in `ResponseState` to prevent heavy JSON reparsing and re-serialization during response view mode toggling.
5. **PERF-008**: Eliminate redundant string cloning in `RequestFile::to_item()` by introducing move-semantics `into_item()` and updating tree conversion sites.

---

## 2. Context & Root Cause Analysis

- **PERF-005**: Every time the application writes or updates a file on disk (requests, settings, pins), `FsWorkspaceStore::suppress_path()` invoked `std::thread::spawn` with `sleep(1500ms)`. Bulk actions (importing collections with 50+ requests) spawned dozens of OS threads simultaneously, leading to thread thrashing and high latency.
- **CC-002**: File system changes emitted through `tokio::sync::mpsc::unbounded_channel::<FsEvent>()` triggered immediate calls to `handle_fs_event` one-by-one. Each event notified GPUI (`cx.notify()`) and re-scanned the directory tree or request models, causing cascade re-renders.
- **TD-07 / TD-005**: Mutex locks (`cookie_store`, `suppressed_paths`, etc.) and `Option` unwraps in UI inspectors could panic if a thread encountered an unhandled issue. Poisoned locks must safely yield inner guards via `.unwrap_or_else(|p| p.into_inner())` and Option lookups must use pattern matching.
- **PERF-007**: `ResponseState::refresh_formatted_body()` reparsed the entire response body with `serde_json::from_str` and `serde_json::to_string_pretty` every time `view_mode` switched between Pretty and Raw. For payloads > 500KB, this blocked the UI thread for 10-50ms.
- **PERF-008**: `RequestFile::to_item()` cloned every string field (`name`, `url`, `params`, `headers`, `auth`, `body`, `settings`, `script`, `examples`). During initial scan or tree reload, `RequestNode` owns its `RequestFile`, meaning all clones are redundant.

---

## 3. Step-by-Step Implementation Tasks

### Task 1: PERF-005 — Single Background Cleaner Worker in `FsWorkspaceStore`
- Update `FsWorkspaceStore` in `crates/core/src/storage/fs_store.rs`:
  - Add `suppress_tx: std::sync::mpsc::Sender<PathBuf>` to `FsWorkspaceStore`.
  - In `FsWorkspaceStore::new(...)`, spawn a single long-lived thread (`workspace-suppress-cleaner`).
  - Worker maintains a queue of `(PathBuf, Instant)` deadlines, waiting on `recv_timeout` and removing expired paths from `suppressed_paths`.
  - In `suppress_path(&self, path: &Path)`: insert path into `suppressed_paths` and send path to `suppress_tx`. Zero threads spawned per write.
  - When `FsWorkspaceStore` is dropped, `suppress_tx` disconnects, cleanly stopping the worker thread.
  - Add unit test verifying suppression expires without spawning OS threads.

### Task 2: CC-002 — Batch FsEvent Notification in `PersistenceState`
- Update `crates/app/src/state/persistence_state.rs`:
  - In `start_watcher`, modify the receiver loop to drain available events from `rx` and debounce over a brief window (e.g. 50-100ms) before processing.
  - Add `handle_fs_events_batch(&mut self, events: Vec<FsEvent>, ...)`:
    - If `FsEvent::TreeChanged` is present, execute a single `reload_tree()` and check conflicts for changed requests, superseding individual `RequestChanged` updates.
    - Otherwise, process unique `RequestChanged`, `RequestDeleted`, and `EnvironmentChanged` events in a single batch.
    - Call `cx.notify()` exactly once at the conclusion of the batch.
  - Keep `handle_fs_event` as a convenience forwarding to `handle_fs_events_batch(vec![event])`.

### Task 3: TD-07 / TD-005 — Poison-Resilient Mutexes & Safe Option Extraction
- Audit remaining mutex and option unwraps:
  - In `crates/core/src/cache/mod.rs:81`: replace `self.order.remove(pos).unwrap()` with safe `if let Some(k) = self.order.remove(pos)`.
  - In `crates/app/src/components/response_inspector.rs:567, 649`: replace `.clone().unwrap()` on `current_resp` and `req_id_opt` with safe `let Some(...)` and guard clauses.
  - In `crates/core/src/storage/fs_store.rs`: ensure `suppressed_paths.lock()` recovers gracefully via `unwrap_or_else(|e| e.into_inner())`.
  - Add unit test in `crates/core/src/http/executor.rs` or `storage` confirming recovery from poisoned Mutex locks without panic.

### Task 4: PERF-007 — Formatted Response Body Caching
- Update `crates/app/src/state/response_state.rs`:
  - Enhance `ResponseState` with body cache representation:
    - Store cached raw display text and lines, pretty display text and lines, and JSON/XML detection flags.
    - Track the identity of the cached response (e.g. `cached_response_id: Option<usize>` using `Arc::as_ptr`).
  - In `refresh_formatted_body()`:
    - If current response pointer matches `cached_response_id`, simply switch between cached raw and pretty representations in $O(1)$ time without re-running `serde_json::from_str` or `to_string_pretty`.
    - Only reformat when the response pointer changes or is updated.
  - Add unit test verifying that switching view modes does not invalidate or duplicate parsing when response is unchanged.

### Task 5: PERF-008 — Move Semantics for `RequestFile::into_item`
- Update `crates/core/src/storage/models.rs`:
  - Add `pub fn into_item(self, id: impl Into<String>, workspace_id: impl Into<String>, folder_id: Option<String>) -> HttpRequestItem`.
  - Implement `From<HttpRequestItem> for RequestFile` (or `pub fn from_item_owned(item: HttpRequestItem) -> Self`) using field moves.
- Update callers in `crates/app/src/state/workspace_data_state.rs`:
  - In `convert_node_to_requests()` and `reload_tree()`, consume `req_node.file.into_item(...)` directly.
- In `crates/app/src/state/persistence_state.rs`:
  - When updating from newly read request file, use `req_file.into_item(...)` instead of `req_file.to_item(...)`.

---

## 4. Verification & Validation Protocol
- Run `cargo fmt --check`
- Run `cargo test --workspace`
- Run `cargo clippy --all-targets -- -D warnings`
- Verify lines-of-code limits (< 650 LOC per module)
- Update `docs/technical-debt.md` marking PERF-005, CC-002, TD-07/TD-005, PERF-007, PERF-008 as `✅ RESOLVED`.

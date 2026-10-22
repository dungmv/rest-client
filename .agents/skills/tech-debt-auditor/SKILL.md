---
name: tech-debt-auditor
description: >-
  Use this skill to audit, diagnose, track, and resolve technical debt, anti-patterns,
  performance bottlenecks, and memory leaks across restclient-core and restclient-app.
---

# Technical Debt Auditor & Resolution Runbook

This skill provides diagnostic checklists and step-by-step procedures to maintain code health, audit anti-patterns, and resolve issues documented in [`docs/technical-debt.md`](docs/technical-debt.md).

---

## 1. Core Technical Debt Checklist & Status

### Unresolved Priority Items (Action Items):

| ID | Issue | Impact | File Location | Recommended Remedy |
| :--- | :--- | :---: | :--- | :--- |
| **PERF-005** | `suppress_path()` spawns new OS thread per file write | 🟠 High | `crates/core/src/storage/fs_store.rs` | Replace `std::thread::spawn` with async task or timer queue. |
| **CC-002** | `FsEvent` reload cascade render notifications | 🟠 High | `crates/app/src/state/persistence_state.rs` | Batch notifications and emit a single `WorkspaceTreeChanged` event. |
| **PERF-007** | `refresh_formatted_body()` reparses JSON on state change | 🟡 Med | `crates/app/src/state/response_state.rs` | Cache formatted body and invalidate only when raw body bytes change. |
| **TD-005** | `unwrap()` on Mutex locks (poisoning risk) | 🟡 Med | `crates/core/src/http/executor.rs` | Use `unwrap_or_else(\|poisoned\| poisoned.into_inner())`. |
| **TD-006** | Developer machine hardcoded paths in tests | 🟡 Med | `crates/core/src/storage/mod.rs` | Use `tempfile::tempdir()` or `env!("CARGO_MANIFEST_DIR")`. |
| **TD-007** | Non-English comments in core/scripting | 🟡 Med | `crates/core/src/scripting/` | Translate all source code comments to concise technical English. |
| **TD-014** | Recursive `$ref` resolution stack overflow risk | 🔵 Low | `crates/core/src/importers/openapi.rs` | Add depth limit (max 32) and cycle detection set. |
| **TD-017** | `body_editor.rs` giant file (>1,900 LOC) | 🔵 Low | `crates/app/src/components/body_editor.rs` | Modularize into `body_editor/` (`json.rs`, `xml.rs`, `form.rs`, `multipart.rs`). |

---

## 2. Anti-Pattern Detection Matrix

When reviewing or writing code, automatically flag and eliminate these anti-patterns:

### 1. God Objects & Struct Bloat
- **Limit**: No struct should exceed **15 fields** without modular grouping.
- **Limit**: No source file should exceed **~650 LOC**.

### 2. Synchronization Inside Render Loop (PERF-006)
- **Rule**: `AppView::render()` must NEVER call synchronization methods, mutation routines, or file system checks.
- State-to-input syncing belongs exclusively in `crates/app/src/app/subscriptions.rs`.

### 3. Unbounded Memory Growth (SL-002)
- **Rule**: Never store arbitrary historical payloads in unbounded `HashMap<K, V>`.
- Always wrap memory caches in `restclient_core::cache::LruMemoryCache` with both item count and byte size limits.

### 4. Blocking the UI Thread (TD-003, CC-004)
- **Rule**: Never run `std::process::Command`, `std::thread::sleep`, or synchronous network calls inside GPUI update blocks.
- Dispatch async tasks using `cx.background_executor().spawn(...)`.

### 5. Catch-All Deserialization (SL-005)
- **Rule**: Every YAML storage model must specify `#[serde(deny_unknown_fields)]`.
- Never include `extra: BTreeMap<String, Value>` fallback fields.

---

## 3. Auditing & Verification Commands

Execute this three-stage validation pipeline on every audit:

```bash
# Stage 1: Check compilation and unit tests
cargo test --workspace

# Stage 2: Strict Clippy linter with deny warnings
cargo clippy --workspace --all-targets -- -D warnings

# Stage 3: Inspect file lengths to ensure no giant files (> 650 lines) are introduced
find crates/ -type f -name "*.rs" -exec wc -l {} + | sort -rn | head -n 20
```

---

## 4. Updating the Technical Debt Registry

When you resolve any technical debt item:
1. Update `docs/technical-debt.md`:
   - Change badge from `🔴 Chưa xử lý` / `🟠 HIGH` to `✅ RESOLVED`.
   - Add a concise explanation of the solution applied and the date of resolution.
2. Ensure the resolution is accompanied by a dedicated unit test.

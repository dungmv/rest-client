# RestClient: High-Performance Desktop REST API Client in Rust

A fast, lightweight, and Git-friendly desktop REST API client (inspired by Yaak, Insomnia, and Bruno) built in **Rust** using Zed's **GPUI** UI framework and `gpui-component`.

Targeted for **macOS** and **Windows** with real HTTP request execution from the ground up and plain-text filesystem persistence designed for seamless Git collaboration.

---

## 🎨 Visual Design

- **Dark Navy Aesthetic:** Deep navy background (`~#1b1b2a`), panels (`#1e1e2f`), subtle 1px borders (`#2c2c42`), 8px border radius, and blue primary buttons (`#2f6bff`).
- **Method Badges:** GET (Purple `#a855f7`), POST (Green `#22c55e`), PUT (Blue `#3b82f6`), DELETE (Red `#ef4444`), PATCH (Yellow `#eab308`).
- **Status Indicators:** 2xx Green, 3xx Blue, 4xx Orange, 5xx Red.
- **Typography:** System monospace for URLs, headers, bodies, JSON, and curl previews; sans-serif for UI labels and titles.

---

## 💾 Filesystem-First Architecture (100% Plain Text)

The filesystem is the **only source of truth** for collections, environments, and workspace configurations. There is no hidden database or proprietary cache holding collection data.

### On-Disk Workspace Layout

```
my-workspace/
  workspace.yaml                 # version, name, default_environment
  .gitignore                     # auto-managed: contains `.local/` and `*.loc`
  environments/
    local.yaml                   # COMMIT: variables; secret variables store NAME only
    staging.yaml
  collections/
    auth/                        # group = directory
      _folder.yaml               # name, order: [login, refresh, logout] (stems)
      login.yaml                 # COMMIT: request + examples
      login.loc                  # GITIGNORE: last response cache (same stem as the yaml)
      refresh.yaml
    health/
      _folder.yaml
      healthz.yaml
  .local/                        # GITIGNORE: personal machine data
    secrets/local.yaml           # secret variable VALUES per environment
    cookies.json                 # cookie jar; expired cookies dropped on save
    pins.yaml                    # ordered relative request paths
    state.yaml                   # active request, environment, layout, recents
```

App-level global settings (recent workspace paths, theme, window geometry) live in a single JSON file in the OS config directory (`dirs::config_dir()`), never inside the workspace repository.

---

## 🤝 Git Collaboration Workflow

### What to Commit vs What to Gitignore
- **Committed Files:**
  - `workspace.yaml`: Workspace metadata and default environment.
  - `environments/*.yaml`: Environment variable declarations. Variables flagged as `secret: true` store **names only** without values.
  - `collections/**/<slug>.yaml`: Requests, settings, scripts, and saved examples.
  - `collections/**/_folder.yaml`: Folder display names and manual request ordering.
- **Gitignored Files (`.local/` and `*.loc`):**
  - `.local/secrets/<env>.yaml`: Actual secret variable values. Never committed.
  - `.local/cookies.json`: Personal cookie jars.
  - `.local/pins.yaml` and `.local/state.yaml`: Individual window, selection, and UI preferences.
  - `*.loc`: Last response caches (JSON format, truncated at 512KB for large payloads; binary bodies store metadata only).

### Deterministic Serialization
To prevent noisy diffs and merge conflicts when working in teams:
1. **Deterministic YAML:** 2-space indentation, trailing newline, strictly Unix LF line endings (even on Windows).
2. **Fixed Key Ordering:** `name`, `method`, `url`, `params`, `headers`, `auth`, `body`, `settings`, `script`, with `examples` **always last**.
3. **No Random IDs:** Request file paths use slugs (`login.yaml`, `login-2.yaml`). Pairing with `.loc` response cache is by exact stem match only.
4. **Unknown Key Preservation:** Preserves unknown keys when saving (flattened map), ensuring newer teammates' schema extensions are never stripped.
5. **Atomic Writes:** All filesystem mutations write to a temporary file in the same directory first, then perform an atomic rename with a 5x retry loop on Windows sharing violations.

### Automatic Protection & Conflict Handling
- **Auto-managed `.gitignore`:** Upon opening a workspace, `.gitignore` is created or verified to ensure `.local/` and `*.loc` are present.
- **Tracked Secret Warning:** If a Git repository is detected and sensitive paths (`.local/` or `*.loc`) are tracked by Git, a persistent warning toast alerts the user.
- **External Change Detection:** Background `notify` watcher automatically reloads changed files. If an external change modifies a file while the user has unsaved edits in memory, a conflict banner allows choosing **"Reload"** or **"Keep mine"**.
- **Parse Error Resilience:** Invalid YAML syntax from Git merge conflicts or edits does not crash the app. The sidebar displays an error icon, the editor presents an error banner with the exact line number, and the last good in-memory version is retained.

---

## 🧱 Codebase Structure

The project is structured as a Cargo virtual workspace with clean separation between UI and the storage trait:

```
restclient/
├── crates/
│   ├── core/                  # Pure business logic, models, executor, storage
│   │   ├── src/
│   │   │   ├── curl/          # cURL CLI importer and exporter
│   │   │   ├── http/          # HttpExecutor trait & ReqwestExecutor with timing breakdowns
│   │   │   ├── interpolation.rs # {{var}} interpolation & syntax scanning
│   │   │   ├── models.rs      # Data models (HttpRequestItem, HttpMethod, Auth, etc.)
│   │   │   ├── scripting/     # Embedded QuickJS (rquickjs) sandbox, API bridge, faker
│   │   │   └── storage/       # Filesystem persistence implementation
│   │   │       ├── fs_ops.rs      # Single-code-path filesystem CRUD operations
│   │   │       ├── fs_store.rs    # FsWorkspaceStore implementing WorkspaceStore trait
│   │   │       ├── models.rs      # Plain serde YAML/JSON models
│   │   │       ├── serialize.rs   # Deterministic LF serialization & atomic write with retry
│   │   │       ├── slug.rs        # Slugs, stem matching, collisions
│   │   │       ├── traits.rs      # WorkspaceStore trait
│   │   │       └── watcher.rs     # Debounced notify filesystem watcher & path suppression
│   │   └── Cargo.toml
│   └── app/                   # GPUI desktop application
│       ├── src/
│       │   ├── components/    # Reusable UI widgets
│       │   ├── platform/      # macOS vs Windows caption buttons & git branch chip
│       │   ├── settings.rs    # OS config directory AppSettings
│       │   ├── state/         # Fine-grained entity states (WorkspaceState, EditorState, ResponseState)
│       │   ├── theme.rs       # Theme tokens & palettes
│       │   └── main.rs        # App entry point, menu, keyboard shortcuts, split layouts
│       └── Cargo.toml
├── Cargo.toml                 # Workspace root
└── README.md
```

---

## ⌨️ Keyboard Shortcuts

| Shortcut | Action |
|---|---|
| `Ctrl + Enter` (or `Cmd + Enter`) | Send active request (or Cancel if in-flight) |
| `Ctrl + K` (or `Cmd + K`) | Toggle Command Palette |
| `Ctrl + Shift + G` (or `Cmd + Shift + G`) | Toggle Git Source Control (Commit, Diff, History, Checkout) |
| `Ctrl + [` (or `Cmd + [`) | Navigate back in history |
| `Ctrl + ]` (or `Cmd + ]`) | Navigate forward in history |
| `Ctrl + F` (or `Cmd + F`) | Focus search / filter |
| `Escape` | Dismiss modal / Command Palette |

---

## 🚀 Building & Running

### Requirements
- Rust stable (2021 edition)
- Windows 10/11 or macOS 12+

### Run Development
```bash
cargo run -p restclient
```

### Build Optimized Release
```bash
cargo build --release -p restclient
```

### Run Test Suite
```bash
cargo test --workspace
```

### Run Linter
```bash
cargo clippy --workspace --all-targets -- -D warnings
```

---

## ⚠️ Known Gaps

1. **AST Code-Folding in Response Body Viewer:**
   - *Current status:* Renders response lines virtualized with high performance, line numbering, and syntax styling.
   - *Future work:* Integration with tree-sitter AST syntax folding when `gpui-component` editor AST folding stabilizes.
2. **Native Pointer Drag-and-Drop Sidebar Reordering:**
   - *Current status:* Requests are reordered via `_folder.yaml`, context menus, and alphabetized fallback. Pinned requests and examples support menu-based reordering.
   - *Future work:* Interactive pointer drag handles once cross-platform mouse grab overlays are finalized.
3. **OpenAPI & Postman Importers:**
   - *Current status:* Full bidirectional cURL command import & export is supported.
   - *Future work:* Direct import from OpenAPI 3.1 specifications and Postman v2.1 collection files.

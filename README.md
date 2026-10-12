# RestClient: High-Performance Desktop REST API Client in Rust

A fast, lightweight, and Git-friendly desktop REST API client (inspired by Yaak, Insomnia, and Bruno) built in **Rust** using Zed's **GPUI** UI framework and the `gpui-component` crate.

Targeted for **macOS** and **Windows** with real HTTP request execution from the ground up.

---

## 🎨 Visual Design

- **Dark Navy Aesthetic:** Deep navy background (`~#1b1b2a`), panels (`#1e1e2f`), subtle 1px borders (`#2c2c42`), 8px border radius, and blue primary buttons (`#2f6bff`).
- **Method Badges:** GET (Purple `#a855f7`), POST (Green `#22c55e`), PUT (Blue `#3b82f6`), DELETE (Red `#ef4444`), PATCH (Yellow `#eab308`).
- **Status Indicators:** 2xx Green, 3xx Blue, 4xx Orange, 5xx Red.
- **Typography:** System monospace for URLs, headers, bodies, JSON, and curl previews; sans-serif for UI labels and titles.

---

## 🧱 Architecture

The project is structured as a Cargo virtual workspace with strict separation between UI and business logic:

```
restclient/
├── crates/
│   ├── core/                  # Pure business logic, models, executor, storage
│   │   ├── src/
│   │   │   ├── curl/          # cURL CLI importer and exporter
│   │   │   ├── http/          # HttpExecutor trait & ReqwestExecutor with timing breakdowns
│   │   │   ├── interpolation.rs # Pure {{var}} interpolation & syntax scanning
│   │   │   ├── models.rs      # Data types (HttpRequestItem, HttpMethod, Auth, etc.)
│   │   │   └── storage/
│   │   │       ├── fs.rs      # Plain-file collection storage (Git-friendly, stable JSON)
│   │   │       └── history.rs # SQLite response history with automatic pruning
│   │   └── Cargo.toml
│   └── app/                   # GPUI UI application
│       ├── src/
│       │   ├── components/    # Reusable UI widgets
│       │   │   ├── body_editor.rs     # JSON / Raw / Form body editor with line numbers
│       │   │   ├── command_palette.rs # Cmd/Ctrl+K fast actions
│       │   │   ├── cookies_modal.rs   # Cookie jar manager
│       │   │   ├── env_modal.rs       # Environment variable editor
│       │   │   ├── kv_table.rs        # Headers / Query params key-value table
│       │   │   ├── request_bar.rs     # Method dropdown, URL bar, Send/Cancel button
│       │   │   ├── response_viewer.rs # Status metrics, timing breakdown, headers & body
│       │   │   ├── sidebar_tree.rs    # Folders & requests tree with search
│       │   │   ├── status_badge.rs    # Color-coded method and status badges
│       │   │   ├── tabs_with_badge.rs # Tabs with counter badges
│       │   │   ├── titlebar.rs        # Custom borderless titlebar with traffic lights / caption buttons
│       │   │   └── toast.rs           # Toast notification system
│       │   ├── platform/      # macOS traffic lights vs Windows caption buttons & git branch chip
│       │   ├── state/         # Fine-grained entity states (WorkspaceState, EditorState, ResponseState)
│       │   ├── theme.rs       # Central token catalog & color palettes
│       │   └── main.rs        # GPUI application entry point & Tokio executor bridge
│       └── Cargo.toml
├── Cargo.toml                 # Workspace root
└── README.md
```

### Decoupled HTTP & Tokio Threading
GPUI runs its UI event loop on the main thread. All HTTP execution runs on a dedicated multi-threaded `tokio::runtime::Runtime`. In-flight requests can be cancelled cleanly, progress is streamed over unbounded channels, and results are dispatched back to GPUI entities without freezing or stuttering the UI.

---

## 📌 Pinned Dependencies

To ensure 100% reproducible builds with GPUI, versions are strictly pinned:

| Crate | Pinned Version | Notes |
|---|---|---|
| `gpui-pre` | `=0.3.9` | Zed's UI framework |
| `gpui-component` | `=0.7.3` | High-level widgets & resizable dock panels |
| `gpui-kit` | `=0.7.3` | System assets and icons |
| `reqwest` | `=0.13.5` | `rustls`, `gzip`, `brotli`, `cookies`, `stream`, `multipart` |
| `reqwest_cookie_store` | `=0.10.0` | In-memory and persistent cookie jars |
| `cookie_store` | `=0.22.1` | RFC 6265 cookie jar |
| `rusqlite` | `=0.32.1` | SQLite backend for response history |
| `tokio` | `=1.53.2` | Full async runtime for networking |

---

## 💾 Storage Design

- **Git-Friendly Plain Files:**
  - Workspaces are stored under `.restclient/workspaces/<workspace_name>/`.
  - Folders represent request groups.
  - Each request is saved as an individual human-readable JSON file with sorted keys. Diffs cleanly in Git pull requests with zero binary merge conflicts.
- **SQLite Response History:**
  - Located at `.restclient/history.db`.
  - Automatically records response status, headers, body, and timing metrics.
  - Keeps the last $N$ runs per request (configurable, default: 20) with automatic pruning.
- **Environments & Variables:**
  - Named variable sets (e.g., `local`, `staging`, `production`).
  - Interpolation supports `{{variable_name}}` in URLs, headers, query params, auth, and bodies.
  - Unresolved variables are highlighted in red.

---

## ⌨️ Keyboard Shortcuts

| Shortcut | Action |
|---|---|
| `Ctrl + Enter` (or `Cmd + Enter`) | Send active request (or Cancel if in-flight) |
| `Ctrl + K` (or `Cmd + K`) | Toggle Command Palette |
| `Ctrl + F` (or `Cmd + F`) | Focus search / filter |
| `Double Click` (Sidebar) | Pin request tab |
| `Escape` | Dismiss modal / Command Palette |

---

## 🚀 Building & Running

### Requirements
- Rust stable (edition 2021)
- Windows 10/11 or macOS 12+

### Run Development
```bash
cargo run -p restclient
```

### Build Optimized Release
```bash
cargo build --release -p restclient
```
The binary will be located at `target/release/restclient.exe` (Windows) or `target/release/restclient` (macOS).

### Run Test Suite
```bash
cargo test --workspace
```

### Run Linter
```bash
cargo clippy --workspace -- -D warnings
```

---

## ⚠️ Known Gaps & Future Roadmap

1. **AST Code-Folding in JSON Viewer:**
   - *Current status:* Renders lines virtualized with line numbering and syntax styling.
   - *Future work:* Integration with tree-sitter syntax folding when `gpui-component` editor AST folding stabilizes.
2. **Native Drag-and-Drop Sidebar Reordering:**
   - *Current status:* Requests and folders are reordered via context actions and search filters.
   - *Future work:* Pointer drag-and-drop handles once cross-platform mouse grab overlays are finalized.
3. **OpenAPI & Postman Importers:**
   - *Current status:* Full bidirectional cURL command import & export is supported.
   - *Future work:* OpenAPI 3.1 schema and Postman v2.1 collection JSON parsers.

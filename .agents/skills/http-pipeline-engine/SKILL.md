---
name: http-pipeline-engine
description: >-
  Use this skill when developing, refactoring, or optimizing HTTP request execution,
  connection pooling, response parsing, hierarchy resolution, or multi-protocol features in core and app.
---

# HTTP Pipeline & Execution Engine Guide

This skill governs the networking pipeline in `zest-core::http` and the application runner service in `crates/app/src/services/`.

---

## 1. Architectural Separation: `HttpEngine` vs `Runner`

The HTTP execution flow is cleanly decoupled from the UI thread into two layers:

```
UI Interaction (AppView / RequestComposer)
      │
      ▼
Runner::send_active_request(cx)  [crates/app/src/services/runner.rs]
      │
      ▼ (Spawns background task via cx.background_executor())
HttpEngine::execute(resolved_request, ...) [crates/app/src/services/http_engine.rs]
      │
      ├── 1. Run Pre-request Script (QuickJS)
      ├── 2. Apply script mutations (headers, JSON body, env vars)
      ├── 3. Send over network via ReqwestExecutor (Connection Pooled)
      ├── 4. Run Post-response Script / Tests
      └── 5. Return Arc<HttpResponseRecord> & PostResponseOutput
      │
      ▼ (Event dispatch)
Runner dispatches ExecEvent::Success / Error back to GPUI state entities
```

### Key Rules:
1. **`HttpEngine` is 100% UI-independent**: It does not depend on GPUI or `Window`. It can be tested in isolation with standard Rust unit tests.
2. **Zero-Copy Payload Transfer (SL-006)**: Always wrap `HttpResponseRecord` in an `Arc<HttpResponseRecord>` before returning from the background thread. Never deep-copy raw body buffers (`Vec<u8>`).

---

## 2. Connection Pooling & Socket Reuse (PERF-001, PERF-002)

In `crates/core/src/http/executor.rs`, `ReqwestExecutor` manages persistent HTTP connection pools:

- **Client Caching**: `reqwest::Client` instances are cached in an `RwLock<HashMap<(bool, bool), reqwest::Client>>` keyed by `(follow_redirects, validate_tls)`.
- **Pre-warming**: The default client is pre-warmed during application startup (`ReqwestExecutor::new()`).
- **No TCP Probing**: Redundant TCP connection probes (`probe_connection`) are completely eliminated.
- **Connection Reuse Tracking**: Keep-alive socket reuse is detected via `active_hosts` tracking, populating `connection_reused = true` in `HttpResponseRecord` (displays `⚡ Connection Reused` badge in UI).
- **Timeouts**: Configured per-request via `RequestBuilder::timeout(Duration::from_secs(t))`.

---

## 3. Hierarchy & Variable Resolution

Before executing a request, resolution must follow the strict inheritance chain:

```
Global Environment
      │
      ▼
Active Environment (with local secrets override)
      │
      ▼
Workspace Settings (Headers, Auth)
      │
      ▼
Parent Folder Hierarchy (Inherited Auth, Headers)
      │
      ▼
Request Settings (Direct overrides)
```

### Variable Interpolation (`zest_core::interpolation`):
- Double curly braces `{{variable_name}}` are replaced with resolved values.
- Dynamic fake variables (`{{$uuid}}`, `{{$timestamp}}`, `{{$randomEmail}}`) are evaluated per execution.
- Use `VariableInterpolator::interpolate_cow()` to avoid heap allocations when no variables are present.

---

## 4. Multi-Protocol Extensions (Spec 03)

Refer to [`docs/specs/03-protocols-graphql-websocket-grpc.md`](docs/specs/03-protocols-graphql-websocket-grpc.md) when adding protocol extensions:
- **GraphQL**: Send POST request with JSON body `{ "query": "...", "variables": { ... } }`, automatic schema introspection via `__schema`.
- **WebSocket**: Asynchronous duplex communication, message history log, auto-reconnect option.
- **Server-Sent Events (SSE)**: Streaming tokens via `Transfer-Encoding: chunked` or `text/event-stream`, real-time incremental UI update in `InspectorDisplays`.
- **gRPC**: HTTP/2 with Protobuf schema reflection.

---

## 5. Verification & Testing

```bash
# 1. Run core HTTP unit tests
cargo test --package zest_core --lib http::executor::tests

# 2. Run runner and service unit tests
cargo test --package zest --lib services::http_engine::tests

# 3. Clippy check
cargo clippy --package zest_core --lib -- -D warnings
```

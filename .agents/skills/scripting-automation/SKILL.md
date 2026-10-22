---
name: scripting-automation
description: >-
  Use this skill when developing, refactoring, or testing the JavaScript scripting engine (rquickjs),
  Postman pm.* API compatibility, pre/post-request scripts, assertions, and dynamic fake variables.
---

# Scripting Engine & Automation Guide

This skill governs the JavaScript automation engine implemented in `restclient-core::scripting`, providing high compatibility with Postman (`pm.*`) and Insomnia scripting workflows.

---

## 1. Engine Architecture & Runtime

- **Runtime**: QuickJS embedded engine via `rquickjs` (v0.8).
- **Core Modules** in `crates/core/src/scripting/`:
  - `sandbox.rs`: JS execution context, timer management, and sandboxing.
  - `api.rs`: Postman-compatible `pm.*` API injection into the JS global scope.
  - `fake_vars.rs`: Macro generation for dynamic fake variables (`{{$uuid}}`, `{{$timestamp}}`, etc.).

---

## 2. Supported `pm.*` API Specifications

Scripts executed in RestClient have access to the global `pm` (and alias `postman`) object:

### 2.1 Variables & Environment Management
- `pm.environment.get(key: string): string`
- `pm.environment.set(key: string, value: any): void`
- `pm.environment.unset(key: string): void`
- `pm.variables.get(key: string): string`
- `pm.variables.set(key: string, value: any): void`

### 2.2 Pre-Request Script Mutations (`apply_pre_script_mutations`)
Pre-request scripts can mutate the outgoing request before network transmission:
- `pm.request.headers.add({ key: string, value: string })`
- `pm.request.headers.upsert({ key: string, value: string })`
- `pm.request.headers.remove(key: string)`
- `pm.request.body.raw`: Read or overwrite the raw request body.

### 2.3 Assertions & Test Suite (`pm.test` & `pm.expect`)
Post-response scripts can declare assertions:
```javascript
pm.test("Status code is 200 OK", function () {
    pm.response.to.have.status(200);
});

pm.test("Response contains access token", function () {
    const jsonData = pm.response.json();
    pm.expect(jsonData.token).to.be.a("string");
    // Request Chaining: Store token for downstream requests
    pm.environment.set("access_token", jsonData.token);
});
```

### 2.4 Cryptographic Utilities
- `pm.crypto.md5(str)`
- `pm.crypto.sha1(str)`
- `pm.crypto.sha256(str)`
- `pm.crypto.hmacSha256(secret, message)`
- `pm.crypto.base64Encode(str)` / `pm.crypto.base64Decode(str)`

---

## 3. Dynamic Fake Variables (`{{$...}}`)

Evaluated dynamically during variable interpolation (`crates/core/src/scripting/fake_vars.rs`):

| Variable | Description | Example Output |
| :--- | :--- | :--- |
| `{{$uuid}}` | UUID v4 | `9b1deb4d-3b7d-4bad-9bdd-2b0d7b3dcb6d` |
| `{{$timestamp}}` | Current UNIX timestamp in seconds | `1761123456` |
| `{{$isoTimestamp}}` | ISO 8601 UTC timestamp | `2026-10-22T09:00:00Z` |
| `{{$randomInt}}` | Random integer between 0 and 1000 | `482` |
| `{{$randomEmail}}` | Random fake email address | `user_4918@example.com` |

---

## 4. Sandboxing & Security Boundaries

1. **No Filesystem / OS Access**: The QuickJS context must never expose Node.js `fs`, `child_process`, or arbitrary networking primitives.
2. **Execution Timeout**: Protect script runs with execution timeouts (default: 5,000ms) to prevent infinite loops from hanging the application.
3. **English Comments**: Maintain 100% English code comments and docstrings in `scripting/` (TD-007 resolution).

---

## 5. Verification & Testing

```bash
# 1. Run all scripting unit tests (sandbox, crypto, assertions, fake vars)
cargo test --package restclient_core --lib scripting::

# 2. Run clippy
cargo clippy --package restclient_core --lib scripting -- -D warnings
```

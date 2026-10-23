---
name: scripting-automation
description: >-
  Use this skill when developing, refactoring, or testing the JavaScript scripting engine (rquickjs),
  `z.*` API, pre/post-request scripts, assertions, and dynamic fake variables.
---

# Scripting Engine & Automation Guide

This skill governs the JavaScript automation engine implemented in `zest-core::scripting`, providing high performance scripting workflows.

---

## 1. Engine Architecture & Runtime

- **Runtime**: QuickJS embedded engine via `rquickjs` (v0.8).
- **Core Modules** in `crates/core/src/scripting/`:
  - `sandbox.rs`: JS execution context, timer management, and sandboxing.
  - `api.rs`: `z.*` API injection into the JS global scope.
  - `fake_vars.rs`: Macro generation for dynamic fake variables (`{{$uuid}}`, `{{$timestamp}}`, etc.).

---

## 2. Supported `z.*` API Specifications

Scripts executed in Zest have access solely to the global `z` object:

### 2.1 Variables & Environment Management
- `z.environment.get(key: string): string`
- `z.environment.set(key: string, value: any): void`
- `z.environment.unset(key: string): void`
- `z.variables.get(key: string): string`
- `z.variables.set(key: string, value: any): void`

### 2.2 Pre-Request Script Mutations (`apply_pre_script_mutations`)
Pre-request scripts can mutate the outgoing request before network transmission:
- `z.request.headers.add(name: string, value: string)`
- `z.request.headers.get(name: string): string | null`
- `z.request.headers.all(): Record<string, string>`
- `z.request.body.json()`: Read JSON request body.
- `z.request.body.setJson(obj: any)`: Overwrite JSON request body.

### 2.3 Assertions & Test Suite (`z.test` & `z.assert`)
Post-response scripts can declare assertions:
```javascript
z.test("Status code is 200 OK", function () {
    z.assert(z.response.status === 200, "Status must be 200");
});

z.test("Response contains access token", function () {
    const jsonData = z.response.json();
    z.assert(jsonData && jsonData.token, "Token must be present");
    // Request Chaining: Store token for downstream requests
    z.environment.set("access_token", jsonData.token);
});
```

### 2.4 Cryptographic Utilities
- `z.crypto.md5(str)`
- `z.crypto.sha256(str)`
- `z.crypto.hmacSha256(secret, message)`
- `z.crypto.base64Encode(str)` / `z.crypto.base64Decode(str)`
- `z.crypto.uuidv4()`

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
cargo test --package zest_core --lib scripting::

# 2. Run clippy
cargo clippy --package zest_core --lib scripting -- -D warnings
```

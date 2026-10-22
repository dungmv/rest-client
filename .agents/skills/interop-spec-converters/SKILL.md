---
name: interop-spec-converters
description: >-
  Use this skill when developing, refactoring, or testing API specification importers,
  exporters (OpenAPI, Postman, Insomnia, cURL, HAR), and code generation in core and app.
---

# Interoperability, Importers & Spec Converters Guide

This skill governs data migration and conversion between external formats and RestClient canonical domain models in `restclient-core::importers` and `restclient-core::curl`.

---

## 1. Supported Formats & Capabilities

| Format | Source Specification | Supported Versions | Key Module |
| :--- | :--- | :--- | :--- |
| **OpenAPI / Swagger** | OpenAPI Specification | 3.0, 3.1 (JSON & YAML) | `crates/core/src/importers/openapi.rs` |
| **Postman** | Postman Collection Format | v2.0, v2.1 (JSON) | `crates/core/src/importers/postman.rs` |
| **Insomnia** | Insomnia Export Format | v4 (JSON/YAML) | `crates/core/src/importers/insomnia.rs` |
| **cURL** | Shell command parser & exporter | Bash, PowerShell, CMD | `crates/core/src/curl/mod.rs` |
| **HAR** | HTTP Archive 1.2 | Browser DevTools export | `crates/core/src/importers/har.rs` |

---

## 2. OpenAPI 3.0 / 3.1 Importer Principles

### 2.1 Cycle-Safe Reference Resolution (TD-014)
When resolving `$ref` pointers (e.g., `#/components/schemas/Pet`), the resolver must guard against circular schemas:
```rust
// ✅ MANDATORY: Use depth limits or a visited set to prevent recursion stack overflow
fn resolve_ref<'a>(
    root: &'a Value,
    val: &'a Value,
    visited: &mut HashSet<String>,
    depth: usize,
) -> Option<&'a Value> {
    if depth > 32 { return None; } // Prevent deep/infinite recursion
    // ...
}
```

### 2.2 Content-Type Parsing & Fallback (TD-01)
Avoid `clippy::never_loop` bugs when iterating over media type mappings:
```rust
// ✅ CORRECT: Extract first media type safely
if let Some((media_type, media_obj)) = content.into_iter().next() {
    // Process single default body format
}
```

---

## 3. cURL Command Parser & Generator (`restclient-core::curl`)

### 3.1 Parser Capabilities
- Detects cURL commands via `is_curl_command(text)`.
- Handles multiline backslash (`\`) and PowerShell backtick (`` ` ``) line continuations.
- Handles flags positioned before or after the URL (`-X POST`, `-H`, `-d`, `--data-raw`, `-u`).
- Parses Basic Auth flags (`-u user:pass`) directly into `AuthType::Basic`.

### 3.2 Importer Integration
Imported cURL strings pasted into the URL address bar or import modal are mapped directly into canonical `HttpRequestItem` without intermediate allocations.

---

## 4. Code Generation Engine (Spec 06)

Refer to [`docs/specs/06-developer-tools-codegen-diff-docs.md`](docs/specs/06-developer-tools-codegen-diff-docs.md) when generating code snippets:
- `cURL`: One-line or multiline shell snippet.
- `Python`: `requests` and `httpx`.
- `JavaScript / TypeScript`: `fetch` and `axios`.
- `Rust`: `reqwest`.
- `Go`: `net/http`.

---

## 5. Verification & Testing

```bash
# 1. Run OpenAPI and Postman importer tests
cargo test --package restclient_core --lib importers::tests

# 2. Run cURL parser and exporter tests
cargo test --package restclient_core --lib curl::tests

# 3. Clippy check
cargo clippy --package restclient_core --lib importers curl -- -D warnings
```

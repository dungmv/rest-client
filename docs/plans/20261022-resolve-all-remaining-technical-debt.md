# Plan: Resolve All Remaining Technical Debt Items

## 1. Goal
Comprehensively resolve all remaining Technical Debt items listed in `docs/technical-debt.md` and `.agents/skills/tech-debt-auditor/SKILL.md`:
1. **TD-006**: Replace developer machine hardcoded path in `crates/core/src/storage/mod.rs`.
2. **TD-08 / TD-007**: Translate remaining Vietnamese source comments to technical English in core & scripting.
3. **PERF-009**: Reuse a single `VariableInterpolator` instance across loops in `crates/core/src/interpolation.rs`.
4. **TD-008**: Encapsulate `suppressed_paths` as private field in `FsWorkspaceStore`.
5. **TD-009**: Box large payload in `PostResponseOutput` / runner events in `crates/app/src/services/runner.rs`.
6. **TD-09**: Optimize struct padding and memory alignment in `TimingBreakdown` (`crates/core/src/models.rs`).
7. **TD-014**: Protect against circular `$ref` infinite recursion in `crates/core/src/importers/openapi.rs`.
8. **PERF-013**: Eliminate excessive string lowercase allocations in `normalize_path_for_compare` (`crates/core/src/storage/watcher.rs`).
9. **TD-018 & TD-019**: Safe git discard and command execution safety in `crates/core/src/git/mod.rs`.
10. **TD-015**: Separate I/O out of `ModalState::default()` in `crates/app/src/state/modal_state.rs`.
11. **CC-003**: Ensure git sync operations run asynchronously without UI thread stalls.
12. **PERF-010**: Execute scripting safely on background thread in `runner.rs`.
13. **TD-011**: Refactor parameter lists in `modals.rs` and `auth_tab.rs` to eliminate `#[allow(clippy::too_many_arguments)]`.
14. **SL-007**: Optimize tree node allocation in `crates/core/src/storage/models.rs`.
15. **TD-017**: Modularize `crates/app/src/components/body_editor.rs` (>2,100 LOC) into `crates/app/src/components/body_editor/`.

---

## 2. Phased Execution

### Phase 1: Core Engine Debt & Optimizations (TD-006, TD-007, PERF-009, TD-008, TD-09, TD-014, PERF-013, TD-018, TD-019)
- Clean up test hardcoded paths in `storage/mod.rs`.
- Translate comments in `scripting/sandbox.rs` and `http/executor.rs`.
- Optimize variable interpolator reuse in `interpolation.rs`.
- Private encapsulation for `suppressed_paths`.
- Struct alignment for `TimingBreakdown`.
- Depth counter (max 32) and visited ref set for OpenAPI `$ref`.
- In-place ASCII lowercase path comparison in `watcher.rs`.
- Command execution timeout / safe discard in `git/mod.rs`.

### Phase 2: App & Concurrency Improvements (TD-009, TD-015, CC-003, PERF-010, TD-011, SL-007)
- Box runner enum payload in `runner.rs`.
- Remove synchronous file read from `ModalState::default()`, load lazily or asynchronously.
- Verify async dispatch in `git_ops.rs` and scripting runner.
- Replace `too_many_arguments` with grouped parameter context structs.
- Optimize recursive folder node capacity allocation.

### Phase 3: Giant Component Modularization (TD-017)
- Decompose `crates/app/src/components/body_editor.rs` into:
  - `crates/app/src/components/body_editor/mod.rs`
  - `crates/app/src/components/body_editor/raw.rs`
  - `crates/app/src/components/body_editor/json.rs`
  - `crates/app/src/components/body_editor/xml.rs`
  - `crates/app/src/components/body_editor/form.rs`
  - `crates/app/src/components/body_editor/multipart.rs`

### Phase 4: Full Verification & Debt Registry Update
- Verify `cargo fmt --check`, `cargo test --workspace`, `cargo clippy --all-targets -- -D warnings`.
- Verify all source files adhere to < 650 LOC.
- Update `docs/technical-debt.md` marking all resolved items.

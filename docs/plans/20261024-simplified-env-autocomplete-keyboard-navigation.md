# Implementation Plan - Simplified Autocomplete & Keyboard Navigation

Simplify the environment variable suggestion popup by removing the title bar (`ENVIRONMENT VARIABLES` / `Click to insert`) and adding full keyboard navigation (`ArrowUp`, `ArrowDown`, `Enter`, `Tab`, `Escape`) across all input fields.

---

## User Review Required

> [!IMPORTANT]
> - **Header Bar Removal**: The top title bar (`ENVIRONMENT VARIABLES` + `Click to insert` and the border line below it) is completely removed as requested. The popup will now render directly as a sleek, compact suggestion list.
> - **Keyboard Navigation**:
>   - `ArrowDown`: Moves selection down (cycles to top when reaching the end) and scrolls the view to keep the active item visible.
>   - `ArrowUp`: Moves selection up (cycles to bottom when reaching the top).
>   - `Enter` / `Tab`: Confirms the selected candidate, replaces the unclosed `{{query` with `{{candidate_name}}`, tokenizes into a tag pill, and closes the popup.
>   - `Escape`: Dismisses the suggestion popup until the query changes or is re-triggered.
> - **Scope of Application**: Applied consistently to all inputs using `render_env_input` (Params table, Headers table, Request Auth, Scope Auth, Multipart form) and the URL bar (`request_bar.rs`).

---

## Context & Problem Analysis

In the current implementation:
1. **Visual Clutter**: The autocomplete dropdown in `crates/app/src/components/env_input.rs` and `crates/app/src/components/request_bar.rs` renders a static top title bar:
   ```rust
   // ENVIRONMENT VARIABLES           Click to insert
   // -----------------------------------------------
   ```
   This takes up vertical space, adds visual complexity, and clutters the UI.
2. **Missing Keyboard Navigation**:
   - There is no `selected_index` or keyboard event handling on the autocomplete popup.
   - Pressing Arrow keys moves the text caret inside the text input instead of cycling suggestions.
   - Pressing Enter or Tab does not pick the suggestion.
   - Users are forced to stop typing, take their hands off the keyboard, and click with the mouse.
3. **No Focus Dismissal**:
   - The popup currently opens whenever text contains an unclosed `{{`, even if the input field is not currently focused or after user clicks away.

---

## Architecture & Design

```mermaid
flowchart TD
    subgraph Input Focus & Typing
        Input["User types {{ or {{prefix in Input"] --> FocusCheck{"Input is focused?"}
        FocusCheck -->|No| Hidden["Hide Autocomplete"]
        FocusCheck -->|Yes| Detect["detect_unclosed_env_var()"]
    end

    subgraph Autocomplete State Management
        Detect --> NavState["AutocompleteNavState (window.use_keyed_state)\n- selected_index\n- dismissed\n- last_query"]
        NavState --> Popup["Render Clean Suggestion List\n(No title bar)"]
    end

    subgraph Keyboard Interception (capture_key_down)
        Key["Key Down Event"] --> MatchKey{Key type?}
        MatchKey -->|"Down / ArrowDown"| Inc["selected_index = (idx + 1) % len\nscroll_to_item(idx)\ncx.stop_propagation()"]
        MatchKey -->|"Up / ArrowUp"| Dec["selected_index = (idx - 1) % len\nscroll_to_item(idx)\ncx.stop_propagation()"]
        MatchKey -->|"Enter / Tab"| Insert["Insert {{selected_candidate}}\nTokenize tag pill\ndismissed = true\ncx.stop_propagation()"]
        MatchKey -->|"Escape"| Dismiss["dismissed = true\ncx.stop_propagation()"]
        MatchKey -->|"Other (typing)"| Pass["Allow event to reach Input\nUpdate query & reset idx"]
    end
```

---

## Proposed Changes

### Component 1: Autocomplete Navigation State & Helpers

#### [MODIFY] `crates/app/src/components/env_autocomplete.rs`
- Add `AutocompleteNavState`:
  ```rust
  #[derive(Default, Clone, Debug)]
  pub struct AutocompleteNavState {
      pub selected_index: usize,
      pub dismissed: bool,
      pub last_query: Option<String>,
  }
  ```
- Add keyboard intercept helper `handle_autocomplete_keydown`:
  ```rust
  pub fn handle_autocomplete_keydown<F>(
      event: &gpui::KeyDownEvent,
      candidates: &[EnvVarCandidate],
      nav_state: &gpui::Entity<AutocompleteNavState>,
      scroll_handle: &gpui::ScrollHandle,
      window: &mut Window,
      cx: &mut App,
      on_select: F,
  ) -> bool
  where
      F: FnOnce(&EnvVarCandidate, &mut Window, &mut App);
  ```
- Add `render_autocomplete_popup` helper to render the unified, title-free suggestion dropdown with:
  - Active selection highlight (`t.bg_selected`).
  - Hover highlight (`t.bg_hover`).
  - Synchronized `ScrollHandle` tracking with `.track_scroll(scroll_handle)`.
  - Clean compact spacing and typography.

---

### Component 2: Unified Input Autocomplete

#### [MODIFY] `crates/app/src/components/env_input.rs`
- Check focus state: `let is_focused = input_state.read(cx).focus_handle(cx).is_focused(window);`
- Use `AutocompleteNavState` and `ScrollHandle` via `window.use_keyed_state`.
- Remove the top title element (`"ENVIRONMENT VARIABLES"` / `"Click to insert"`).
- Attach `.capture_key_down` on the input wrapper to intercept navigation keys when autocomplete is active.
- Highlight the active candidate item at `selected_index`.
- When Enter, Tab, or mouse click triggers selection:
  - Replace the unclosed query range with `{{candidate.name}}`.
  - Convert to tokenized tags via `build_tokenized_content`.
  - Call `on_update` callback.
  - Set `dismissed = true` and `cx.notify()`.

---

### Component 3: Request URL Bar Autocomplete

#### [MODIFY] `crates/app/src/components/request_bar.rs`
- Remove the `"ENVIRONMENT VARIABLES"` / `"Click to insert"` header.
- Add `AutocompleteNavState` and `ScrollHandle` for the URL input.
- Intercept ArrowUp, ArrowDown, Enter, Tab, Escape via `.capture_key_down`.
- Highlight selected candidate and auto-scroll.
- Support selecting via Enter/Tab or click to apply URL and update workspace request state.

---

## Verification Plan

### Automated Tests
1. Run unit tests for `env_autocomplete`:
   ```bash
   cargo test -p zest --lib components::env_autocomplete
   ```
2. Run workspace clippy analysis to ensure 0 warnings:
   ```bash
   cargo clippy --all-targets -- -D warnings
   ```
3. Run full test suite:
   ```bash
   cargo test
   ```

### Manual Verification
1. Open Zest.
2. Navigate to any request with Params or Headers table.
3. In a Key or Value field, type `{{`.
   - Verify the suggestion popup opens immediately without any title header.
   - Verify the first item is visibly selected.
4. Press `ArrowDown` and `ArrowUp`.
   - Verify the selection moves up and down smoothly.
   - Verify the list scrolls if navigating past visible bounds.
5. Press `Enter` or `Tab`.
   - Verify the selected variable is inserted as a token tag pill `{{var}}`.
   - Verify the popup closes immediately.
6. Type `{{` again and press `Escape`.
   - Verify the popup dismisses cleanly without closing the tab or modal.
7. Test the URL bar in `request_bar.rs`:
   - Type `https://api.com/{{` in the URL bar.
   - Use Arrow keys and Enter to select an environment variable.

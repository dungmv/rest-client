# Subtle Git Changes and Status Badges Plan

## Goal Description
Zest is an API development and testing client, not a dedicated Git GUI. Currently, when workspace files have unstaged or uncommitted changes, the sidebar applies aggressive visual styling:
1. Every folder with modified descendants turns both the folder icon and text bright yellow, accompanied by a bold yellow letter **"M"** on the right.
2. Every modified request turns its text bright yellow, medium font weight, accompanied by a bold yellow letter **"M"** (or **"U"**) on the right.
3. If a request is unsaved, it displays both a blue dot **"●"** and a yellow letter **"M"**.
4. Status code badges in the sidebar (`200`) and the response inspector status pill (`● 200 OK`) are rendered in bold font weight (`gpui::FontWeight::BOLD`), creating unnecessary visual noise.

This task refactors these indicators to be clean, subtle, and non-distracting:
- Replace the bold **"M"** and **"U"** badges on folders and requests with a subtle status dot (6px circle) placed at the right edge.
- Restore normal text colors (`t.text_primary` / `t.text_secondary`) and font weight (`NORMAL`) for folder and request labels regardless of Git status.
- Consolidate request change indicators into a single clean dot (e.g. primary for unsaved buffer, yellow for Git modified, green for Git new).
- Change HTTP status code badges (`200`) in the sidebar and the response inspector header from `BOLD` to `NORMAL` font weight.

---

## User Review Required
> [!NOTE]
> The Git calculation engine (`calculate_request_git_status` and `calculate_folder_git_status`) and context menu Git actions (such as "Discard Changes") remain 100% intact and functional. Only the visual rendering in the tree and badge components is softened.

---

## Proposed Changes

### Component 1: Status Code Badges & Response Inspector
Make `StatusCodeBadge` default to normal font weight and provide a builder method matching `MethodBadge`. Update `response_inspector` status badge font weight.

#### [MODIFY] `crates/app/src/components/status_badge.rs`
- Add `font_weight: gpui::FontWeight` field to `StatusCodeBadge`.
- Default `font_weight` to `gpui::FontWeight::NORMAL` in `StatusCodeBadge::new`.
- Add `pub fn font_weight(mut self, weight: gpui::FontWeight) -> Self` builder.
- Add unit test verifying default normal font weight and builder method.

#### [MODIFY] `crates/app/src/components/response_inspector.rs`
- Change status badge `font_weight` from `gpui::FontWeight::BOLD` to `gpui::FontWeight::NORMAL`.

---

### Component 2: Sidebar Folder & Request Nodes
Remove loud text coloring and bold letter badges. Use subtle status dots.

#### [MODIFY] `crates/app/src/components/sidebar/folder_node.rs`
- Keep folder icon color as `t.text_secondary` (or normal theme color) instead of `folder_color`.
- Keep folder label text color as `t.text_primary` instead of `folder_color`.
- In the right-hand indicators, replace the bold letter "M" / "U" with a subtle 6px rounded circle:
  ```rust
  .when(folder_git_status != SidebarGitStatus::Clean, |this| {
      let dot_color = match folder_git_status {
          SidebarGitStatus::New => git_green,
          SidebarGitStatus::Modified => git_yellow,
          SidebarGitStatus::Clean => unreachable!(),
      };
      this.child(div().w(px(6.)).h(px(6.)).rounded_full().bg(dot_color))
  })
  ```

#### [MODIFY] `crates/app/src/components/sidebar/request_node.rs`
- Keep request label text color as `t.text_primary` and `gpui::FontWeight::NORMAL` instead of coloring it bright yellow/green with `gpui::FontWeight::MEDIUM`.
- Consolidate change indicator at the right: replace bold "M" / "U" and separate unsaved dot with a single subtle 6px rounded circle:
  ```rust
  .when(is_unsaved || req_git_status != SidebarGitStatus::Clean, |this| {
      let dot_color = if is_unsaved {
          t.primary
      } else {
          match req_git_status {
              SidebarGitStatus::New => git_green,
              SidebarGitStatus::Modified => git_yellow,
              SidebarGitStatus::Clean => unreachable!(),
          }
      };
      this.child(div().w(px(6.)).h(px(6.)).rounded_full().bg(dot_color))
  })
  ```

---

## Verification Plan

### Automated Tests
1. Run all unit tests across the workspace:
   ```bash
   cargo test
   ```
2. Run strict clippy analysis:
   ```bash
   cargo clippy --all-targets -- -D warnings
   ```

### Manual Verification
1. Launch Zest in a workspace with modified requests/folders.
2. Verify folder labels and icons remain neutral and standard color (no aggressive yellow).
3. Verify a subtle colored dot appears on the right of modified folders where "M" used to be.
4. Verify request labels remain neutral color and normal font weight.
5. Verify a subtle colored dot appears on the right of modified requests where "M" used to be.
6. Verify status code pills (`200`) in the sidebar and response inspector display with clean, non-bold font weight.

use crate::components::*;
use crate::state::{EditorState, ToastState, WorkspaceState};
use gpui::prelude::FluentBuilder as _;
use gpui::{App, Div, Entity, ParentElement};

pub fn attach_modal_overlays(
    root: Div,
    workspace_state: &Entity<WorkspaceState>,
    editor_state: &Entity<EditorState>,
    toast_state: &Entity<ToastState>,
    send_request_cb: impl 'static + Clone + Fn(&mut App),
    cx: &mut App,
) -> Div {
    let ws = workspace_state.read(cx);
    let show_env_modal = ws.show_env_modal;
    let show_cookies_modal = ws.show_cookies_modal;
    let show_settings_modal = ws.show_settings_modal;
    let show_scope_settings_modal = ws.scope_settings_target.is_some();
    let show_git_modal = ws.show_git_modal;
    let show_import_modal = ws.show_import_modal;
    let show_rename_workspace_modal = ws.show_rename_workspace_modal;
    let show_command_palette = ws.show_command_palette;

    let ws_entity = workspace_state.clone();
    let ed_entity = editor_state.clone();
    let toast_entity = toast_state.clone();

    root.when(show_env_modal, |this| {
        this.child(EnvironmentModal::new(ws_entity.clone()))
    })
    .when(show_cookies_modal, |this| {
        this.child(CookiesModal::new(ws_entity.clone()))
    })
    .when(show_settings_modal, |this| {
        this.child(SettingsModal::new(ws_entity.clone(), ed_entity.clone()))
    })
    .when(show_scope_settings_modal, |this| {
        this.child(ScopeSettingsModal::new(ws_entity.clone()))
    })
    .when(show_git_modal, |this| {
        this.child(GitModal::new(ws_entity.clone()))
    })
    .when(show_import_modal, |this| {
        this.child(ImportModal::new(ws_entity.clone(), toast_entity.clone()))
    })
    .when(show_rename_workspace_modal, |this| {
        this.child(RenameWorkspaceModal::new(ws_entity.clone()))
    })
    .when(show_command_palette, move |this| {
        let send_cb = send_request_cb.clone();
        this.child(CommandPalette::new(
            ws_entity.clone(),
            ed_entity.clone(),
            move |_, cx| {
                send_cb(cx);
            },
        ))
    })
    // Toast Notifications container bottom-right
    .child(ToastContainer::new(toast_entity))
}

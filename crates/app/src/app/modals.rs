use crate::components::*;
use crate::state::{
    EditorState, EnvironmentState, GitState, ModalState, PersistenceState, SidebarState,
    ToastState, WorkspaceDataState,
};
use gpui::prelude::FluentBuilder as _;
use gpui::{App, Div, Entity, ParentElement, Window};

#[allow(clippy::too_many_arguments)]
pub fn attach_modal_overlays(
    root: Div,
    workspace_state: &Entity<WorkspaceDataState>,
    env_state: &Entity<EnvironmentState>,
    persistence_state: &Entity<PersistenceState>,
    modal_state: &Entity<ModalState>,
    sidebar_state: &Entity<SidebarState>,
    git_state: &Entity<GitState>,
    editor_state: &Entity<EditorState>,
    toast_state: &Entity<ToastState>,
    send_request_cb: impl 'static + Clone + Fn(&mut Window, &mut App),
    cx: &mut App,
) -> Div {
    let modal = modal_state.read(cx);
    let show_env_modal = modal.show_env_modal;
    let show_cookies_modal = modal.show_cookies_modal;
    let show_settings_modal = modal.show_settings_modal;
    let show_scope_settings_modal = modal.scope_settings_target.is_some();
    let show_import_modal = modal.show_import_modal;
    let show_rename_workspace_modal = modal.show_rename_workspace_modal;
    let show_command_palette = modal.show_command_palette;

    let show_git_modal = git_state.read(cx).show_modal;

    let ws_entity = workspace_state.clone();
    let env_entity = env_state.clone();
    let per_entity = persistence_state.clone();
    let modal_entity = modal_state.clone();
    let sidebar_entity = sidebar_state.clone();
    let git_entity = git_state.clone();
    let ed_entity = editor_state.clone();
    let toast_entity = toast_state.clone();

    root.when(show_env_modal, |this| {
        this.child(EnvironmentModal::new(
            env_entity.clone(),
            modal_entity.clone(),
            sidebar_entity.clone(),
        ))
    })
    .when(show_cookies_modal, |this| {
        this.child(CookiesModal::new(
            ws_entity.clone(),
            modal_entity.clone(),
        ))
    })
    .when(show_settings_modal, |this| {
        this.child(SettingsModal::new(
            ws_entity.clone(),
            modal_entity.clone(),
            ed_entity.clone(),
        ))
    })
    .when(show_scope_settings_modal, |this| {
        this.child(ScopeSettingsModal::new(
            ws_entity.clone(),
            env_entity.clone(),
            modal_entity.clone(),
        ))
    })
    .when(show_git_modal, |this| {
        this.child(GitModal::new(git_entity.clone(), ws_entity.clone()))
    })
    .when(show_import_modal, |this| {
        this.child(ImportModal::new(
            ws_entity.clone(),
            modal_entity.clone(),
            toast_entity.clone(),
        ))
    })
    .when(show_rename_workspace_modal, |this| {
        this.child(RenameWorkspaceModal::new(
            ws_entity.clone(),
            sidebar_entity.clone(),
        ))
    })
    .when(show_command_palette, move |this| {
        let send_cb = send_request_cb.clone();
        this.child(CommandPalette::new(
            ws_entity.clone(),
            env_entity.clone(),
            per_entity.clone(),
            modal_entity.clone(),
            sidebar_entity.clone(),
            git_entity.clone(),
            ed_entity.clone(),
            send_cb,
        ))
    })
    // Toast Notifications container bottom-right
    .child(ToastContainer::new(toast_entity))
}

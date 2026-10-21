pub mod branch_modal;
pub mod commit_modal;
pub mod common;
pub mod diff_view;
pub mod history_modal;
pub mod remotes_modal;

pub use branch_modal::*;
pub use commit_modal::*;
pub use common::*;
pub use diff_view::*;
pub use history_modal::*;
pub use remotes_modal::*;

use crate::state::{GitActiveModal, WorkspaceState};
use gpui::{div, App, AppContext as _, Entity, IntoElement, RenderOnce, Window};

#[derive(IntoElement)]
pub struct GitModal {
    workspace_state: Entity<WorkspaceState>,
}

impl GitModal {
    pub fn new(workspace_state: Entity<WorkspaceState>) -> Self {
        Self { workspace_state }
    }
}

impl RenderOnce for GitModal {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let ws = self.workspace_state.read(cx);
        let active_modal = ws.active_git_modal;

        match active_modal {
            Some(GitActiveModal::Commit) => GitCommitModal::new(self.workspace_state.clone()).into_any_element(),
            Some(GitActiveModal::History) => GitHistoryModal::new(self.workspace_state.clone()).into_any_element(),
            Some(GitActiveModal::NewBranch) => GitBranchModal::new(self.workspace_state.clone()).into_any_element(),
            Some(GitActiveModal::Remotes) => GitRemotesModal::new(self.workspace_state.clone()).into_any_element(),
            None => div().into_any_element(),
        }
    }
}

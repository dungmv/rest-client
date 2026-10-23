use gpui::actions;

actions!(
    zest,
    [
        SendRequest,
        ToggleCommandPalette,
        ToggleSidebar,
        ToggleSettings,
        ToggleGitModal,
        OpenImportModal,
        NavigateBack,
        NavigateForward,
        DismissModal,
    ]
);

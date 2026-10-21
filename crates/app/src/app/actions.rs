use gpui::actions;

actions!(restclient, [
    SendRequest,
    ToggleCommandPalette,
    ToggleSidebar,
    ToggleSettings,
    ToggleGitModal,
    OpenImportModal,
    NavigateBack,
    NavigateForward,
    DismissModal,
]);

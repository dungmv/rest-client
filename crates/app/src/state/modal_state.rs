use crate::settings::AppSettings;
use gpui::Context;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScopeSettingsTarget {
    Workspace,
    Folder(String),
}

#[derive(Debug, Clone)]
pub struct ModalState {
    pub show_env_modal: bool,
    pub selected_env_modal_id: Option<String>,
    pub env_filter_query: String,
    pub var_filter_query: String,
    pub show_cookies_modal: bool,
    pub show_command_palette: bool,
    pub show_settings_modal: bool,
    pub scope_settings_target: Option<ScopeSettingsTarget>,
    pub show_rename_workspace_modal: bool,
    pub show_import_modal: bool,
    pub import_target_folder_rel: Option<String>,
    pub theme_preference: crate::theme::ThemePreference,
    pub split_layout_vertical: bool,
}

impl Default for ModalState {
    fn default() -> Self {
        Self {
            show_env_modal: false,
            selected_env_modal_id: None,
            env_filter_query: String::new(),
            var_filter_query: String::new(),
            show_cookies_modal: false,
            show_command_palette: false,
            show_settings_modal: false,
            scope_settings_target: None,
            show_rename_workspace_modal: false,
            show_import_modal: false,
            import_target_folder_rel: None,
            theme_preference: crate::theme::ThemePreference::System,
            split_layout_vertical: false,
        }
    }
}

impl ModalState {
    pub fn new() -> Self {
        let mut state = Self::default();
        let settings = AppSettings::load();
        state.theme_preference =
            crate::theme::ThemePreference::from_str_opt(settings.theme.as_deref());
        state
    }

    pub fn toggle_split_layout(&mut self, cx: &mut Context<Self>) {
        self.split_layout_vertical = !self.split_layout_vertical;
        cx.notify();
    }

    pub fn toggle_env_modal(&mut self, cx: &mut Context<Self>) {
        self.show_env_modal = !self.show_env_modal;
        cx.notify();
    }

    pub fn open_env_modal(&mut self, cx: &mut Context<Self>) {
        self.show_env_modal = true;
        cx.notify();
    }

    pub fn close_env_modal(&mut self, cx: &mut Context<Self>) {
        self.show_env_modal = false;
        cx.notify();
    }

    pub fn open_env_modal_with_id(&mut self, env_id: Option<String>, cx: &mut Context<Self>) {
        self.show_env_modal = true;
        self.selected_env_modal_id = env_id;
        self.var_filter_query.clear();
        cx.notify();
    }

    pub fn select_modal_environment(&mut self, env_id: Option<String>, cx: &mut Context<Self>) {
        self.selected_env_modal_id = env_id;
        self.var_filter_query.clear();
        cx.notify();
    }

    pub fn set_env_filter_query(&mut self, query: String, cx: &mut Context<Self>) {
        self.env_filter_query = query;
        cx.notify();
    }

    pub fn set_var_filter_query(&mut self, query: String, cx: &mut Context<Self>) {
        self.var_filter_query = query;
        cx.notify();
    }

    pub fn toggle_cookies_modal(&mut self, cx: &mut Context<Self>) {
        self.show_cookies_modal = !self.show_cookies_modal;
        cx.notify();
    }

    pub fn open_cookies_modal(&mut self, cx: &mut Context<Self>) {
        self.show_cookies_modal = true;
        cx.notify();
    }

    pub fn close_cookies_modal(&mut self, cx: &mut Context<Self>) {
        self.show_cookies_modal = false;
        cx.notify();
    }

    pub fn toggle_command_palette(&mut self, cx: &mut Context<Self>) {
        self.show_command_palette = !self.show_command_palette;
        cx.notify();
    }

    pub fn open_command_palette(&mut self, cx: &mut Context<Self>) {
        self.show_command_palette = true;
        cx.notify();
    }

    pub fn close_command_palette(&mut self, cx: &mut Context<Self>) {
        self.show_command_palette = false;
        cx.notify();
    }

    pub fn toggle_settings_modal(&mut self, cx: &mut Context<Self>) {
        self.show_settings_modal = !self.show_settings_modal;
        cx.notify();
    }

    pub fn open_settings_modal(&mut self, cx: &mut Context<Self>) {
        self.show_settings_modal = true;
        cx.notify();
    }

    pub fn close_settings_modal(&mut self, cx: &mut Context<Self>) {
        self.show_settings_modal = false;
        cx.notify();
    }

    pub fn open_scope_settings(&mut self, target: ScopeSettingsTarget, cx: &mut Context<Self>) {
        self.scope_settings_target = Some(target);
        cx.notify();
    }

    pub fn open_workspace_settings(&mut self, cx: &mut Context<Self>) {
        self.scope_settings_target = Some(ScopeSettingsTarget::Workspace);
        cx.notify();
    }

    pub fn open_folder_settings(&mut self, folder_id: String, cx: &mut Context<Self>) {
        self.scope_settings_target = Some(ScopeSettingsTarget::Folder(folder_id));
        cx.notify();
    }

    pub fn close_scope_settings(&mut self, cx: &mut Context<Self>) {
        self.scope_settings_target = None;
        cx.notify();
    }

    pub fn open_import_modal(&mut self, target_folder_rel: Option<String>, cx: &mut Context<Self>) {
        self.import_target_folder_rel = target_folder_rel;
        self.show_import_modal = true;
        cx.notify();
    }

    pub fn close_import_modal(&mut self, cx: &mut Context<Self>) {
        self.show_import_modal = false;
        self.import_target_folder_rel = None;
        cx.notify();
    }

    pub fn toggle_rename_workspace_modal(&mut self, cx: &mut Context<Self>) {
        self.show_rename_workspace_modal = !self.show_rename_workspace_modal;
        cx.notify();
    }

    pub fn set_theme_preference(
        &mut self,
        pref: crate::theme::ThemePreference,
        cx: &mut Context<Self>,
    ) {
        self.theme_preference = pref;
        let appearance = cx.window_appearance();
        let mode = crate::theme::resolve_theme_mode(pref, appearance);
        crate::theme::set_theme(pref, mode);

        let gpui_mode = match mode {
            crate::theme::ThemeMode::Dark => gpui_component::ThemeMode::Dark,
            crate::theme::ThemeMode::Light => gpui_component::ThemeMode::Light,
        };
        gpui_component::Theme::change(gpui_mode, None, cx);

        let mut settings = AppSettings::load();
        settings.theme = Some(pref.as_str().to_string());
        let _ = settings.save();

        cx.refresh_windows();
        cx.notify();
    }

    pub fn dismiss_top_modal(&mut self, cx: &mut Context<Self>) -> bool {
        if self.show_command_palette {
            self.show_command_palette = false;
            cx.notify();
            return true;
        }
        if self.show_rename_workspace_modal {
            self.show_rename_workspace_modal = false;
            cx.notify();
            return true;
        }
        if self.show_import_modal {
            self.show_import_modal = false;
            self.import_target_folder_rel = None;
            cx.notify();
            return true;
        }
        if self.show_cookies_modal {
            self.show_cookies_modal = false;
            cx.notify();
            return true;
        }
        if self.show_settings_modal {
            self.show_settings_modal = false;
            cx.notify();
            return true;
        }
        if self.scope_settings_target.is_some() {
            self.scope_settings_target = None;
            cx.notify();
            return true;
        }
        if self.show_env_modal {
            self.show_env_modal = false;
            cx.notify();
            return true;
        }
        false
    }
}

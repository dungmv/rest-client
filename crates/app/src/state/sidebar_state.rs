use super::WorkspaceDataState;
use gpui::{px, AppContext, Context, Entity, Pixels, Window};
use gpui_component::input::InputState;
use std::collections::HashSet;

pub struct SidebarState {
    pub filter_query: String,
    pub sidebar_collapsed: bool,
    pub sidebar_width: Pixels,
    pub collapsed_folders: HashSet<String>,
    pub collapsed_requests: HashSet<String>,
    pub renaming_request_id: Option<String>,
    pub rename_input: Option<Entity<InputState>>,
    pub rename_subscription: Option<gpui::Subscription>,
    pub renaming_folder_id: Option<String>,
    pub rename_folder_input: Option<Entity<InputState>>,
    pub rename_folder_subscription: Option<gpui::Subscription>,
    pub renaming_env_id: Option<String>,
    pub rename_env_input: Option<Entity<InputState>>,
    pub rename_env_subscription: Option<gpui::Subscription>,
    pub renaming_example: Option<(String, String)>,
    pub rename_example_input: Option<Entity<InputState>>,
    pub rename_example_subscription: Option<gpui::Subscription>,
    pub rename_workspace_input: Option<Entity<InputState>>,
    pub rename_workspace_subscription: Option<gpui::Subscription>,
    pub nav_history: Vec<String>,
    pub nav_index: usize,
}

impl Default for SidebarState {
    fn default() -> Self {
        Self {
            filter_query: String::new(),
            sidebar_collapsed: false,
            sidebar_width: px(260.),
            collapsed_folders: HashSet::new(),
            collapsed_requests: HashSet::new(),
            renaming_request_id: None,
            rename_input: None,
            rename_subscription: None,
            renaming_folder_id: None,
            rename_folder_input: None,
            rename_folder_subscription: None,
            renaming_env_id: None,
            rename_env_input: None,
            rename_env_subscription: None,
            renaming_example: None,
            rename_example_input: None,
            rename_example_subscription: None,
            rename_workspace_input: None,
            rename_workspace_subscription: None,
            nav_history: Vec::new(),
            nav_index: 0,
        }
    }
}

impl SidebarState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_filter_query(&mut self, query: String, cx: &mut Context<Self>) {
        self.filter_query = query;
        cx.notify();
    }

    pub fn toggle_sidebar(&mut self, cx: &mut Context<Self>) {
        self.sidebar_collapsed = !self.sidebar_collapsed;
        cx.notify();
    }

    pub fn set_sidebar_width(&mut self, width: Pixels, cx: &mut Context<Self>) {
        self.sidebar_width = width;
        cx.notify();
    }

    pub fn toggle_folder_collapsed(&mut self, folder_id: &str, cx: &mut Context<Self>) {
        if self.collapsed_folders.contains(folder_id) {
            self.collapsed_folders.remove(folder_id);
        } else {
            self.collapsed_folders.insert(folder_id.to_string());
        }
        cx.notify();
    }

    pub fn toggle_request_collapsed(&mut self, req_id: &str, cx: &mut Context<Self>) {
        if self.collapsed_requests.contains(req_id) {
            self.collapsed_requests.remove(req_id);
        } else {
            self.collapsed_requests.insert(req_id.to_string());
        }
        cx.notify();
    }

    pub fn record_navigation(&mut self, req_id: &str) {
        if self.nav_history.get(self.nav_index).map(|s| s.as_str()) == Some(req_id) {
            return;
        }
        if self.nav_index + 1 < self.nav_history.len() {
            self.nav_history.truncate(self.nav_index + 1);
        }
        self.nav_history.push(req_id.to_string());
        self.nav_index = self.nav_history.len() - 1;
    }

    pub fn navigate_back(&mut self) -> Option<String> {
        if self.nav_index > 0 {
            self.nav_index -= 1;
            self.nav_history.get(self.nav_index).cloned()
        } else {
            None
        }
    }

    pub fn navigate_forward(&mut self) -> Option<String> {
        if self.nav_index + 1 < self.nav_history.len() {
            self.nav_index += 1;
            self.nav_history.get(self.nav_index).cloned()
        } else {
            None
        }
    }

    pub fn cancel_renaming(&mut self, cx: &mut Context<Self>) {
        self.cancel_renaming_request(cx);
    }

    pub fn start_renaming_request(
        &mut self,
        req_id: String,
        initial_name: String,
        ws: Entity<WorkspaceDataState>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.cancel_any_renaming(cx);

        let input = cx.new(|cx| {
            let mut s = InputState::new(window, cx);
            s.set_value(initial_name, window, cx);
            s.select_all(window, cx);
            s.focus(window, cx);
            s
        });

        let r_id = req_id.clone();
        let sub = cx.subscribe(&input, move |this, input, event: &gpui_component::input::InputEvent, cx| {
            match event {
                gpui_component::input::InputEvent::PressEnter { .. } | gpui_component::input::InputEvent::Blur => {
                    let val = input.read(cx).value().to_string();
                    this.cancel_renaming_request(cx);
                    let trimmed = val.trim();
                    if !trimmed.is_empty() {
                        ws.update(cx, |ws, cx| {
                            ws.rename_request(&r_id, trimmed, cx);
                        });
                    }
                }
                _ => {}
            }
        });

        self.renaming_request_id = Some(req_id);
        self.rename_input = Some(input);
        self.rename_subscription = Some(sub);
        cx.notify();
    }

    pub fn start_renaming_folder(
        &mut self,
        folder_id: String,
        initial_name: String,
        ws: Entity<WorkspaceDataState>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.cancel_any_renaming(cx);

        let input = cx.new(|cx| {
            let mut s = InputState::new(window, cx);
            s.set_value(initial_name, window, cx);
            s.select_all(window, cx);
            s.focus(window, cx);
            s
        });

        let f_id = folder_id.clone();
        let sub = cx.subscribe(&input, move |this, input, event: &gpui_component::input::InputEvent, cx| {
            match event {
                gpui_component::input::InputEvent::PressEnter { .. } | gpui_component::input::InputEvent::Blur => {
                    let val = input.read(cx).value().to_string();
                    this.cancel_renaming_folder(cx);
                    let trimmed = val.trim();
                    if !trimmed.is_empty() {
                        ws.update(cx, |ws, cx| {
                            ws.rename_folder(&f_id, trimmed.to_string(), cx);
                        });
                    }
                }
                _ => {}
            }
        });

        self.renaming_folder_id = Some(folder_id);
        self.rename_folder_input = Some(input);
        self.rename_folder_subscription = Some(sub);
        cx.notify();
    }

    pub fn start_renaming_example(
        &mut self,
        req_id: String,
        example_name: String,
        ws: Entity<WorkspaceDataState>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.cancel_any_renaming(cx);

        let input = cx.new(|cx| {
            let mut s = InputState::new(window, cx);
            s.set_value(example_name.clone(), window, cx);
            s.select_all(window, cx);
            s.focus(window, cx);
            s
        });

        let r_id = req_id.clone();
        let old_ex_name = example_name.clone();
        let sub = cx.subscribe(&input, move |this, input, event: &gpui_component::input::InputEvent, cx| {
            match event {
                gpui_component::input::InputEvent::PressEnter { .. } | gpui_component::input::InputEvent::Blur => {
                    let val = input.read(cx).value().to_string();
                    this.cancel_renaming_example(cx);
                    let trimmed = val.trim();
                    if !trimmed.is_empty() && trimmed != old_ex_name {
                        ws.update(cx, |ws, cx| {
                            ws.rename_example(&r_id, &old_ex_name, trimmed, cx);
                        });
                    }
                }
                _ => {}
            }
        });

        self.renaming_example = Some((req_id, example_name));
        self.rename_example_input = Some(input);
        self.rename_example_subscription = Some(sub);
        cx.notify();
    }

    pub fn start_renaming_workspace(
        &mut self,
        curr_name: String,
        ws: Entity<WorkspaceDataState>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.cancel_any_renaming(cx);

        let input = cx.new(|cx| {
            let mut s = InputState::new(window, cx);
            s.set_value(curr_name, window, cx);
            s.select_all(window, cx);
            s.focus(window, cx);
            s
        });

        let sub = cx.subscribe(&input, move |this, input, event: &gpui_component::input::InputEvent, cx| {
            if let gpui_component::input::InputEvent::PressEnter { .. } = event {
                let val = input.read(cx).value().to_string();
                this.cancel_renaming_workspace(cx);
                let trimmed = val.trim();
                if !trimmed.is_empty() {
                    ws.update(cx, |ws, cx| {
                        ws.rename_workspace(trimmed.to_string(), cx);
                    });
                }
            }
        });

        self.rename_workspace_input = Some(input);
        self.rename_workspace_subscription = Some(sub);
        cx.notify();
    }

    pub fn start_renaming_environment(
        &mut self,
        env_id: String,
        initial_name: String,
        ws: Entity<WorkspaceDataState>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.cancel_any_renaming(cx);

        let input = cx.new(|cx| {
            let mut s = InputState::new(window, cx);
            s.set_value(initial_name, window, cx);
            s.select_all(window, cx);
            s.focus(window, cx);
            s
        });

        let e_id = env_id.clone();
        let sub = cx.subscribe(&input, move |this, input, event: &gpui_component::input::InputEvent, cx| {
            match event {
                gpui_component::input::InputEvent::PressEnter { .. } | gpui_component::input::InputEvent::Blur => {
                    let val = input.read(cx).value().to_string();
                    this.cancel_renaming_environment(cx);
                    let trimmed = val.trim();
                    if !trimmed.is_empty() {
                        ws.update(cx, |ws, cx| {
                            ws.rename_environment(&e_id, trimmed.to_string(), cx);
                        });
                    }
                }
                _ => {}
            }
        });

        self.renaming_env_id = Some(env_id);
        self.rename_env_input = Some(input);
        self.rename_env_subscription = Some(sub);
        cx.notify();
    }

    pub fn cancel_renaming_request(&mut self, cx: &mut Context<Self>) {
        if self.renaming_request_id.is_some() {
            self.renaming_request_id = None;
            self.rename_input = None;
            self.rename_subscription = None;
            cx.notify();
        }
    }

    pub fn cancel_renaming_folder(&mut self, cx: &mut Context<Self>) {
        if self.renaming_folder_id.is_some() {
            self.renaming_folder_id = None;
            self.rename_folder_input = None;
            self.rename_folder_subscription = None;
            cx.notify();
        }
    }

    pub fn cancel_renaming_environment(&mut self, cx: &mut Context<Self>) {
        if self.renaming_env_id.is_some() {
            self.renaming_env_id = None;
            self.rename_env_input = None;
            self.rename_env_subscription = None;
            cx.notify();
        }
    }

    pub fn cancel_renaming_example(&mut self, cx: &mut Context<Self>) {
        if self.renaming_example.is_some() {
            self.renaming_example = None;
            self.rename_example_input = None;
            self.rename_example_subscription = None;
            cx.notify();
        }
    }

    pub fn cancel_renaming_workspace(&mut self, cx: &mut Context<Self>) {
        if self.rename_workspace_input.is_some() {
            self.rename_workspace_input = None;
            self.rename_workspace_subscription = None;
            cx.notify();
        }
    }

    pub fn cancel_any_renaming(&mut self, cx: &mut Context<Self>) -> bool {
        if self.renaming_request_id.is_some() {
            self.cancel_renaming_request(cx);
            return true;
        }
        if self.renaming_folder_id.is_some() {
            self.cancel_renaming_folder(cx);
            return true;
        }
        if self.renaming_env_id.is_some() {
            self.cancel_renaming_environment(cx);
            return true;
        }
        if self.renaming_example.is_some() {
            self.cancel_renaming_example(cx);
            return true;
        }
        if self.rename_workspace_input.is_some() {
            self.cancel_renaming_workspace(cx);
            return true;
        }
        false
    }
}

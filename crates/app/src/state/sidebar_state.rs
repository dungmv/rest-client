use super::{EnvironmentState, WorkspaceDataState};
use gpui::{px, AppContext, Context, Entity, Pixels, Window};
use gpui_component::input::InputState;
use std::collections::HashSet;

pub struct RenamingState<T> {
    pub id: T,
    pub input: Entity<InputState>,
    pub subscription: gpui::Subscription,
}

pub struct SidebarState {
    pub filter_query: String,
    pub sidebar_collapsed: bool,
    pub sidebar_width: Pixels,
    pub collapsed_folders: HashSet<String>,
    pub collapsed_requests: HashSet<String>,
    pub renaming_request: Option<RenamingState<String>>,
    pub renaming_folder: Option<RenamingState<String>>,
    pub renaming_env: Option<RenamingState<String>>,
    pub renaming_example: Option<RenamingState<(String, String)>>,
    pub renaming_workspace: Option<RenamingState<String>>,
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
            renaming_request: None,
            renaming_folder: None,
            renaming_env: None,
            renaming_example: None,
            renaming_workspace: None,
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
        let sub = cx.subscribe(
            &input,
            move |this, input, event: &gpui_component::input::InputEvent, cx| match event {
                gpui_component::input::InputEvent::PressEnter { .. }
                | gpui_component::input::InputEvent::Blur => {
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
            },
        );

        self.renaming_request = Some(RenamingState {
            id: req_id,
            input,
            subscription: sub,
        });
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
        let sub = cx.subscribe(
            &input,
            move |this, input, event: &gpui_component::input::InputEvent, cx| match event {
                gpui_component::input::InputEvent::PressEnter { .. }
                | gpui_component::input::InputEvent::Blur => {
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
            },
        );

        self.renaming_folder = Some(RenamingState {
            id: folder_id,
            input,
            subscription: sub,
        });
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
        let sub = cx.subscribe(
            &input,
            move |this, input, event: &gpui_component::input::InputEvent, cx| match event {
                gpui_component::input::InputEvent::PressEnter { .. }
                | gpui_component::input::InputEvent::Blur => {
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
            },
        );

        self.renaming_example = Some(RenamingState {
            id: (req_id, example_name),
            input,
            subscription: sub,
        });
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
            s.set_value(curr_name.clone(), window, cx);
            s.select_all(window, cx);
            s.focus(window, cx);
            s
        });

        let sub = cx.subscribe(
            &input,
            move |this, input, event: &gpui_component::input::InputEvent, cx| {
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
            },
        );

        self.renaming_workspace = Some(RenamingState {
            id: curr_name,
            input,
            subscription: sub,
        });
        cx.notify();
    }

    pub fn start_renaming_environment(
        &mut self,
        env_id: String,
        initial_name: String,
        env_state: Entity<EnvironmentState>,
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
        let sub = cx.subscribe(
            &input,
            move |this, input, event: &gpui_component::input::InputEvent, cx| match event {
                gpui_component::input::InputEvent::PressEnter { .. }
                | gpui_component::input::InputEvent::Blur => {
                    let val = input.read(cx).value().to_string();
                    this.cancel_renaming_environment(cx);
                    let trimmed = val.trim();
                    if !trimmed.is_empty() {
                        env_state.update(cx, |env, cx| {
                            env.rename_environment(&e_id, trimmed.to_string(), cx);
                        });
                    }
                }
                _ => {}
            },
        );

        self.renaming_env = Some(RenamingState {
            id: env_id,
            input,
            subscription: sub,
        });
        cx.notify();
    }

    pub fn renaming_request_id(&self) -> Option<&str> {
        self.renaming_request.as_ref().map(|r| r.id.as_str())
    }

    pub fn rename_input(&self) -> Option<&Entity<InputState>> {
        self.renaming_request.as_ref().map(|r| &r.input)
    }

    pub fn renaming_folder_id(&self) -> Option<&str> {
        self.renaming_folder.as_ref().map(|r| r.id.as_str())
    }

    pub fn rename_folder_input(&self) -> Option<&Entity<InputState>> {
        self.renaming_folder.as_ref().map(|r| &r.input)
    }

    pub fn renaming_env_id(&self) -> Option<&str> {
        self.renaming_env.as_ref().map(|r| r.id.as_str())
    }

    pub fn rename_env_input(&self) -> Option<&Entity<InputState>> {
        self.renaming_env.as_ref().map(|r| &r.input)
    }

    pub fn renaming_example(&self) -> Option<&(String, String)> {
        self.renaming_example.as_ref().map(|r| &r.id)
    }

    pub fn rename_example_input(&self) -> Option<&Entity<InputState>> {
        self.renaming_example.as_ref().map(|r| &r.input)
    }

    pub fn rename_workspace_input(&self) -> Option<&Entity<InputState>> {
        self.renaming_workspace.as_ref().map(|r| &r.input)
    }

    pub fn cancel_renaming_request(&mut self, cx: &mut Context<Self>) {
        if self.renaming_request.take().is_some() {
            cx.notify();
        }
    }

    pub fn cancel_renaming_folder(&mut self, cx: &mut Context<Self>) {
        if self.renaming_folder.take().is_some() {
            cx.notify();
        }
    }

    pub fn cancel_renaming_environment(&mut self, cx: &mut Context<Self>) {
        if self.renaming_env.take().is_some() {
            cx.notify();
        }
    }

    pub fn cancel_renaming_example(&mut self, cx: &mut Context<Self>) {
        if self.renaming_example.take().is_some() {
            cx.notify();
        }
    }

    pub fn cancel_renaming_workspace(&mut self, cx: &mut Context<Self>) {
        if self.renaming_workspace.take().is_some() {
            cx.notify();
        }
    }

    pub fn cancel_any_renaming(&mut self, cx: &mut Context<Self>) -> bool {
        let mut cancelled = false;
        if self.renaming_request.take().is_some() {
            cancelled = true;
        }
        if self.renaming_folder.take().is_some() {
            cancelled = true;
        }
        if self.renaming_env.take().is_some() {
            cancelled = true;
        }
        if self.renaming_example.take().is_some() {
            cancelled = true;
        }
        if self.renaming_workspace.take().is_some() {
            cancelled = true;
        }
        if cancelled {
            cx.notify();
        }
        cancelled
    }
}

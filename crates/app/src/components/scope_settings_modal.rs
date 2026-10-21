use crate::components::kv_table::KvTable;
use crate::state::{ScopeSettingsTarget, WorkspaceState};
use crate::theme::theme;
use gpui::prelude::FluentBuilder as _;
use gpui::{
    div, px, App, AppContext as _, Entity, InteractiveElement as _, IntoElement, MouseButton,
    ParentElement, RenderOnce, StatefulInteractiveElement as _, Styled, Window,
};
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::menu::{DropdownMenu as _, PopupMenuItem};
use gpui_component::scroll::ScrollableElement as _;
use gpui_component::Icon;
use gpui_kit::assets::IconName;
use restclient_core::models::{Auth, KeyValuePair};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ScopeSettingsTab {
    #[default]
    Headers,
    Auth,
}

#[derive(IntoElement)]
pub struct ScopeSettingsModal {
    workspace_state: Entity<WorkspaceState>,
}

impl ScopeSettingsModal {
    pub fn new(workspace_state: Entity<WorkspaceState>) -> Self {
        Self { workspace_state }
    }
}

impl RenderOnce for ScopeSettingsModal {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = theme();
        let ws_clone = self.workspace_state.clone();
        let target = match &ws_clone.read(cx).scope_settings_target {
            Some(target) => target.clone(),
            None => return div().into_any_element(),
        };

        // Scope details (name, headers, auth)
        let (title, folder_id_opt, current_headers, current_auth) = {
            let ws = ws_clone.read(cx);
            match &target {
                ScopeSettingsTarget::Workspace => {
                    let name = ws
                        .active_workspace
                        .as_ref()
                        .map(|w| w.name.clone())
                        .unwrap_or_else(|| "Workspace".to_string());
                    let headers = ws
                        .active_workspace
                        .as_ref()
                        .map(|w| w.headers.clone())
                        .unwrap_or_default();
                    let auth = ws
                        .active_workspace
                        .as_ref()
                        .map(|w| w.auth.clone())
                        .unwrap_or(Auth::None);
                    (format!("Workspace Settings: {}", name), None, headers, auth)
                }
                ScopeSettingsTarget::Folder(f_id) => {
                    let folder = ws.folders.iter().find(|f| &f.id == f_id).cloned();
                    let name = folder.as_ref().map(|f| f.name.clone()).unwrap_or_else(|| f_id.clone());
                    let headers = folder.as_ref().map(|f| f.headers.clone()).unwrap_or_default();
                    let auth = folder.as_ref().map(|f| f.auth.clone()).unwrap_or(Auth::Inherit);
                    (format!("Folder Settings: {}", name), Some(f_id.clone()), headers, auth)
                }
            }
        };

        // Tab state: Headers vs Auth
        let tab_state = window.use_state(cx, |_, _| ScopeSettingsTab::Headers);
        let active_tab = *tab_state.read(cx);

        let is_folder = folder_id_opt.is_some();
        let headers_count = current_headers.len();

        let auth_label = match &current_auth {
            Auth::Inherit => "Inherit",
            Auth::None => "None",
            Auth::Bearer { .. } => "Bearer Token",
            Auth::Basic { .. } => "Basic Auth",
            Auth::ApiKey { .. } => "API Key",
        };

        let is_inherit = matches!(current_auth, Auth::Inherit);
        let is_none = matches!(current_auth, Auth::None);
        let is_bearer = matches!(current_auth, Auth::Bearer { .. });
        let is_basic = matches!(current_auth, Auth::Basic { .. });
        let is_apikey = matches!(current_auth, Auth::ApiKey { .. });

        // Input state holders for Auth
        let bearer_token_holder = window.use_keyed_state(
            gpui::SharedString::from(format!("scope-auth-bearer-{:?}", target)),
            cx,
            |window, cx| {
                let init_val = match &current_auth {
                    Auth::Bearer { token } => token.clone(),
                    _ => String::new(),
                };
                let input = cx.new(|cx| InputState::new(window, cx).placeholder("Token or {{token}}"));
                input.update(cx, |s, cx| s.set_value(init_val, window, cx));

                let inp = input.clone();
                let ws_s = ws_clone.clone();
                let tgt = target.clone();
                let sub = cx.subscribe_in(&input, window, move |_, _, ev, _, cx| {
                    if let InputEvent::Change = ev {
                        let token = inp.read(cx).value().to_string();
                        let new_auth = Auth::Bearer { token };
                        match &tgt {
                            ScopeSettingsTarget::Workspace => {
                                let cur_h = ws_s.read(cx).active_workspace.as_ref().map(|w| w.headers.clone()).unwrap_or_default();
                                ws_s.update(cx, |state, cx| state.save_workspace_scope_settings(cur_h, new_auth, cx));
                            }
                            ScopeSettingsTarget::Folder(fid) => {
                                let cur_h = ws_s.read(cx).folders.iter().find(|f| &f.id == fid).map(|f| f.headers.clone()).unwrap_or_default();
                                ws_s.update(cx, |state, cx| state.save_folder_scope_settings(fid, cur_h, new_auth, cx));
                            }
                        }
                    }
                });
                (input, sub)
            },
        );
        let bearer_input = bearer_token_holder.read(cx).0.clone();

        let basic_user_holder = window.use_keyed_state(
            gpui::SharedString::from(format!("scope-auth-basic-u-{:?}", target)),
            cx,
            |window, cx| {
                let init_val = match &current_auth {
                    Auth::Basic { username, .. } => username.clone(),
                    _ => String::new(),
                };
                let input = cx.new(|cx| InputState::new(window, cx).placeholder("Username"));
                input.update(cx, |s, cx| s.set_value(init_val, window, cx));

                let inp = input.clone();
                let ws_s = ws_clone.clone();
                let tgt = target.clone();
                let sub = cx.subscribe_in(&input, window, move |_, _, ev, _, cx| {
                    if let InputEvent::Change = ev {
                        let u = inp.read(cx).value().to_string();
                        let p = match &tgt {
                            ScopeSettingsTarget::Workspace => match &ws_s.read(cx).workspace_meta.auth {
                                restclient_core::storage::models::RequestAuth::Basic { password, .. } => password.clone(),
                                _ => String::new(),
                            },
                            ScopeSettingsTarget::Folder(fid) => match ws_s.read(cx).folders.iter().find(|f| &f.id == fid).map(|f| &f.auth) {
                                Some(Auth::Basic { password, .. }) => password.clone(),
                                _ => String::new(),
                            },
                        };
                        let new_auth = Auth::Basic { username: u, password: p };
                        match &tgt {
                            ScopeSettingsTarget::Workspace => {
                                let cur_h = ws_s.read(cx).active_workspace.as_ref().map(|w| w.headers.clone()).unwrap_or_default();
                                ws_s.update(cx, |state, cx| state.save_workspace_scope_settings(cur_h, new_auth, cx));
                            }
                            ScopeSettingsTarget::Folder(fid) => {
                                let cur_h = ws_s.read(cx).folders.iter().find(|f| &f.id == fid).map(|f| f.headers.clone()).unwrap_or_default();
                                ws_s.update(cx, |state, cx| state.save_folder_scope_settings(fid, cur_h, new_auth, cx));
                            }
                        }
                    }
                });
                (input, sub)
            },
        );
        let basic_user_input = basic_user_holder.read(cx).0.clone();

        let basic_pass_holder = window.use_keyed_state(
            gpui::SharedString::from(format!("scope-auth-basic-p-{:?}", target)),
            cx,
            |window, cx| {
                let init_val = match &current_auth {
                    Auth::Basic { password, .. } => password.clone(),
                    _ => String::new(),
                };
                let input = cx.new(|cx| InputState::new(window, cx).placeholder("Password"));
                input.update(cx, |s, cx| s.set_value(init_val, window, cx));

                let inp = input.clone();
                let ws_s = ws_clone.clone();
                let tgt = target.clone();
                let sub = cx.subscribe_in(&input, window, move |_, _, ev, _, cx| {
                    if let InputEvent::Change = ev {
                        let p = inp.read(cx).value().to_string();
                        let u = match &tgt {
                            ScopeSettingsTarget::Workspace => match &ws_s.read(cx).workspace_meta.auth {
                                restclient_core::storage::models::RequestAuth::Basic { username, .. } => username.clone(),
                                _ => String::new(),
                            },
                            ScopeSettingsTarget::Folder(fid) => match ws_s.read(cx).folders.iter().find(|f| &f.id == fid).map(|f| &f.auth) {
                                Some(Auth::Basic { username, .. }) => username.clone(),
                                _ => String::new(),
                            },
                        };
                        let new_auth = Auth::Basic { username: u, password: p };
                        match &tgt {
                            ScopeSettingsTarget::Workspace => {
                                let cur_h = ws_s.read(cx).active_workspace.as_ref().map(|w| w.headers.clone()).unwrap_or_default();
                                ws_s.update(cx, |state, cx| state.save_workspace_scope_settings(cur_h, new_auth, cx));
                            }
                            ScopeSettingsTarget::Folder(fid) => {
                                let cur_h = ws_s.read(cx).folders.iter().find(|f| &f.id == fid).map(|f| f.headers.clone()).unwrap_or_default();
                                ws_s.update(cx, |state, cx| state.save_folder_scope_settings(fid, cur_h, new_auth, cx));
                            }
                        }
                    }
                });
                (input, sub)
            },
        );
        let basic_pass_input = basic_pass_holder.read(cx).0.clone();

        let api_key_holder = window.use_keyed_state(
            gpui::SharedString::from(format!("scope-auth-api-key-{:?}", target)),
            cx,
            |window, cx| {
                let (init_k, _) = match &current_auth {
                    Auth::ApiKey { key, value, .. } => (key.clone(), value.clone()),
                    _ => ("X-API-Key".to_string(), String::new()),
                };
                let input = cx.new(|cx| InputState::new(window, cx).placeholder("Key"));
                input.update(cx, |s, cx| s.set_value(init_k, window, cx));

                let inp = input.clone();
                let ws_s = ws_clone.clone();
                let tgt = target.clone();
                let sub = cx.subscribe_in(&input, window, move |_, _, ev, _, cx| {
                    if let InputEvent::Change = ev {
                        let k = inp.read(cx).value().to_string();
                        let (v, in_h) = match &tgt {
                            ScopeSettingsTarget::Workspace => match &ws_s.read(cx).workspace_meta.auth {
                                restclient_core::storage::models::RequestAuth::ApiKey { value, in_header, .. } => (value.clone(), *in_header),
                                _ => (String::new(), true),
                            },
                            ScopeSettingsTarget::Folder(fid) => match ws_s.read(cx).folders.iter().find(|f| &f.id == fid).map(|f| &f.auth) {
                                Some(Auth::ApiKey { value, in_header, .. }) => (value.clone(), *in_header),
                                _ => (String::new(), true),
                            },
                        };
                        let new_auth = Auth::ApiKey { key: k, value: v, in_header: in_h };
                        match &tgt {
                            ScopeSettingsTarget::Workspace => {
                                let cur_h = ws_s.read(cx).active_workspace.as_ref().map(|w| w.headers.clone()).unwrap_or_default();
                                ws_s.update(cx, |state, cx| state.save_workspace_scope_settings(cur_h, new_auth, cx));
                            }
                            ScopeSettingsTarget::Folder(fid) => {
                                let cur_h = ws_s.read(cx).folders.iter().find(|f| &f.id == fid).map(|f| f.headers.clone()).unwrap_or_default();
                                ws_s.update(cx, |state, cx| state.save_folder_scope_settings(fid, cur_h, new_auth, cx));
                            }
                        }
                    }
                });
                (input, sub)
            },
        );
        let api_key_input = api_key_holder.read(cx).0.clone();

        let api_val_holder = window.use_keyed_state(
            gpui::SharedString::from(format!("scope-auth-api-val-{:?}", target)),
            cx,
            |window, cx| {
                let (_, init_v) = match &current_auth {
                    Auth::ApiKey { key, value, .. } => (key.clone(), value.clone()),
                    _ => ("X-API-Key".to_string(), String::new()),
                };
                let input = cx.new(|cx| InputState::new(window, cx).placeholder("Value"));
                input.update(cx, |s, cx| s.set_value(init_v, window, cx));

                let inp = input.clone();
                let ws_s = ws_clone.clone();
                let tgt = target.clone();
                let sub = cx.subscribe_in(&input, window, move |_, _, ev, _, cx| {
                    if let InputEvent::Change = ev {
                        let v = inp.read(cx).value().to_string();
                        let (k, in_h) = match &tgt {
                            ScopeSettingsTarget::Workspace => match &ws_s.read(cx).workspace_meta.auth {
                                restclient_core::storage::models::RequestAuth::ApiKey { key, in_header, .. } => (key.clone(), *in_header),
                                _ => ("X-API-Key".to_string(), true),
                            },
                            ScopeSettingsTarget::Folder(fid) => match ws_s.read(cx).folders.iter().find(|f| &f.id == fid).map(|f| &f.auth) {
                                Some(Auth::ApiKey { key, in_header, .. }) => (key.clone(), *in_header),
                                _ => ("X-API-Key".to_string(), true),
                            },
                        };
                        let new_auth = Auth::ApiKey { key: k, value: v, in_header: in_h };
                        match &tgt {
                            ScopeSettingsTarget::Workspace => {
                                let cur_h = ws_s.read(cx).active_workspace.as_ref().map(|w| w.headers.clone()).unwrap_or_default();
                                ws_s.update(cx, |state, cx| state.save_workspace_scope_settings(cur_h, new_auth, cx));
                            }
                            ScopeSettingsTarget::Folder(fid) => {
                                let cur_h = ws_s.read(cx).folders.iter().find(|f| &f.id == fid).map(|f| f.headers.clone()).unwrap_or_default();
                                ws_s.update(cx, |state, cx| state.save_folder_scope_settings(fid, cur_h, new_auth, cx));
                            }
                        }
                    }
                });
                (input, sub)
            },
        );
        let api_val_input = api_val_holder.read(cx).0.clone();

        // Render modal
        div()
            .id("scope-settings-modal-overlay")
            .absolute()
            .inset_0()
            .bg(gpui::Hsla { a: 0.6, ..t.bg_app })
            .flex()
            .items_center()
            .justify_center()
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_click({
                let ws_s = ws_clone.clone();
                move |_, _, cx| {
                    ws_s.update(cx, |state, cx| state.close_scope_settings(cx));
                }
            })
            .child(
                div()
                    .id("scope-settings-modal-dialog")
                    .occlude()
                    .w(px(640.))
                    .max_h(px(580.))
                    .min_h(px(400.))
                    .rounded(px(8.))
                    .bg(t.bg_card)
                    .border_1()
                    .border_color(t.border)
                    .shadow_2xl()
                    .flex()
                    .flex_col()
                    .overflow_hidden()
                    .on_click(|_, _, cx| cx.stop_propagation())
                    // Header
                    .child(
                        div()
                            .h(px(46.))
                            .px(px(16.))
                            .border_b_1()
                            .border_color(t.border)
                            .bg(t.bg_titlebar)
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(8.))
                                    .child(Icon::new(IconName::Settings).size(px(18.)))
                                    .child(
                                        div()
                                            .text_sm()
                                            .font_weight(gpui::FontWeight::BOLD)
                                            .text_color(t.text_primary)
                                            .child(title),
                                    ),
                            )
                            .child({
                                let ws_s = ws_clone.clone();
                                div()
                                    .id("close-scope-settings-modal")
                                    .cursor_pointer()
                                    .px(px(6.))
                                    .py(px(2.))
                                    .rounded(px(4.))
                                    .text_color(t.text_muted)
                                    .hover(|s| s.bg(t.bg_hover).text_color(t.text_primary))
                                    .on_click(move |_, _, cx| {
                                        ws_s.update(cx, |state, cx| state.close_scope_settings(cx));
                                    })
                                    .child("✕")
                            }),
                    )
                    // Tab Bar
                    .child(
                        div()
                            .px(px(16.))
                            .pt(px(8.))
                            .border_b_1()
                            .border_color(t.border)
                            .flex()
                            .items_center()
                            .gap(px(16.))
                            .child({
                                let tab = tab_state.clone();
                                let is_active = active_tab == ScopeSettingsTab::Headers;
                                div()
                                    .id("tab-scope-headers")
                                    .cursor_pointer()
                                    .pb(px(8.))
                                    .border_b_2()
                                    .border_color(if is_active { t.border_focus } else { gpui::transparent_black() })
                                    .text_xs()
                                    .font_weight(if is_active { gpui::FontWeight::BOLD } else { gpui::FontWeight::NORMAL })
                                    .text_color(if is_active { t.text_primary } else { t.text_muted })
                                    .on_click(move |_, window, cx| {
                                        tab.update(cx, |state, _| *state = ScopeSettingsTab::Headers);
                                        window.refresh();
                                    })
                                    .child(format!("Headers ({})", headers_count))
                            })
                            .child({
                                let tab = tab_state.clone();
                                let is_active = active_tab == ScopeSettingsTab::Auth;
                                div()
                                    .id("tab-scope-auth")
                                    .cursor_pointer()
                                    .pb(px(8.))
                                    .border_b_2()
                                    .border_color(if is_active { t.border_focus } else { gpui::transparent_black() })
                                    .text_xs()
                                    .font_weight(if is_active { gpui::FontWeight::BOLD } else { gpui::FontWeight::NORMAL })
                                    .text_color(if is_active { t.text_primary } else { t.text_muted })
                                    .on_click(move |_, window, cx| {
                                        tab.update(cx, |state, _| *state = ScopeSettingsTab::Auth);
                                        window.refresh();
                                    })
                                    .child(format!("Auth ({})", auth_label))
                            }),
                    )
                    // Tab Body
                    .child(
                        div()
                            .flex_1()
                            .p(px(16.))
                            .overflow_y_scrollbar()
                            .child(match active_tab {
                                ScopeSettingsTab::Headers => {
                                    let ws_up = ws_clone.clone();
                                    let tgt = target.clone();
                                    let pairs = current_headers.clone();

                                    let on_update = {
                                        let ws_s = ws_up.clone();
                                        let tgt = tgt.clone();
                                        move |idx: usize, key: String, val: String, enabled: bool, _window: &mut Window, cx: &mut App| {
                                            let mut cur_headers = match &tgt {
                                                ScopeSettingsTarget::Workspace => {
                                                    ws_s.read(cx).active_workspace.as_ref().map(|w| w.headers.clone()).unwrap_or_default()
                                                }
                                                ScopeSettingsTarget::Folder(fid) => {
                                                    ws_s.read(cx).folders.iter().find(|f| &f.id == fid).map(|f| f.headers.clone()).unwrap_or_default()
                                                }
                                            };
                                            if idx < cur_headers.len() {
                                                cur_headers[idx].key = key;
                                                cur_headers[idx].value = val;
                                                cur_headers[idx].enabled = enabled;
                                                match &tgt {
                                                    ScopeSettingsTarget::Workspace => {
                                                        let a = ws_s.read(cx).active_workspace.as_ref().map(|w| w.auth.clone()).unwrap_or(Auth::None);
                                                        ws_s.update(cx, |state, cx| state.save_workspace_scope_settings(cur_headers, a, cx));
                                                    }
                                                    ScopeSettingsTarget::Folder(fid) => {
                                                        let a = ws_s.read(cx).folders.iter().find(|f| &f.id == fid).map(|f| f.auth.clone()).unwrap_or(Auth::Inherit);
                                                        ws_s.update(cx, |state, cx| state.save_folder_scope_settings(fid, cur_headers, a, cx));
                                                    }
                                                }
                                            }
                                        }
                                    };

                                    let on_add = {
                                        let ws_s = ws_up.clone();
                                        let tgt = tgt.clone();
                                        move |_window: &mut Window, cx: &mut App| {
                                            let mut cur_headers = match &tgt {
                                                ScopeSettingsTarget::Workspace => {
                                                    ws_s.read(cx).active_workspace.as_ref().map(|w| w.headers.clone()).unwrap_or_default()
                                                }
                                                ScopeSettingsTarget::Folder(fid) => {
                                                    ws_s.read(cx).folders.iter().find(|f| &f.id == fid).map(|f| f.headers.clone()).unwrap_or_default()
                                                }
                                            };
                                            cur_headers.push(KeyValuePair::new("", ""));
                                            match &tgt {
                                                ScopeSettingsTarget::Workspace => {
                                                    let a = ws_s.read(cx).active_workspace.as_ref().map(|w| w.auth.clone()).unwrap_or(Auth::None);
                                                    ws_s.update(cx, |state, cx| state.save_workspace_scope_settings(cur_headers, a, cx));
                                                }
                                                ScopeSettingsTarget::Folder(fid) => {
                                                    let a = ws_s.read(cx).folders.iter().find(|f| &f.id == fid).map(|f| f.auth.clone()).unwrap_or(Auth::Inherit);
                                                    ws_s.update(cx, |state, cx| state.save_folder_scope_settings(fid, cur_headers, a, cx));
                                                }
                                            }
                                        }
                                    };

                                    let on_remove = {
                                        let ws_s = ws_up.clone();
                                        let tgt = tgt.clone();
                                        move |idx: usize, _window: &mut Window, cx: &mut App| {
                                            let mut cur_headers = match &tgt {
                                                ScopeSettingsTarget::Workspace => {
                                                    ws_s.read(cx).active_workspace.as_ref().map(|w| w.headers.clone()).unwrap_or_default()
                                                }
                                                ScopeSettingsTarget::Folder(fid) => {
                                                    ws_s.read(cx).folders.iter().find(|f| &f.id == fid).map(|f| f.headers.clone()).unwrap_or_default()
                                                }
                                            };
                                            if idx < cur_headers.len() {
                                                cur_headers.remove(idx);
                                                match &tgt {
                                                    ScopeSettingsTarget::Workspace => {
                                                        let a = ws_s.read(cx).active_workspace.as_ref().map(|w| w.auth.clone()).unwrap_or(Auth::None);
                                                        ws_s.update(cx, |state, cx| state.save_workspace_scope_settings(cur_headers, a, cx));
                                                    }
                                                    ScopeSettingsTarget::Folder(fid) => {
                                                        let a = ws_s.read(cx).folders.iter().find(|f| &f.id == fid).map(|f| f.auth.clone()).unwrap_or(Auth::Inherit);
                                                        ws_s.update(cx, |state, cx| state.save_folder_scope_settings(fid, cur_headers, a, cx));
                                                    }
                                                }
                                            }
                                        }
                                    };

                                    div()
                                        .flex()
                                        .flex_col()
                                        .gap(px(10.))
                                        .child(
                                            div()
                                                .text_xs()
                                                .text_color(t.text_muted)
                                                .child(if is_folder {
                                                    "Headers configured here apply to all requests in this folder (and subfolders)."
                                                } else {
                                                    "Headers configured here apply to all requests in this workspace."
                                                }),
                                        )
                                        .child(
                                            KvTable::new("scope-headers-table", pairs, on_update, on_add, on_remove)
                                                .workspace_state(ws_clone.clone()),
                                        )
                                        .into_any_element()
                                }
                                ScopeSettingsTab::Auth => {
                                    let ws_s = ws_clone.clone();
                                    let tgt = target.clone();

                                    div()
                                        .flex()
                                        .flex_col()
                                        .gap(px(14.))
                                        .child(
                                            div()
                                                .flex()
                                                .items_center()
                                                .gap(px(10.))
                                                .child(
                                                    div()
                                                        .w(px(70.))
                                                        .text_xs()
                                                        .font_weight(gpui::FontWeight::SEMIBOLD)
                                                        .text_color(t.text_secondary)
                                                        .child("Type:"),
                                                )
                                                .child({
                                                    let ws_drop = ws_s.clone();
                                                    let tgt_drop = tgt.clone();

                                                    Button::new("btn-scope-auth-type-dropdown")
                                                        .ghost()
                                                        .compact()
                                                        .px(px(8.))
                                                        .py(px(4.))
                                                        .h(px(28.))
                                                        .rounded(px(4.))
                                                        .bg(t.bg_card)
                                                        .border_1()
                                                        .border_color(t.border)
                                                        .child(
                                                            div()
                                                                .text_xs()
                                                                .font_weight(gpui::FontWeight::MEDIUM)
                                                                .text_color(t.text_primary)
                                                                .child(auth_label),
                                                        )
                                                        .dropdown_menu(move |menu, _window, _cx| {
                                                            let mut m = menu;
                                                            if is_folder {
                                                                let ws_inh = ws_drop.clone();
                                                                let tgt_inh = tgt_drop.clone();
                                                                m = m.item(
                                                                    PopupMenuItem::new("Inherit")
                                                                        .checked(is_inherit)
                                                                        .on_click(move |_, _, cx| {
                                                                            if let ScopeSettingsTarget::Folder(fid) = &tgt_inh {
                                                                                let cur_h = ws_inh.read(cx).folders.iter().find(|f| &f.id == fid).map(|f| f.headers.clone()).unwrap_or_default();
                                                                                ws_inh.update(cx, |s, cx| s.save_folder_scope_settings(fid, cur_h, Auth::Inherit, cx));
                                                                            }
                                                                        }),
                                                                );
                                                            }

                                                            let ws_none = ws_drop.clone();
                                                            let tgt_none = tgt_drop.clone();
                                                            let ws_bearer = ws_drop.clone();
                                                            let tgt_bearer = tgt_drop.clone();
                                                            let ws_basic = ws_drop.clone();
                                                            let tgt_basic = tgt_drop.clone();
                                                            let ws_api = ws_drop.clone();
                                                            let tgt_api = tgt_drop.clone();

                                                            m.item(
                                                                PopupMenuItem::new("None")
                                                                    .checked(is_none)
                                                                    .on_click(move |_, _, cx| {
                                                                        match &tgt_none {
                                                                            ScopeSettingsTarget::Workspace => {
                                                                                let cur_h = ws_none.read(cx).active_workspace.as_ref().map(|w| w.headers.clone()).unwrap_or_default();
                                                                                ws_none.update(cx, |s, cx| s.save_workspace_scope_settings(cur_h, Auth::None, cx));
                                                                            }
                                                                            ScopeSettingsTarget::Folder(fid) => {
                                                                                let cur_h = ws_none.read(cx).folders.iter().find(|f| &f.id == fid).map(|f| f.headers.clone()).unwrap_or_default();
                                                                                ws_none.update(cx, |s, cx| s.save_folder_scope_settings(fid, cur_h, Auth::None, cx));
                                                                            }
                                                                        }
                                                                    }),
                                                            )
                                                            .item(
                                                                PopupMenuItem::new("Bearer Token")
                                                                    .checked(is_bearer)
                                                                    .on_click(move |_, _, cx| {
                                                                        let new_auth = Auth::Bearer { token: String::new() };
                                                                        match &tgt_bearer {
                                                                            ScopeSettingsTarget::Workspace => {
                                                                                let cur_h = ws_bearer.read(cx).active_workspace.as_ref().map(|w| w.headers.clone()).unwrap_or_default();
                                                                                ws_bearer.update(cx, |s, cx| s.save_workspace_scope_settings(cur_h, new_auth, cx));
                                                                            }
                                                                            ScopeSettingsTarget::Folder(fid) => {
                                                                                let cur_h = ws_bearer.read(cx).folders.iter().find(|f| &f.id == fid).map(|f| f.headers.clone()).unwrap_or_default();
                                                                                ws_bearer.update(cx, |s, cx| s.save_folder_scope_settings(fid, cur_h, new_auth, cx));
                                                                            }
                                                                        }
                                                                    }),
                                                            )
                                                            .item(
                                                                PopupMenuItem::new("Basic Auth")
                                                                    .checked(is_basic)
                                                                    .on_click(move |_, _, cx| {
                                                                        let new_auth = Auth::Basic { username: String::new(), password: String::new() };
                                                                        match &tgt_basic {
                                                                            ScopeSettingsTarget::Workspace => {
                                                                                let cur_h = ws_basic.read(cx).active_workspace.as_ref().map(|w| w.headers.clone()).unwrap_or_default();
                                                                                ws_basic.update(cx, |s, cx| s.save_workspace_scope_settings(cur_h, new_auth, cx));
                                                                            }
                                                                            ScopeSettingsTarget::Folder(fid) => {
                                                                                let cur_h = ws_basic.read(cx).folders.iter().find(|f| &f.id == fid).map(|f| f.headers.clone()).unwrap_or_default();
                                                                                ws_basic.update(cx, |s, cx| s.save_folder_scope_settings(fid, cur_h, new_auth, cx));
                                                                            }
                                                                        }
                                                                    }),
                                                            )
                                                            .item(
                                                                PopupMenuItem::new("API Key")
                                                                    .checked(is_apikey)
                                                                    .on_click(move |_, _, cx| {
                                                                        let new_auth = Auth::ApiKey { key: "X-API-Key".to_string(), value: String::new(), in_header: true };
                                                                        match &tgt_api {
                                                                            ScopeSettingsTarget::Workspace => {
                                                                                let cur_h = ws_api.read(cx).active_workspace.as_ref().map(|w| w.headers.clone()).unwrap_or_default();
                                                                                ws_api.update(cx, |s, cx| s.save_workspace_scope_settings(cur_h, new_auth, cx));
                                                                            }
                                                                            ScopeSettingsTarget::Folder(fid) => {
                                                                                let cur_h = ws_api.read(cx).folders.iter().find(|f| &f.id == fid).map(|f| f.headers.clone()).unwrap_or_default();
                                                                                ws_api.update(cx, |s, cx| s.save_folder_scope_settings(fid, cur_h, new_auth, cx));
                                                                            }
                                                                        }
                                                                    }),
                                                            )
                                                        })
                                                }),
                                        )
                                        .child({
                                            match current_auth {
                                                Auth::Inherit => {
                                                    let desc = ws_s.read(cx).get_inherited_auth_description(folder_id_opt.as_deref());
                                                    div()
                                                        .p(px(10.))
                                                        .rounded(px(6.))
                                                        .bg(t.bg_app)
                                                        .border_1()
                                                        .border_color(t.border)
                                                        .flex()
                                                        .items_center()
                                                        .gap(px(8.))
                                                        .child(Icon::new(IconName::Info).size(px(16.)))
                                                        .child(
                                                            div()
                                                                .text_xs()
                                                                .text_color(t.text_secondary)
                                                                .child(desc),
                                                        )
                                                }
                                                Auth::None => {
                                                    div()
                                                        .pt(px(4.))
                                                        .text_xs()
                                                        .italic()
                                                        .text_color(t.text_muted)
                                                        .child(if is_folder {
                                                            "No authentication will be added to requests in this folder."
                                                        } else {
                                                            "No authentication will be added to requests in this workspace."
                                                        })
                                                }
                                                Auth::Bearer { .. } => {
                                                    div()
                                                        .flex()
                                                        .items_center()
                                                        .gap(px(10.))
                                                        .child(
                                                            div()
                                                                .w(px(70.))
                                                                .text_xs()
                                                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                                                .text_color(t.text_secondary)
                                                                .child("Token:"),
                                                        )
                                                        .child(
                                                            div()
                                                                .flex_1()
                                                                .h(px(28.))
                                                                .px(px(6.))
                                                                .rounded(px(4.))
                                                                .bg(t.bg_input)
                                                                .border_1()
                                                                .border_color(t.border)
                                                                .flex()
                                                                .items_center()
                                                                .child(
                                                                    Input::new(&bearer_input)
                                                                        .appearance(false)
                                                                        .bordered(false)
                                                                        .focus_bordered(false),
                                                                ),
                                                        )
                                                }
                                                Auth::Basic { .. } => {
                                                    div()
                                                        .flex()
                                                        .flex_col()
                                                        .gap(px(8.))
                                                        .child(
                                                            div()
                                                                .flex()
                                                                .items_center()
                                                                .gap(px(10.))
                                                                .child(
                                                                    div()
                                                                        .w(px(70.))
                                                                        .text_xs()
                                                                        .font_weight(gpui::FontWeight::SEMIBOLD)
                                                                        .text_color(t.text_secondary)
                                                                        .child("Username:"),
                                                                )
                                                                .child(
                                                                    div()
                                                                        .flex_1()
                                                                        .h(px(28.))
                                                                        .px(px(6.))
                                                                        .rounded(px(4.))
                                                                        .bg(t.bg_input)
                                                                        .border_1()
                                                                        .border_color(t.border)
                                                                        .flex()
                                                                        .items_center()
                                                                        .child(
                                                                            Input::new(&basic_user_input)
                                                                                .appearance(false)
                                                                                .bordered(false)
                                                                                .focus_bordered(false),
                                                                        ),
                                                                ),
                                                        )
                                                        .child(
                                                            div()
                                                                .flex()
                                                                .items_center()
                                                                .gap(px(10.))
                                                                .child(
                                                                    div()
                                                                        .w(px(70.))
                                                                        .text_xs()
                                                                        .font_weight(gpui::FontWeight::SEMIBOLD)
                                                                        .text_color(t.text_secondary)
                                                                        .child("Password:"),
                                                                )
                                                                .child(
                                                                    div()
                                                                        .flex_1()
                                                                        .h(px(28.))
                                                                        .px(px(6.))
                                                                        .rounded(px(4.))
                                                                        .bg(t.bg_input)
                                                                        .border_1()
                                                                        .border_color(t.border)
                                                                        .flex()
                                                                        .items_center()
                                                                        .child(
                                                                            Input::new(&basic_pass_input)
                                                                                .appearance(false)
                                                                                .bordered(false)
                                                                                .focus_bordered(false),
                                                                        ),
                                                                ),
                                                        )
                                                }
                                                Auth::ApiKey { in_header, .. } => {
                                                    div()
                                                        .flex()
                                                        .flex_col()
                                                        .gap(px(8.))
                                                        .child(
                                                            div()
                                                                .flex()
                                                                .items_center()
                                                                .gap(px(10.))
                                                                .child(
                                                                    div()
                                                                        .w(px(70.))
                                                                        .text_xs()
                                                                        .font_weight(gpui::FontWeight::SEMIBOLD)
                                                                        .text_color(t.text_secondary)
                                                                        .child("Key:"),
                                                                )
                                                                .child(
                                                                    div()
                                                                        .flex_1()
                                                                        .h(px(28.))
                                                                        .px(px(6.))
                                                                        .rounded(px(4.))
                                                                        .bg(t.bg_input)
                                                                        .border_1()
                                                                        .border_color(t.border)
                                                                        .flex()
                                                                        .items_center()
                                                                        .child(
                                                                            Input::new(&api_key_input)
                                                                                .appearance(false)
                                                                                .bordered(false)
                                                                                .focus_bordered(false),
                                                                        ),
                                                                ),
                                                        )
                                                        .child(
                                                            div()
                                                                .flex()
                                                                .items_center()
                                                                .gap(px(10.))
                                                                .child(
                                                                    div()
                                                                        .w(px(70.))
                                                                        .text_xs()
                                                                        .font_weight(gpui::FontWeight::SEMIBOLD)
                                                                        .text_color(t.text_secondary)
                                                                        .child("Value:"),
                                                                )
                                                                .child(
                                                                    div()
                                                                        .flex_1()
                                                                        .h(px(28.))
                                                                        .px(px(6.))
                                                                        .rounded(px(4.))
                                                                        .bg(t.bg_input)
                                                                        .border_1()
                                                                        .border_color(t.border)
                                                                        .flex()
                                                                        .items_center()
                                                                        .child(
                                                                            Input::new(&api_val_input)
                                                                                .appearance(false)
                                                                                .bordered(false)
                                                                                .focus_bordered(false),
                                                                        ),
                                                                ),
                                                        )
                                                        .child(
                                                            div()
                                                                .flex()
                                                                .items_center()
                                                                .gap(px(10.))
                                                                .child(
                                                                    div()
                                                                        .w(px(70.))
                                                                        .text_xs()
                                                                        .font_weight(gpui::FontWeight::SEMIBOLD)
                                                                        .text_color(t.text_secondary)
                                                                        .child("Add to:"),
                                                                )
                                                                .child({
                                                                    let loc_label = if in_header { "Header" } else { "Query Params" };
                                                                    let ws_loc = ws_s.clone();
                                                                    let tgt_loc = tgt.clone();
                                                                    let k_input = api_key_input.clone();
                                                                    let v_input = api_val_input.clone();

                                                                    Button::new("btn-scope-auth-apikey-location-dropdown")
                                                                        .ghost()
                                                                        .compact()
                                                                        .px(px(8.))
                                                                        .py(px(4.))
                                                                        .h(px(28.))
                                                                        .rounded(px(4.))
                                                                        .bg(t.bg_card)
                                                                        .border_1()
                                                                        .border_color(t.border)
                                                                        .child(
                                                                            div()
                                                                                .text_xs()
                                                                                .font_weight(gpui::FontWeight::MEDIUM)
                                                                                .text_color(t.text_primary)
                                                                                .child(loc_label),
                                                                        )
                                                                        .dropdown_menu(move |menu, _window, _cx| {
                                                                            let ws_h = ws_loc.clone();
                                                                            let tgt_h = tgt_loc.clone();
                                                                            let k_h = k_input.clone();
                                                                            let v_h = v_input.clone();

                                                                            let ws_q = ws_loc.clone();
                                                                            let tgt_q = tgt_loc.clone();
                                                                            let k_q = k_input.clone();
                                                                            let v_q = v_input.clone();

                                                                            menu.item(
                                                                                PopupMenuItem::new("Header")
                                                                                    .checked(in_header)
                                                                                    .on_click(move |_, _, cx| {
                                                                                        let key = k_h.read(cx).value().to_string();
                                                                                        let val = v_h.read(cx).value().to_string();
                                                                                        match &tgt_h {
                                                                                            ScopeSettingsTarget::Workspace => {
                                                                                                let cur_h = ws_h.read(cx).active_workspace.as_ref().map(|w| w.headers.clone()).unwrap_or_default();
                                                                                                ws_h.update(cx, |s, cx| s.save_workspace_scope_settings(cur_h, Auth::ApiKey { key, value: val, in_header: true }, cx));
                                                                                            }
                                                                                            ScopeSettingsTarget::Folder(fid) => {
                                                                                                let cur_h = ws_h.read(cx).folders.iter().find(|f| &f.id == fid).map(|f| f.headers.clone()).unwrap_or_default();
                                                                                                ws_h.update(cx, |s, cx| s.save_folder_scope_settings(fid, cur_h, Auth::ApiKey { key, value: val, in_header: true }, cx));
                                                                                            }
                                                                                        }
                                                                                    }),
                                                                            )
                                                                            .item(
                                                                                PopupMenuItem::new("Query Params")
                                                                                    .checked(!in_header)
                                                                                    .on_click(move |_, _, cx| {
                                                                                        let key = k_q.read(cx).value().to_string();
                                                                                        let val = v_q.read(cx).value().to_string();
                                                                                        match &tgt_q {
                                                                                            ScopeSettingsTarget::Workspace => {
                                                                                                let cur_h = ws_q.read(cx).active_workspace.as_ref().map(|w| w.headers.clone()).unwrap_or_default();
                                                                                                ws_q.update(cx, |s, cx| s.save_workspace_scope_settings(cur_h, Auth::ApiKey { key, value: val, in_header: false }, cx));
                                                                                            }
                                                                                            ScopeSettingsTarget::Folder(fid) => {
                                                                                                let cur_h = ws_q.read(cx).folders.iter().find(|f| &f.id == fid).map(|f| f.headers.clone()).unwrap_or_default();
                                                                                                ws_q.update(cx, |s, cx| s.save_folder_scope_settings(fid, cur_h, Auth::ApiKey { key, value: val, in_header: false }, cx));
                                                                                            }
                                                                                        }
                                                                                    }),
                                                                            )
                                                                        })
                                                                }),
                                                        )
                                                }
                                            }
                                        })
                                        .into_any_element()
                                }
                            }),
                    ),
            )
            .into_any_element()
    }
}

use super::auth_input::render_auth_input;
use crate::state::{EditorState, WorkspaceState};
use crate::theme::theme;
use gpui::{
    div, px, App, Div, Entity, ParentElement, Styled, Window,
};
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::input::InputState;
use gpui_component::menu::{DropdownMenu as _, PopupMenuItem};
use gpui_component::Icon;
use gpui_kit::assets::IconName;
use restclient_core::models::Auth;

pub struct AuthTabInputs<'a> {
    pub bearer_token_input: &'a Entity<InputState>,
    pub basic_user_input: &'a Entity<InputState>,
    pub basic_pass_input: &'a Entity<InputState>,
    pub apikey_key_input: &'a Entity<InputState>,
    pub apikey_val_input: &'a Entity<InputState>,
}

pub fn render_auth_tab(
    auth: &Auth,
    folder_id: Option<&str>,
    workspace_state: &Entity<WorkspaceState>,
    editor_state: &Entity<EditorState>,
    inputs: AuthTabInputs,
    window: &mut Window,
    cx: &mut App,
) -> Div {
    let t = theme();
    let ed_c = editor_state.clone();
    let auth_label = match auth {
        Auth::Inherit => "Inherit",
        Auth::None => "None",
        Auth::Bearer { .. } => "Bearer Token",
        Auth::Basic { .. } => "Basic Auth",
        Auth::ApiKey { .. } => "API Key",
    };
    let is_inherit = matches!(auth, Auth::Inherit);
    let is_none = matches!(auth, Auth::None);
    let is_bearer = matches!(auth, Auth::Bearer { .. });
    let is_basic = matches!(auth, Auth::Basic { .. });
    let is_apikey = matches!(auth, Auth::ApiKey { .. });

    div()
        .p(px(16.))
        .flex()
        .flex_col()
        .gap(px(10.))
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
                    let ed_m = ed_c.clone();
                    Button::new("btn-auth-type-dropdown")
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
                            let ed_inh = ed_m.clone();
                            let ed_none = ed_m.clone();
                            let ed_bearer = ed_m.clone();
                            let ed_basic = ed_m.clone();
                            let ed_apikey = ed_m.clone();

                            menu.item(
                                PopupMenuItem::new("Inherit")
                                    .checked(is_inherit)
                                    .on_click(move |_, _, cx| {
                                        ed_inh.update(cx, |state, cx| {
                                            if !matches!(
                                                state.active_request.as_ref().map(|r| &r.auth),
                                                Some(Auth::Inherit)
                                            ) {
                                                state.set_auth(Auth::Inherit, cx);
                                            }
                                        });
                                    }),
                            )
                            .item(
                                PopupMenuItem::new("None")
                                    .checked(is_none)
                                    .on_click(move |_, _, cx| {
                                        ed_none.update(cx, |state, cx| {
                                            if !matches!(
                                                state.active_request.as_ref().map(|r| &r.auth),
                                                Some(Auth::None)
                                            ) {
                                                state.set_auth(Auth::None, cx);
                                            }
                                        });
                                    }),
                            )
                            .item(
                                PopupMenuItem::new("Bearer Token")
                                    .checked(is_bearer)
                                    .on_click(move |_, _, cx| {
                                        ed_bearer.update(cx, |state, cx| {
                                            if !matches!(
                                                state.active_request.as_ref().map(|r| &r.auth),
                                                Some(Auth::Bearer { .. })
                                            ) {
                                                state.set_auth(
                                                    Auth::Bearer {
                                                        token: "".to_string(),
                                                    },
                                                    cx,
                                                );
                                            }
                                        });
                                    }),
                            )
                            .item(
                                PopupMenuItem::new("Basic Auth")
                                    .checked(is_basic)
                                    .on_click(move |_, _, cx| {
                                        ed_basic.update(cx, |state, cx| {
                                            if !matches!(
                                                state.active_request.as_ref().map(|r| &r.auth),
                                                Some(Auth::Basic { .. })
                                            ) {
                                                state.set_auth(
                                                    Auth::Basic {
                                                        username: "".to_string(),
                                                        password: "".to_string(),
                                                    },
                                                    cx,
                                                );
                                            }
                                        });
                                    }),
                            )
                            .item(
                                PopupMenuItem::new("API Key")
                                    .checked(is_apikey)
                                    .on_click(move |_, _, cx| {
                                        ed_apikey.update(cx, |state, cx| {
                                            if !matches!(
                                                state.active_request.as_ref().map(|r| &r.auth),
                                                Some(Auth::ApiKey { .. })
                                            ) {
                                                state.set_auth(
                                                    Auth::ApiKey {
                                                        key: "X-API-Key".to_string(),
                                                        value: "".to_string(),
                                                        in_header: true,
                                                    },
                                                    cx,
                                                );
                                            }
                                        });
                                    }),
                            )
                        })
                }),
        )
        .child({
            match auth {
                Auth::Inherit => {
                    let desc = workspace_state
                        .read(cx)
                        .get_inherited_auth_description(folder_id);
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
                Auth::None => div()
                    .pt(px(4.))
                    .text_xs()
                    .italic()
                    .text_color(t.text_muted)
                    .child("No authentication will be added to this request."),
                Auth::Bearer { .. } => {
                    let ed_b = ed_c.clone();
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
                                .w_full()
                                .max_w(px(400.))
                                .child(render_auth_input(
                                    "auth-bearer-token-input",
                                    inputs.bearer_token_input,
                                    workspace_state,
                                    window,
                                    cx,
                                    move |new_token, _window, cx| {
                                        ed_b.update(cx, |ed, cx| {
                                            ed.set_bearer_token(new_token, cx);
                                        });
                                    },
                                )),
                        )
                }
                Auth::Basic { .. } => {
                    let ed_u = ed_c.clone();
                    let ed_p = ed_c.clone();
                    let p_in = inputs.basic_pass_input.clone();
                    let u_in = inputs.basic_user_input.clone();
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(10.))
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
                                        .w_full()
                                        .max_w(px(400.))
                                        .child(render_auth_input(
                                            "auth-basic-user-input",
                                            inputs.basic_user_input,
                                            workspace_state,
                                            window,
                                            cx,
                                            move |new_user, _window, cx| {
                                                let pass = p_in.read(cx).value().to_string();
                                                ed_u.update(cx, |ed, cx| {
                                                    ed.set_basic_auth(new_user, pass, cx);
                                                });
                                            },
                                        )),
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
                                        .w_full()
                                        .max_w(px(400.))
                                        .child(render_auth_input(
                                            "auth-basic-pass-input",
                                            inputs.basic_pass_input,
                                            workspace_state,
                                            window,
                                            cx,
                                            move |new_pass, _window, cx| {
                                                let user = u_in.read(cx).value().to_string();
                                                ed_p.update(cx, |ed, cx| {
                                                    ed.set_basic_auth(user, new_pass, cx);
                                                });
                                            },
                                        )),
                                ),
                        )
                }
                Auth::ApiKey { in_header, .. } => {
                    let in_header_val = *in_header;
                    let ed_loc = ed_c.clone();
                    let ed_k = ed_c.clone();
                    let ed_v = ed_c.clone();
                    let v_in = inputs.apikey_val_input.clone();
                    let k_in = inputs.apikey_key_input.clone();
                    let loc_label = if in_header_val {
                        "Header"
                    } else {
                        "Query Params"
                    };

                    div()
                        .flex()
                        .flex_col()
                        .gap(px(10.))
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
                                        .w_full()
                                        .max_w(px(400.))
                                        .child(render_auth_input(
                                            "auth-apikey-key-input",
                                            inputs.apikey_key_input,
                                            workspace_state,
                                            window,
                                            cx,
                                            move |new_key, _window, cx| {
                                                let val = v_in.read(cx).value().to_string();
                                                ed_k.update(cx, |ed, cx| {
                                                    ed.set_api_key(
                                                        new_key,
                                                        val,
                                                        in_header_val,
                                                        cx,
                                                    );
                                                });
                                            },
                                        )),
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
                                        .w_full()
                                        .max_w(px(400.))
                                        .child(render_auth_input(
                                            "auth-apikey-val-input",
                                            inputs.apikey_val_input,
                                            workspace_state,
                                            window,
                                            cx,
                                            move |new_val, _window, cx| {
                                                let key = k_in.read(cx).value().to_string();
                                                ed_v.update(cx, |ed, cx| {
                                                    ed.set_api_key(
                                                        key,
                                                        new_val,
                                                        in_header_val,
                                                        cx,
                                                    );
                                                });
                                            },
                                        )),
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
                                    let ed_l = ed_loc.clone();
                                    let k_input = inputs.apikey_key_input.clone();
                                    let v_input = inputs.apikey_val_input.clone();
                                    Button::new("btn-auth-apikey-location-dropdown")
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
                                            let ed_h = ed_l.clone();
                                            let ed_q = ed_l.clone();
                                            let k_h = k_input.clone();
                                            let v_h = v_input.clone();
                                            let k_q = k_input.clone();
                                            let v_q = v_input.clone();

                                            menu.item(
                                                PopupMenuItem::new("Header")
                                                    .checked(in_header_val)
                                                    .on_click(move |_, _, cx| {
                                                        let key = k_h.read(cx).value().to_string();
                                                        let val = v_h.read(cx).value().to_string();
                                                        ed_h.update(cx, |ed, cx| {
                                                            ed.set_api_key(key, val, true, cx);
                                                        });
                                                    }),
                                            )
                                            .item(
                                                PopupMenuItem::new("Query Params")
                                                    .checked(!in_header_val)
                                                    .on_click(move |_, _, cx| {
                                                        let key = k_q.read(cx).value().to_string();
                                                        let val = v_q.read(cx).value().to_string();
                                                        ed_q.update(cx, |ed, cx| {
                                                            ed.set_api_key(key, val, false, cx);
                                                        });
                                                    }),
                                            )
                                        })
                                }),
                        )
                }
            }
        })
}

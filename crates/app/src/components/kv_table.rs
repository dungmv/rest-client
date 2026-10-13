use crate::theme::theme;
use gpui::{
    div, px, App, AppContext as _, Entity, InteractiveElement as _, IntoElement, ParentElement,
    RenderOnce, StatefulInteractiveElement as _, Styled, Subscription, Window,
};
use gpui_component::input::{Input, InputEvent, InputState};
use restclient_core::models::KeyValuePair;
use std::rc::Rc;

struct KvRowInputHolder {
    key_input: Entity<InputState>,
    val_input: Entity<InputState>,
    _subscriptions: Vec<Subscription>,
}

#[derive(IntoElement)]
pub struct KvTable {
    id_prefix: String,
    pairs: Vec<KeyValuePair>,
    on_update: Rc<dyn Fn(usize, String, String, bool, &mut Window, &mut App)>,
    on_add: Rc<dyn Fn(&mut Window, &mut App)>,
    on_remove: Rc<dyn Fn(usize, &mut Window, &mut App)>,
}

impl KvTable {
    pub fn new(
        id_prefix: impl Into<String>,
        pairs: Vec<KeyValuePair>,
        on_update: impl Fn(usize, String, String, bool, &mut Window, &mut App) + 'static,
        on_add: impl Fn(&mut Window, &mut App) + 'static,
        on_remove: impl Fn(usize, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id_prefix: id_prefix.into(),
            pairs,
            on_update: Rc::new(on_update),
            on_add: Rc::new(on_add),
            on_remove: Rc::new(on_remove),
        }
    }
}

impl RenderOnce for KvTable {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = theme();
        let on_add_cb = self.on_add.clone();
        let id_prefix = self.id_prefix.clone();

        div()
            .w_full()
            .flex()
            .flex_col()
            .gap(px(4.))
            .p(px(8.))
            // Table Header
            .child(
                div()
                    .flex()
                    .items_center()
                    .px(px(6.))
                    .py(px(4.))
                    .text_xs()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(t.text_muted)
                    .border_b_1()
                    .border_color(t.border)
                    .child(div().w(px(28.)).child(""))
                    .child(div().w(px(220.)).child("Key"))
                    .child(div().flex_1().child("Value"))
                    .child(div().w(px(28.)).child("")),
            )
            // Table Rows
            .children(self.pairs.into_iter().enumerate().map(|(idx, pair)| {
                let on_update = self.on_update.clone();
                let on_remove = self.on_remove.clone();
                let key_val = pair.key.clone();
                let val_val = pair.value.clone();
                let enabled = pair.enabled;
                let row_prefix = format!("{}-row-{}", id_prefix, idx);

                let on_up_k = on_update.clone();
                let on_up_v = on_update.clone();
                let row_holder = window.use_keyed_state(
                    gpui::SharedString::from(format!("{}-holder", row_prefix)),
                    cx,
                    |window, cx| {
                        let key_input = cx.new(|cx| InputState::new(window, cx).placeholder("Key..."));
                        let val_input = cx.new(|cx| InputState::new(window, cx).placeholder("Value..."));

                        let k_in_1 = key_input.clone();
                        let v_in_1 = val_input.clone();
                        let sub1 = cx.subscribe_in(&key_input, window, move |_, _, event, window, cx| {
                            if let InputEvent::Change = event {
                                let k = k_in_1.read(cx).value().to_string();
                                let v = v_in_1.read(cx).value().to_string();
                                on_up_k(idx, k, v, true, window, cx);
                            }
                        });

                        let k_in_2 = key_input.clone();
                        let v_in_2 = val_input.clone();
                        let sub2 = cx.subscribe_in(&val_input, window, move |_, _, event, window, cx| {
                            if let InputEvent::Change = event {
                                let k = k_in_2.read(cx).value().to_string();
                                let v = v_in_2.read(cx).value().to_string();
                                on_up_v(idx, k, v, true, window, cx);
                            }
                        });

                        KvRowInputHolder {
                            key_input,
                            val_input,
                            _subscriptions: vec![sub1, sub2],
                        }
                    },
                );

                let (key_input, val_input) = {
                    let h = row_holder.read(cx);
                    (h.key_input.clone(), h.val_input.clone())
                };

                if key_input.read(cx).value().as_str() != key_val {
                    key_input.update(cx, |inp, cx| {
                        inp.set_value(key_val.clone(), window, cx);
                    });
                }
                if val_input.read(cx).value().as_str() != val_val {
                    val_input.update(cx, |inp, cx| {
                        inp.set_value(val_val.clone(), window, cx);
                    });
                }

                div()
                    .flex()
                    .items_center()
                    .px(px(6.))
                    .py(px(2.))
                    .gap(px(8.))
                    .rounded(px(4.))
                    .hover(|s| s.bg(t.bg_hover))
                    // Checkbox
                    .child({
                        let on_update_c = on_update.clone();
                        let k = key_val.clone();
                        let v = val_val.clone();
                        div()
                            .id(gpui::SharedString::from(format!("{}-check", row_prefix)))
                            .w(px(20.))
                            .h(px(20.))
                            .rounded(px(4.))
                            .border_1()
                            .border_color(if enabled { t.primary } else { t.border })
                            .bg(if enabled { t.primary } else { t.bg_input })
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .text_xs()
                            .text_color(t.primary_foreground)
                            .on_click(move |_, window, cx| {
                                on_update_c(idx, k.clone(), v.clone(), !enabled, window, cx);
                            })
                            .child(if enabled { "✓" } else { "" })
                    })
                    // Key Input
                    .child(
                        div()
                            .w(px(220.))
                            .h(px(28.))
                            .px(px(4.))
                            .rounded(px(4.))
                            .bg(t.bg_input)
                            .border_1()
                            .border_color(t.border)
                            .flex()
                            .items_center()
                            .child(
                                Input::new(&key_input)
                                    .id(gpui::SharedString::from(format!("{}-key", row_prefix)))
                                    .appearance(false)
                                    .bordered(false)
                                    .focus_bordered(false),
                            ),
                    )
                    // Value Input
                    .child(
                        div()
                            .flex_1()
                            .h(px(28.))
                            .px(px(4.))
                            .rounded(px(4.))
                            .bg(t.bg_input)
                            .border_1()
                            .border_color(t.border)
                            .flex()
                            .items_center()
                            .child(
                                Input::new(&val_input)
                                    .id(gpui::SharedString::from(format!("{}-val", row_prefix)))
                                    .appearance(false)
                                    .bordered(false)
                                    .focus_bordered(false),
                            ),
                    )
                    // Delete button
                    .child({
                        div()
                            .id(gpui::SharedString::from(format!("{}-del", row_prefix)))
                            .w(px(24.))
                            .h(px(24.))
                            .rounded(px(4.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .hover(|s| s.bg(t.bg_hover).text_color(t.status_5xx))
                            .text_color(t.text_muted)
                            .text_xs()
                            .on_click(move |_, window, cx| {
                                on_remove(idx, window, cx);
                            })
                            .child("×")
                    })
            }))
            // Add row button
            .child(
                div()
                    .pt(px(6.))
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(
                        div()
                            .id(gpui::SharedString::from(format!("{}-btn-add", id_prefix)))
                            .px(px(10.))
                            .py(px(4.))
                            .rounded(px(4.))
                            .bg(t.bg_card)
                            .border_1()
                            .border_color(t.border)
                            .cursor_pointer()
                            .hover(|s| s.bg(t.bg_hover).text_color(t.text_primary))
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_xs()
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .text_color(t.text_secondary)
                            .on_click(move |_, window, cx| {
                                on_add_cb(window, cx);
                            })
                            .child("+ Add Row"),
                    ),
            )
    }
}

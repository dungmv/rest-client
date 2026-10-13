use crate::theme::theme;
use gpui::{
    div, px, App, InteractiveElement as _, IntoElement, ParentElement,
    RenderOnce, StatefulInteractiveElement as _, Styled, Window,
};
use restclient_core::models::KeyValuePair;
use std::rc::Rc;

#[derive(IntoElement)]
pub struct KvTable {
    pairs: Vec<KeyValuePair>,
    on_update: Rc<dyn Fn(usize, String, String, bool, &mut Window, &mut App)>,
    on_add: Rc<dyn Fn(&mut Window, &mut App)>,
    on_remove: Rc<dyn Fn(usize, &mut Window, &mut App)>,
}

impl KvTable {
    pub fn new(
        pairs: Vec<KeyValuePair>,
        on_update: impl Fn(usize, String, String, bool, &mut Window, &mut App) + 'static,
        on_add: impl Fn(&mut Window, &mut App) + 'static,
        on_remove: impl Fn(usize, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            pairs,
            on_update: Rc::new(on_update),
            on_add: Rc::new(on_add),
            on_remove: Rc::new(on_remove),
        }
    }
}

impl RenderOnce for KvTable {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let t = theme();
        let on_add_cb = self.on_add.clone();

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
                    .child(div().w(px(24.)).child(""))
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
                            .id(format!("check-row-{}", idx))
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
                    // Key
                    .child(
                        div()
                            .w(px(220.))
                            .h(px(28.))
                            .px(px(8.))
                            .rounded(px(4.))
                            .bg(t.bg_input)
                            .border_1()
                            .border_color(t.border)
                            .flex()
                            .items_center()
                            .text_xs()
                            .font_family(".SystemMonoFont")
                            .text_color(if enabled { t.text_primary } else { t.text_muted })
                            .child(if key_val.is_empty() { "Key...".to_string() } else { key_val }),
                    )
                    // Value
                    .child(
                        div()
                            .flex_1()
                            .h(px(28.))
                            .px(px(8.))
                            .rounded(px(4.))
                            .bg(t.bg_input)
                            .border_1()
                            .border_color(t.border)
                            .flex()
                            .items_center()
                            .text_xs()
                            .font_family(".SystemMonoFont")
                            .text_color(if enabled { t.text_primary } else { t.text_muted })
                            .child(if val_val.is_empty() { "Value...".to_string() } else { val_val }),
                    )
                    // Delete button
                    .child({
                        div()
                            .id(format!("del-row-{}", idx))
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
            // Add row & common quick templates
            .child(
                div()
                    .pt(px(6.))
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(
                        div()
                            .id("btn-add-kv-row")
                            .px(px(10.))
                            .py(px(4.))
                            .rounded(px(4.))
                            .bg(t.primary)
                            .cursor_pointer()
                            .hover(|s| s.bg(t.primary_hover))
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_xs()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(t.primary_foreground)
                            .on_click(move |_, window, cx| {
                                on_add_cb(window, cx);
                            })
                            .child("+ Add Row"),
                    )
            )
    }
}

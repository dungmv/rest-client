use crate::theme::theme;
use gpui::{div, px, App, IntoElement, ParentElement, RenderOnce, Styled, Window};
use restclient_core::models::HttpMethod;

#[derive(IntoElement)]
pub struct MethodBadge {
    method: HttpMethod,
}

impl MethodBadge {
    pub fn new(method: HttpMethod) -> Self {
        Self { method }
    }
}

impl RenderOnce for MethodBadge {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let t = theme();
        let color = t.method_color(self.method);
        let bg_color = gpui::Hsla {
            a: 0.15,
            ..color
        };

        div()
            .px(px(6.))
            .py(px(1.5))
            .rounded(px(4.))
            .bg(bg_color)
            .text_color(color)
            .text_xs()
            .font_weight(gpui::FontWeight::BOLD)
            .child(self.method.as_str())
    }
}

#[derive(IntoElement)]
pub struct StatusCodeBadge {
    status_code: u16,
}

impl StatusCodeBadge {
    pub fn new(status_code: u16) -> Self {
        Self { status_code }
    }
}

impl RenderOnce for StatusCodeBadge {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let t = theme();
        let color = t.status_color(self.status_code);

        div()
            .px(px(5.))
            .py(px(1.))
            .text_color(color)
            .text_xs()
            .font_weight(gpui::FontWeight::SEMIBOLD)
            .child(format!("{}", self.status_code))
    }
}

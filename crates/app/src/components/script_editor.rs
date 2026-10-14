use crate::theme::theme;
use gpui::prelude::FluentBuilder as _;
use gpui::{
    div, px, App, Entity, InteractiveElement as _, IntoElement, ParentElement, RenderOnce,
    StatefulInteractiveElement as _, Styled, Window,
};
use gpui_component::input::{Textarea, TextareaState};
use std::rc::Rc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptEditorKind {
    PreRequest,
    Tests,
}

#[derive(IntoElement)]
pub struct ScriptEditor {
    textarea: Entity<TextareaState>,
    kind: ScriptEditorKind,
    error_message: Option<String>,
    on_insert_snippet: Rc<dyn Fn(&str, &mut Window, &mut App)>,
}

impl ScriptEditor {
    pub fn new(
        textarea: Entity<TextareaState>,
        kind: ScriptEditorKind,
        error_message: Option<String>,
        on_insert_snippet: impl Fn(&str, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            textarea,
            kind,
            error_message,
            on_insert_snippet: Rc::new(on_insert_snippet),
        }
    }
}

impl RenderOnce for ScriptEditor {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let t = theme();
        let kind = self.kind;
        let on_insert = self.on_insert_snippet.clone();

        let snippets: Vec<(&'static str, &'static str)> = match kind {
            ScriptEditorKind::PreRequest => vec![
                (
                    "Set Timestamp Header",
                    "client.request.headers.add(\"X-Timestamp\", Date.now().toString());\n",
                ),
                (
                    "HMAC-SHA256 Signature",
                    "const sig = client.crypto.hmacSha256(\"my_secret\", Date.now().toString());\nclient.request.headers.add(\"X-Signature\", sig);\n",
                ),
                (
                    "Set Env Variable",
                    "client.environment.set(\"custom_key\", \"custom_value\");\n",
                ),
                (
                    "Mutate JSON Body",
                    "const body = client.request.body.json();\nif (body) {\n    body.timestamp = Date.now();\n    client.request.body.setJson(body);\n}\n",
                ),
            ],
            ScriptEditorKind::Tests => vec![
                (
                    "Status code: 200",
                    "client.test(\"Status code is 200\", () => {\n    client.assert(client.response.status === 200, \"Expected 200 OK\");\n});\n",
                ),
                (
                    "Response time < 500ms",
                    "client.test(\"Response time is under 500ms\", () => {\n    client.assert(client.response.time < 500, \"Too slow\");\n});\n",
                ),
                (
                    "JSON value check",
                    "client.test(\"Verify JSON property\", () => {\n    const data = client.response.json();\n    client.assert(data !== null, \"Response should be JSON\");\n});\n",
                ),
                (
                    "Save Token to Env",
                    "// Request Chaining: extract token from response\nconst data = client.response.json();\nif (data && data.token) {\n    client.environment.set(\"access_token\", data.token);\n}\n",
                ),
            ],
        };

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(t.bg_editor)
            // Error banner if any
            .when_some(self.error_message, |this, err| {
                this.child(
                    div()
                        .px(px(12.))
                        .py(px(6.))
                        .bg(gpui::Hsla {
                            a: 0.15,
                            ..t.method_delete
                        })
                        .border_b_1()
                        .border_color(gpui::Hsla {
                            a: 0.35,
                            ..t.method_delete
                        })
                        .text_xs()
                        .text_color(t.method_delete)
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .child("⚠ Script Error:")
                        .child(err),
                )
            })
            // Main workspace: Code Textarea on left, Snippets pane on right
            .child(
                div()
                    .flex_1()
                    .flex()
                    .child(
                        // Left: Code Editor Area
                        div()
                            .flex_1()
                            .h_full()
                            .p(px(8.))
                            .child(
                                Textarea::new(&self.textarea)
                                    .appearance(false)
                                    .bordered(false)
                                    .h(gpui::relative(1.)),
                            ),
                    )
                    // Right: Snippets Toolbar
                    .child(
                        div()
                            .w(px(200.))
                            .h_full()
                            .border_l_1()
                            .border_color(t.border)
                            .bg(t.bg_panel)
                            .p(px(10.))
                            .flex()
                            .flex_col()
                            .gap(px(6.))
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(gpui::FontWeight::BOLD)
                                    .text_color(t.text_secondary)
                                    .mb(px(4.))
                                    .child("SNIPPETS"),
                            )
                            .children(snippets.into_iter().enumerate().map(|(idx, (label, code))| {
                                let insert_cb = on_insert.clone();
                                let snippet_code = code;
                                div()
                                    .id(format!("snippet-btn-{}-{}", match kind {
                                        ScriptEditorKind::PreRequest => "pre",
                                        ScriptEditorKind::Tests => "test",
                                    }, idx))
                                    .px(px(8.))
                                    .py(px(5.))
                                    .rounded(px(4.))
                                    .bg(t.bg_editor)
                                    .border_1()
                                    .border_color(t.border)
                                    .text_xs()
                                    .text_color(t.text_primary)
                                    .hover(|s| s.bg(t.bg_hover).border_color(t.border_focus))
                                    .cursor_pointer()
                                    .on_click(move |_, window, cx| {
                                        insert_cb(snippet_code, window, cx);
                                    })
                                    .child(label)
                            })),
                    ),
            )
    }
}

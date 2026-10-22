use crate::state::{ResponseState, ResponseViewMode};
use gpui::{App, AppContext as _, Entity, Window};
use gpui_component::input::{EditorState as CodeEditorState, TextareaState};
use std::rc::Rc;

type HighlighterFactory =
    Rc<dyn Fn(&str) -> Option<Box<dyn gpui_component::input::InputHighlighter>>>;

pub struct InspectorDisplays {
    pub response_editor: Entity<CodeEditorState>,
    pub request_url_textarea: Entity<TextareaState>,
    pub request_body_editor: Entity<CodeEditorState>,
    pub last_synced_response_id: Option<String>,
    pub last_synced_response_view_mode: Option<ResponseViewMode>,
    pub last_synced_example_name: Option<String>,
}

impl InspectorDisplays {
    pub fn new(window: &mut Window, highlighter_factory: HighlighterFactory, cx: &mut App) -> Self {
        let response_editor = cx.new(|cx| {
            let mut state = CodeEditorState::new(window, cx);
            state.set_highlighter_factory(highlighter_factory.clone(), cx);
            state
                .language("json")
                .line_number(true)
                .folding(true)
                .soft_wrap(false)
        });

        let request_url_textarea = cx.new(|cx| TextareaState::new(window, cx).auto_grow(1, 8));

        let request_body_editor = cx.new(|cx| {
            let mut state = CodeEditorState::new(window, cx);
            state.set_highlighter_factory(highlighter_factory, cx);
            state.language("json").line_number(true).folding(false)
        });

        Self {
            response_editor,
            request_url_textarea,
            request_body_editor,
            last_synced_response_id: None,
            last_synced_response_view_mode: None,
            last_synced_example_name: None,
        }
    }

    pub fn sync_response_editor(
        &mut self,
        response_state: &Entity<ResponseState>,
        window: &mut Window,
        cx: &mut App,
    ) {
        let resp_s = response_state.read(cx);
        let current_resp_id = resp_s.current_response.as_ref().map(|r| r.id.clone());
        let current_mode = resp_s.view_mode;
        let current_ex = resp_s.active_example_name.clone();

        let resp_changed = current_resp_id != self.last_synced_response_id
            || Some(current_mode) != self.last_synced_response_view_mode
            || current_ex != self.last_synced_example_name;

        if resp_changed {
            self.last_synced_response_id = current_resp_id;
            self.last_synced_response_view_mode = Some(current_mode);
            self.last_synced_example_name = current_ex;

            let (display_text, is_json, is_xml) = if let Some(fmt) = &resp_s.formatted_body {
                (fmt.display_text.to_string(), fmt.is_json, fmt.is_xml)
            } else if let Some(resp) = &resp_s.current_response {
                let text = String::from_utf8_lossy(&resp.body).to_string();
                let is_j = resp.is_json()
                    || text.trim_start().starts_with('{')
                    || text.trim_start().starts_with('[');
                let is_x = !is_j
                    && (resp.is_xml()
                        || text.trim_start().starts_with("<?xml")
                        || (text.trim_start().starts_with('<') && text.trim_end().ends_with('>')));
                (text, is_j, is_x)
            } else {
                (String::new(), false, false)
            };

            let lang = if is_json {
                "json"
            } else if is_xml {
                "xml"
            } else {
                "plaintext"
            };

            self.response_editor.update(cx, |txt, cx| {
                txt.set_value(display_text, window, cx);
                txt.set_highlighter(lang, cx);
                txt.set_readonly(true, cx);
            });
        }
    }

    pub fn sync_sent_request_preview(
        &mut self,
        response_state: &Entity<ResponseState>,
        window: &mut Window,
        cx: &mut App,
    ) {
        let (sent_url, body_text, lang) = {
            let resp_s = response_state.read(cx);
            if let Some(resp) = &resp_s.current_response {
                if let Some(sent) = &resp.request_sent {
                    let (body_text, lang) = if let Some(preview) = &sent.body_preview {
                        let is_j = preview.trim_start().starts_with('{')
                            || preview.trim_start().starts_with('[');
                        let text = if is_j {
                            serde_json::from_str::<serde_json::Value>(preview)
                                .map(|v| {
                                    serde_json::to_string_pretty(&v).unwrap_or(preview.clone())
                                })
                                .unwrap_or_else(|_| preview.clone())
                        } else {
                            preview.clone()
                        };
                        (text, if is_j { "json" } else { "plaintext" })
                    } else {
                        (String::new(), "plaintext")
                    };
                    (Some(sent.url.clone()), Some(body_text), Some(lang))
                } else {
                    (None, None, None)
                }
            } else {
                (None, None, None)
            }
        };

        if let Some(sent_url) = sent_url {
            self.request_url_textarea.update(cx, |txt, cx| {
                if txt.value() != sent_url {
                    txt.set_value(sent_url, window, cx);
                    txt.set_readonly(true, cx);
                }
            });
        }
        if let (Some(body_text), Some(lang)) = (body_text, lang) {
            self.request_body_editor.update(cx, |txt, cx| {
                if txt.value() != body_text {
                    txt.set_value(body_text, window, cx);
                    txt.set_highlighter(lang, cx);
                    txt.set_readonly(true, cx);
                }
            });
        }
    }
}

use std::ops::Range;
use crate::components::kv_table::KvTable;
use crate::state::{EditorState, ToastState, WorkspaceState};
use crate::theme::theme;
use gpui::prelude::FluentBuilder as _;
use gpui::{
    div, px, App, AppContext as _, Context, Entity, HighlightStyle, InteractiveElement as _,
    IntoElement, ParentElement, RenderOnce, SharedString, StatefulInteractiveElement as _,
    Styled, Window,
};
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::input::{
    Editor, EditorState as CodeEditorState, FoldRange, HighlightStyleResolver, InputEdit,
    InputHighlighter, Rope, Textarea, TextareaState,
};
use gpui_component::menu::{DropdownMenu as _, PopupMenuItem};
use gpui_component::Icon;
use gpui_kit::assets::IconName;
use gpui_component::scroll::ScrollableElement as _;
use restclient_core::models::{KeyValuePair, MultipartItem, RequestBody};

pub struct JsonHighlighter {
    pub text: Rope,
    pub cached_str: String,
}

fn persist_active_request_without_redraw(
    editor: &Entity<EditorState>,
    workspace: Option<&Entity<WorkspaceState>>,
    cx: &mut App,
) {
    let (Some(workspace), Some(req_id)) = (
        workspace,
        editor.read(cx).active_request.as_ref().map(|r| r.id.clone()),
    ) else {
        return;
    };
    let is_scratch = editor.read(cx).active_request.as_ref().map(|r| r.workspace_id == "scratch").unwrap_or(true);
    if !is_scratch {
        let ed = editor.clone();
        workspace.update(cx, |ws, cx| ws.schedule_save_active_request(req_id, ed, cx));
    }
}

impl JsonHighlighter {
    pub fn new() -> Self {
        Self {
            text: Rope::new(),
            cached_str: String::new(),
        }
    }
}

impl Default for JsonHighlighter {
    fn default() -> Self {
        Self::new()
    }
}

fn clamp_char_boundary_left(s: &str, mut idx: usize) -> usize {
    if idx >= s.len() {
        return s.len();
    }
    while idx > 0 && !s.is_char_boundary(idx) {
        idx -= 1;
    }
    idx
}

fn clamp_char_boundary_right(s: &str, mut idx: usize) -> usize {
    if idx >= s.len() {
        return s.len();
    }
    while idx < s.len() && !s.is_char_boundary(idx) {
        idx += 1;
    }
    idx
}

impl InputHighlighter for JsonHighlighter {
    fn language(&self) -> SharedString {
        "json".into()
    }

    fn update(
        &mut self,
        _edit: Option<InputEdit>,
        text: &Rope,
        _folding: bool,
        _window: &mut Window,
        _cx: &mut Context<CodeEditorState>,
    ) {
        self.text = text.clone();
        self.cached_str = text.to_string();
    }

    fn styles(
        &self,
        range: &Range<usize>,
        _resolver: &dyn HighlightStyleResolver,
    ) -> Vec<(Range<usize>, HighlightStyle)> {
        let temp_str;
        let full_str: &str = if !self.cached_str.is_empty() {
            &self.cached_str
        } else if self.text.len() != 0 {
            temp_str = self.text.to_string();
            &temp_str
        } else {
            ""
        };
        if full_str.is_empty() || range.is_empty() {
            return vec![(range.clone(), HighlightStyle::default())];
        }

        let start = clamp_char_boundary_left(full_str, range.start);
        let end = clamp_char_boundary_right(full_str, range.end.min(full_str.len()));

        if start >= end {
            return vec![(range.clone(), HighlightStyle::default())];
        }

        let slice_str = &full_str[start..end];
        let slice_bytes = slice_str.as_bytes();
        let len = slice_bytes.len();
        let mut i = 0;
        let mut raw_tokens = Vec::new();
        let t = theme();

        while i < len {
            let b = slice_bytes[i];
            if b == b'"' {
                let tok_start = i;
                i += 1;
                let mut escaped = false;
                while i < len {
                    let ch = slice_bytes[i];
                    if escaped {
                        escaped = false;
                        i += 1;
                    } else if ch == b'\\' {
                        escaped = true;
                        i += 1;
                    } else if ch == b'"' {
                        i += 1;
                        break;
                    } else if ch == b'\n' {
                        break;
                    } else {
                        i += 1;
                    }
                }
                let tok_end = i;
                let mut j = tok_end;
                while j < len
                    && (slice_bytes[j] == b' '
                        || slice_bytes[j] == b'\t'
                        || slice_bytes[j] == b'\r'
                        || slice_bytes[j] == b'\n')
                {
                    j += 1;
                }
                let is_key = j < len && slice_bytes[j] == b':';
                let color = if is_key { t.json_key } else { t.json_string };
                let str_content = &slice_str[tok_start..tok_end];
                if str_content.contains("{{") && str_content.contains("}}") {
                    let mut cur_pos = 0;
                    while let Some(open_idx) = str_content[cur_pos..].find("{{") {
                        let abs_open = cur_pos + open_idx;
                        if let Some(close_idx) = str_content[abs_open..].find("}}") {
                            let abs_close = abs_open + close_idx + 2;
                            if abs_open > cur_pos {
                                raw_tokens.push((
                                    start + tok_start + cur_pos..start + tok_start + abs_open,
                                    HighlightStyle {
                                        color: Some(color),
                                        ..Default::default()
                                    },
                                ));
                            }
                            raw_tokens.push((
                                start + tok_start + abs_open..start + tok_start + abs_close,
                                HighlightStyle {
                                    color: Some(t.var_resolved),
                                    font_weight: Some(gpui::FontWeight::SEMIBOLD),
                                    ..Default::default()
                                },
                            ));
                            cur_pos = abs_close;
                        } else {
                            break;
                        }
                    }
                    if cur_pos < str_content.len() {
                        raw_tokens.push((
                            start + tok_start + cur_pos..start + tok_start + str_content.len(),
                            HighlightStyle {
                                color: Some(color),
                                ..Default::default()
                            },
                        ));
                    }
                } else {
                    raw_tokens.push((
                        start + tok_start..start + tok_end,
                        HighlightStyle {
                            color: Some(color),
                            ..Default::default()
                        },
                    ));
                }
            } else if matches!(b, b'{' | b'}' | b'[' | b']' | b':' | b',') {
                raw_tokens.push((
                    start + i..start + i + 1,
                    HighlightStyle {
                        color: Some(t.json_punctuation),
                        ..Default::default()
                    },
                ));
                i += 1;
            } else if b == b'-' || b.is_ascii_digit() {
                let num_start = i;
                i += 1;
                while i < len
                    && (slice_bytes[i].is_ascii_digit()
                        || matches!(slice_bytes[i], b'.' | b'e' | b'E' | b'+' | b'-'))
                {
                    i += 1;
                }
                raw_tokens.push((
                    start + num_start..start + i,
                    HighlightStyle {
                        color: Some(t.json_number),
                        ..Default::default()
                    },
                ));
            } else if slice_str[i..].starts_with("true")
                && (i + 4 == len
                    || !slice_bytes[i + 4].is_ascii_alphanumeric() && slice_bytes[i + 4] != b'_')
            {
                raw_tokens.push((
                    start + i..start + i + 4,
                    HighlightStyle {
                        color: Some(t.json_boolean),
                        font_weight: Some(gpui::FontWeight::SEMIBOLD),
                        ..Default::default()
                    },
                ));
                i += 4;
            } else if slice_str[i..].starts_with("false")
                && (i + 5 == len
                    || !slice_bytes[i + 5].is_ascii_alphanumeric() && slice_bytes[i + 5] != b'_')
            {
                raw_tokens.push((
                    start + i..start + i + 5,
                    HighlightStyle {
                        color: Some(t.json_boolean),
                        font_weight: Some(gpui::FontWeight::SEMIBOLD),
                        ..Default::default()
                    },
                ));
                i += 5;
            } else if slice_str[i..].starts_with("null")
                && (i + 4 == len
                    || !slice_bytes[i + 4].is_ascii_alphanumeric() && slice_bytes[i + 4] != b'_')
            {
                raw_tokens.push((
                    start + i..start + i + 4,
                    HighlightStyle {
                        color: Some(t.json_null),
                        font_style: Some(gpui::FontStyle::Italic),
                        ..Default::default()
                    },
                ));
                i += 4;
            } else {
                let ch = slice_str[i..].chars().next().unwrap();
                i += ch.len_utf8();
            }
        }

        let mut full_styles = Vec::new();
        let mut cursor = range.start;

        for (token_range, style) in raw_tokens {
            if token_range.start > cursor {
                full_styles.push((cursor..token_range.start, HighlightStyle::default()));
            }
            cursor = token_range.end;
            full_styles.push((token_range, style));
        }

        if cursor < range.end {
            full_styles.push((cursor..range.end, HighlightStyle::default()));
        }

        full_styles
    }

    fn fold_ranges(&self, text: &Rope) -> Vec<FoldRange> {
        let mut folds = Vec::new();
        let mut stack = Vec::new();
        let s = text.to_string();
        let mut current_line = 0;
        let mut in_string = false;
        let mut escaped = false;
        let bytes = s.as_bytes();
        let len = bytes.len();
        let mut i = 0;

        while i < len {
            let b = bytes[i];
            if in_string {
                if escaped {
                    escaped = false;
                } else if b == b'\\' {
                    escaped = true;
                } else if b == b'"' {
                    in_string = false;
                }
            } else {
                match b {
                    b'"' => in_string = true,
                    b'{' | b'[' => stack.push(current_line),
                    b'}' | b']' => {
                        if let Some(start_line) = stack.pop() {
                            if current_line > start_line {
                                folds.push(FoldRange::new(start_line, current_line));
                            }
                        }
                    }
                    b'\n' => current_line += 1,
                    _ => {}
                }
            }
            if in_string && b == b'\n' {
                current_line += 1;
            }
            i += 1;
        }
        folds.sort_by(|a, b| a.start_line.cmp(&b.start_line).then_with(|| b.end_line.cmp(&a.end_line)));
        folds.dedup_by_key(|f| f.start_line);
        folds
    }
}

pub struct XmlHighlighter {
    pub text: Rope,
    pub cached_str: String,
}

impl XmlHighlighter {
    pub fn new() -> Self {
        Self {
            text: Rope::new(),
            cached_str: String::new(),
        }
    }
}

impl Default for XmlHighlighter {
    fn default() -> Self {
        Self::new()
    }
}

impl InputHighlighter for XmlHighlighter {
    fn language(&self) -> SharedString {
        "xml".into()
    }

    fn update(
        &mut self,
        _edit: Option<InputEdit>,
        text: &Rope,
        _folding: bool,
        _window: &mut Window,
        _cx: &mut Context<CodeEditorState>,
    ) {
        self.text = text.clone();
        self.cached_str = text.to_string();
    }

    fn styles(
        &self,
        range: &Range<usize>,
        _resolver: &dyn HighlightStyleResolver,
    ) -> Vec<(Range<usize>, HighlightStyle)> {
        let temp_str;
        let full_str: &str = if !self.cached_str.is_empty() {
            &self.cached_str
        } else if self.text.len() != 0 {
            temp_str = self.text.to_string();
            &temp_str
        } else {
            ""
        };
        if full_str.is_empty() || range.is_empty() {
            return vec![(range.clone(), HighlightStyle::default())];
        }

        let start = clamp_char_boundary_left(full_str, range.start);
        let end = clamp_char_boundary_right(full_str, range.end.min(full_str.len()));

        if start >= end {
            return vec![(range.clone(), HighlightStyle::default())];
        }

        let slice_str = &full_str[start..end];
        let slice_bytes = slice_str.as_bytes();
        let len = slice_bytes.len();
        let mut i = 0;
        let mut raw_tokens = Vec::new();
        let t = theme();

        while i < len {
            // Check comment <!-- ... -->
            if i + 3 < len && &slice_bytes[i..i + 4] == b"<!--" {
                let c_start = i;
                i += 4;
                while i + 2 < len && &slice_bytes[i..i + 3] != b"-->" {
                    i += 1;
                }
                if i + 2 < len {
                    i += 3;
                }
                raw_tokens.push((
                    start + c_start..start + i,
                    HighlightStyle {
                        color: Some(t.text_muted),
                        font_style: Some(gpui::FontStyle::Italic),
                        ..Default::default()
                    },
                ));
            }
            // Check CDATA <![CDATA[ ... ]]>
            else if i + 8 < len && &slice_bytes[i..i + 9] == b"<![CDATA[" {
                let cd_start = i;
                i += 9;
                while i + 2 < len && &slice_bytes[i..i + 3] != b"]]>" {
                    i += 1;
                }
                if i + 2 < len {
                    i += 3;
                }
                raw_tokens.push((
                    start + cd_start..start + i,
                    HighlightStyle {
                        color: Some(t.json_number),
                        ..Default::default()
                    },
                ));
            }
            // Check XML Declaration / Processing instruction <? ... ?>
            else if i + 1 < len && &slice_bytes[i..i + 2] == b"<?" {
                let d_start = i;
                i += 2;
                while i + 1 < len && &slice_bytes[i..i + 2] != b"?>" {
                    i += 1;
                }
                if i + 1 < len {
                    i += 2;
                }
                raw_tokens.push((
                    start + d_start..start + i,
                    HighlightStyle {
                        color: Some(t.json_null),
                        font_style: Some(gpui::FontStyle::Italic),
                        ..Default::default()
                    },
                ));
            }
            // Check Tags: <tag ...> or </tag>
            else if slice_bytes[i] == b'<' {
                let tag_start = i;
                let is_closing = i + 1 < len && slice_bytes[i + 1] == b'/';
                let tag_bracket_len = if is_closing { 2 } else { 1 };
                raw_tokens.push((
                    start + tag_start..start + tag_start + tag_bracket_len,
                    HighlightStyle {
                        color: Some(t.json_punctuation),
                        ..Default::default()
                    },
                ));
                i += tag_bracket_len;

                // Tag name
                let name_start = i;
                while i < len && (slice_bytes[i].is_ascii_alphanumeric() || slice_bytes[i] == b':' || slice_bytes[i] == b'_' || slice_bytes[i] == b'-') {
                    i += 1;
                }
                if i > name_start {
                    raw_tokens.push((
                        start + name_start..start + i,
                        HighlightStyle {
                            color: Some(t.primary),
                            font_weight: Some(gpui::FontWeight::SEMIBOLD),
                            ..Default::default()
                        },
                    ));
                }

                // Attributes until '>' or '/>'
                while i < len && slice_bytes[i] != b'>' {
                    if slice_bytes[i] == b'/' && i + 1 < len && slice_bytes[i + 1] == b'>' {
                        raw_tokens.push((
                            start + i..start + i + 2,
                            HighlightStyle {
                                color: Some(t.json_punctuation),
                                ..Default::default()
                            },
                        ));
                        i += 2;
                        break;
                    }

                    // Attribute name
                    if slice_bytes[i].is_ascii_alphabetic() || slice_bytes[i] == b'_' || slice_bytes[i] == b':' {
                        let attr_start = i;
                        while i < len && (slice_bytes[i].is_ascii_alphanumeric() || slice_bytes[i] == b'_' || slice_bytes[i] == b':' || slice_bytes[i] == b'-') {
                            i += 1;
                        }
                        raw_tokens.push((
                            start + attr_start..start + i,
                            HighlightStyle {
                                color: Some(t.json_key),
                                ..Default::default()
                            },
                        ));
                        continue;
                    }

                    // '='
                    if slice_bytes[i] == b'=' {
                        raw_tokens.push((
                            start + i..start + i + 1,
                            HighlightStyle {
                                color: Some(t.json_punctuation),
                                ..Default::default()
                            },
                        ));
                        i += 1;
                        continue;
                    }

                    // Attribute string value: "..." or '...'
                    if slice_bytes[i] == b'"' || slice_bytes[i] == b'\'' {
                        let quote = slice_bytes[i];
                        let val_start = i;
                        i += 1;
                        while i < len && slice_bytes[i] != quote && slice_bytes[i] != b'\n' {
                            i += 1;
                        }
                        if i < len && slice_bytes[i] == quote {
                            i += 1;
                        }
                        let val_str = &slice_str[val_start..i];
                        if val_str.contains("{{") && val_str.contains("}}") {
                            let mut cur_pos = 0;
                            while let Some(open_idx) = val_str[cur_pos..].find("{{") {
                                let abs_open = cur_pos + open_idx;
                                if let Some(close_idx) = val_str[abs_open..].find("}}") {
                                    let abs_close = abs_open + close_idx + 2;
                                    if abs_open > cur_pos {
                                        raw_tokens.push((
                                            start + val_start + cur_pos..start + val_start + abs_open,
                                            HighlightStyle {
                                                color: Some(t.json_string),
                                                ..Default::default()
                                            },
                                        ));
                                    }
                                    raw_tokens.push((
                                        start + val_start + abs_open..start + val_start + abs_close,
                                        HighlightStyle {
                                            color: Some(t.var_resolved),
                                            font_weight: Some(gpui::FontWeight::SEMIBOLD),
                                            ..Default::default()
                                        },
                                    ));
                                    cur_pos = abs_close;
                                } else {
                                    break;
                                }
                            }
                            if cur_pos < val_str.len() {
                                raw_tokens.push((
                                    start + val_start + cur_pos..start + val_start + val_str.len(),
                                    HighlightStyle {
                                        color: Some(t.json_string),
                                        ..Default::default()
                                    },
                                ));
                            }
                        } else {
                            raw_tokens.push((
                                start + val_start..start + i,
                                HighlightStyle {
                                    color: Some(t.json_string),
                                    ..Default::default()
                                },
                            ));
                        }
                        continue;
                    }

                    i += 1;
                }

                if i < len && slice_bytes[i] == b'>' {
                    raw_tokens.push((
                        start + i..start + i + 1,
                        HighlightStyle {
                            color: Some(t.json_punctuation),
                            ..Default::default()
                        },
                    ));
                    i += 1;
                }
            }
            // Check {{var}} outside tags
            else if i + 1 < len && slice_bytes[i] == b'{' && slice_bytes[i + 1] == b'{' {
                let v_start = i;
                if let Some(close_idx) = slice_str[i..].find("}}") {
                    let v_end = i + close_idx + 2;
                    raw_tokens.push((
                        start + v_start..start + v_end,
                        HighlightStyle {
                            color: Some(t.var_resolved),
                            font_weight: Some(gpui::FontWeight::SEMIBOLD),
                            ..Default::default()
                        },
                    ));
                    i = v_end;
                } else {
                    i += 2;
                }
            } else {
                let ch = slice_str[i..].chars().next().unwrap();
                i += ch.len_utf8();
            }
        }

        let mut full_styles = Vec::new();
        let mut cursor = range.start;

        for (token_range, style) in raw_tokens {
            if token_range.start > cursor {
                full_styles.push((cursor..token_range.start, HighlightStyle::default()));
            }
            cursor = token_range.end;
            full_styles.push((token_range, style));
        }

        if cursor < range.end {
            full_styles.push((cursor..range.end, HighlightStyle::default()));
        }

        full_styles
    }

    fn fold_ranges(&self, text: &Rope) -> Vec<FoldRange> {
        let mut folds = Vec::new();
        let mut tag_stack: Vec<(String, usize)> = Vec::new();
        let s = text.to_string();
        let bytes = s.as_bytes();
        let len = bytes.len();
        let mut current_line = 0;
        let mut i = 0;

        while i < len {
            if bytes[i] == b'\n' {
                current_line += 1;
                i += 1;
                continue;
            }

            // Check comment <!-- ... -->
            if i + 3 < len && &bytes[i..i + 4] == b"<!--" {
                let comment_start = current_line;
                i += 4;
                while i + 2 < len && &bytes[i..i + 3] != b"-->" {
                    if bytes[i] == b'\n' {
                        current_line += 1;
                    }
                    i += 1;
                }
                if i + 2 < len {
                    i += 3;
                }
                if current_line > comment_start {
                    folds.push(FoldRange::new(comment_start, current_line));
                }
                continue;
            }

            // Check CDATA <![CDATA[ ... ]]>
            if i + 8 < len && &bytes[i..i + 9] == b"<![CDATA[" {
                let cdata_start = current_line;
                i += 9;
                while i + 2 < len && &bytes[i..i + 3] != b"]]>" {
                    if bytes[i] == b'\n' {
                        current_line += 1;
                    }
                    i += 1;
                }
                if i + 2 < len {
                    i += 3;
                }
                if current_line > cdata_start {
                    folds.push(FoldRange::new(cdata_start, current_line));
                }
                continue;
            }

            // Check XML Declaration / Processing Instruction <? ... ?> or <!DOCTYPE ... >
            if i + 1 < len && (bytes[i] == b'<' && (bytes[i + 1] == b'?' || bytes[i + 1] == b'!')) {
                let decl_start = current_line;
                let is_pi = bytes[i + 1] == b'?';
                i += 2;
                while i < len {
                    if bytes[i] == b'\n' {
                        current_line += 1;
                    }
                    if is_pi && i + 1 < len && bytes[i] == b'?' && bytes[i + 1] == b'>' {
                        i += 2;
                        break;
                    } else if !is_pi && bytes[i] == b'>' {
                        i += 1;
                        break;
                    }
                    i += 1;
                }
                if current_line > decl_start {
                    folds.push(FoldRange::new(decl_start, current_line));
                }
                continue;
            }

            // Check Closing tag: </tag_name>
            if i + 1 < len && bytes[i] == b'<' && bytes[i + 1] == b'/' {
                i += 2;
                let name_start = i;
                while i < len && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b':' || bytes[i] == b'_' || bytes[i] == b'-') {
                    i += 1;
                }
                let tag_name = &s[name_start..i];
                while i < len && bytes[i] != b'>' {
                    if bytes[i] == b'\n' {
                        current_line += 1;
                    }
                    i += 1;
                }
                if i < len && bytes[i] == b'>' {
                    i += 1;
                }

                if let Some(pos) = tag_stack.iter().rposition(|(name, _)| name == tag_name) {
                    let (_, start_line) = tag_stack.remove(pos);
                    if current_line > start_line {
                        folds.push(FoldRange::new(start_line, current_line));
                    }
                }
                continue;
            }

            // Check Opening tag: <tag_name ...> or <tag_name ... />
            if bytes[i] == b'<' && i + 1 < len && (bytes[i + 1].is_ascii_alphabetic() || bytes[i + 1] == b'_' || bytes[i + 1] == b':') {
                let tag_start_line = current_line;
                i += 1;
                let name_start = i;
                while i < len && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b':' || bytes[i] == b'_' || bytes[i] == b'-') {
                    i += 1;
                }
                let tag_name = s[name_start..i].to_string();

                let mut self_closing = false;
                let mut in_quote: Option<u8> = None;

                while i < len {
                    let b = bytes[i];
                    if b == b'\n' {
                        current_line += 1;
                    }
                    if let Some(q) = in_quote {
                        if b == q {
                            in_quote = None;
                        }
                    } else if b == b'"' || b == b'\'' {
                        in_quote = Some(b);
                    } else if b == b'/' && i + 1 < len && bytes[i + 1] == b'>' {
                        self_closing = true;
                        i += 2;
                        break;
                    } else if b == b'>' {
                        i += 1;
                        break;
                    }
                    i += 1;
                }

                if !self_closing {
                    tag_stack.push((tag_name, tag_start_line));
                }
                continue;
            }

            i += 1;
        }

        folds.sort_by(|a, b| a.start_line.cmp(&b.start_line).then_with(|| b.end_line.cmp(&a.end_line)));
        folds.dedup_by_key(|f| f.start_line);
        folds
    }
}

fn format_file_size(path_str: &str) -> String {
    if path_str.is_empty() {
        return "".to_string();
    }
    if let Ok(meta) = std::fs::metadata(path_str) {
        let bytes = meta.len();
        if bytes < 1024 {
            format!("{} B", bytes)
        } else if bytes < 1024 * 1024 {
            format!("{:.1} KB", bytes as f64 / 1024.0)
        } else {
            format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
        }
    } else {
        "File not found".to_string()
    }
}

pub fn format_xml(xml: &str) -> Result<String, String> {
    use quick_xml::reader::Reader;
    use quick_xml::writer::Writer;
    use std::io::Cursor;

    let trimmed = xml.trim();
    if trimmed.is_empty() {
        return Ok(String::new());
    }

    let mut reader = Reader::from_str(trimmed);
    reader.config_mut().trim_text(true);

    let mut writer = Writer::new_with_indent(Cursor::new(Vec::new()), b' ', 2);
    loop {
        match reader.read_event() {
            Ok(quick_xml::events::Event::Eof) => break,
            Ok(event) => {
                writer
                    .write_event(event)
                    .map_err(|e| format!("XML write error: {}", e))?;
            }
            Err(e) => return Err(format!("XML syntax error: {}", e)),
        }
    }

    let result = writer.into_inner().into_inner();
    String::from_utf8(result).map_err(|e| format!("UTF-8 conversion error: {}", e))
}

#[derive(IntoElement)]
pub struct BodyEditor {
    editor_state: Entity<EditorState>,
    json_editor: Entity<CodeEditorState>,
    xml_editor: Entity<CodeEditorState>,
    raw_textarea: Entity<TextareaState>,
    workspace_state: Option<Entity<WorkspaceState>>,
    toast_state: Option<Entity<ToastState>>,
}

impl BodyEditor {
    pub fn new(
        editor_state: Entity<EditorState>,
        json_editor: Entity<CodeEditorState>,
        xml_editor: Entity<CodeEditorState>,
        raw_textarea: Entity<TextareaState>,
    ) -> Self {
        Self {
            editor_state,
            json_editor,
            xml_editor,
            raw_textarea,
            workspace_state: None,
            toast_state: None,
        }
    }

    pub fn workspace_state(mut self, ws: Entity<WorkspaceState>) -> Self {
        self.workspace_state = Some(ws);
        self
    }

    pub fn toast_state(mut self, ts: Entity<ToastState>) -> Self {
        self.toast_state = Some(ts);
        self
    }
}

impl RenderOnce for BodyEditor {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = theme();
        let ed = self.editor_state.read(cx);
        let ed_entity = self.editor_state.clone();

        let req_id_str = ed.active_request.as_ref().map(|r| r.id.clone()).unwrap_or_else(|| "scratch".to_string());
        let body_ref = ed.active_request.as_ref().map(|r| &r.body);

        let (is_none, is_json, is_xml, is_text, is_multipart, is_form, is_binary) = match body_ref {
            None | Some(RequestBody::None) => (true, false, false, false, false, false, false),
            Some(RequestBody::Json(_)) => (false, true, false, false, false, false, false),
            Some(RequestBody::Raw { content_type, .. })
                if content_type == "application/xml" || content_type == "text/xml" =>
            {
                (false, false, true, false, false, false, false)
            }
            Some(RequestBody::Raw { .. }) => (false, false, false, true, false, false, false),
            Some(RequestBody::Multipart(_)) => (false, false, false, false, true, false, false),
            Some(RequestBody::FormUrlEncoded(_)) => (false, false, false, false, false, true, false),
            Some(RequestBody::Binary { .. }) => (false, false, false, false, false, false, true),
        };

        let current_label = if is_none {
            "None"
        } else if is_json {
            "JSON"
        } else if is_xml {
            "XML"
        } else if is_text {
            "Text"
        } else if is_multipart {
            "Multipart"
        } else if is_form {
            "Form URL-Encoded"
        } else if is_binary {
            "Binary Stream"
        } else {
            "None"
        };

        div()
            .size_full()
            .flex()
            .flex_col()
            // Top format selector bar: Dropdown [Body Type ▾] (left) and Format Button (right)
            .child(
                div()
                    .h(px(32.))
                    .px(px(8.))
                    .border_b_1()
                    .border_color(t.border)
                    .bg(t.bg_editor)
                    .flex()
                    .items_center()
                    .justify_between()
                    .child({
                        let ed_menu = ed_entity.clone();
                        Button::new("btn-body-type-dropdown")
                            .ghost()
                            .compact()
                            .px(px(8.))
                            .py(px(2.))
                            .h(px(24.))
                            .rounded(px(4.))
                            .bg(t.bg_card)
                            .border_1()
                            .border_color(t.border)
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(6.))
                                    .child(
                                        div()
                                            .text_xs()
                                            .font_weight(gpui::FontWeight::MEDIUM)
                                            .text_color(t.text_primary)
                                            .child(current_label),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(t.text_muted)
                                            .child("▾"),
                                    ),
                            )
                            .dropdown_menu(move |menu, _window, _cx| {
                                let mut m = menu;

                                // 1. None
                                let ed_none = ed_menu.clone();
                                m = m.item(
                                    PopupMenuItem::new("None")
                                        .checked(is_none)
                                        .on_click(move |_, _, cx| {
                                            ed_none.update(cx, |state, cx| {
                                                state.set_body_type(RequestBody::None, cx);
                                            });
                                        }),
                                );

                                // 2. Text group: JSON, XML, Text
                                m = m.separator().label("Text");

                                let ed_json = ed_menu.clone();
                                m = m.item(
                                    PopupMenuItem::new("JSON")
                                        .checked(is_json)
                                        .on_click(move |_, _, cx| {
                                            ed_json.update(cx, |state, cx| {
                                                let current_str = match &state.active_request.as_ref().map(|r| &r.body) {
                                                    Some(RequestBody::Json(s)) if !s.trim().is_empty() => s.clone(),
                                                    Some(RequestBody::Raw { content, .. }) if !content.trim().is_empty() => content.clone(),
                                                    _ => "{\n  \n}".to_string(),
                                                };
                                                state.set_body_type(RequestBody::Json(current_str), cx);
                                            });
                                        }),
                                );

                                let ed_xml = ed_menu.clone();
                                m = m.item(
                                    PopupMenuItem::new("XML")
                                        .checked(is_xml)
                                        .on_click(move |_, _, cx| {
                                            ed_xml.update(cx, |state, cx| {
                                                let current_str = match &state.active_request.as_ref().map(|r| &r.body) {
                                                    Some(RequestBody::Raw { content, .. }) if !content.trim().is_empty() => content.clone(),
                                                    _ => "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<root>\n  \n</root>".to_string(),
                                                };
                                                state.set_body_type(
                                                    RequestBody::Raw {
                                                        content: current_str,
                                                        content_type: "application/xml".to_string(),
                                                    },
                                                    cx,
                                                );
                                            });
                                        }),
                                );

                                let ed_text = ed_menu.clone();
                                m = m.item(
                                    PopupMenuItem::new("Text")
                                        .checked(is_text)
                                        .on_click(move |_, _, cx| {
                                            ed_text.update(cx, |state, cx| {
                                                let current_str = match &state.active_request.as_ref().map(|r| &r.body) {
                                                    Some(RequestBody::Raw { content, .. }) => content.clone(),
                                                    Some(RequestBody::Json(s)) => s.clone(),
                                                    _ => "".to_string(),
                                                };
                                                state.set_body_type(
                                                    RequestBody::Raw {
                                                        content: current_str,
                                                        content_type: "text/plain".to_string(),
                                                    },
                                                    cx,
                                                );
                                            });
                                        }),
                                );

                                // 3. Form Data group: Multipart, Form URL-Encoded
                                m = m.separator().label("Form Data");

                                let ed_mp = ed_menu.clone();
                                m = m.item(
                                    PopupMenuItem::new("Multipart")
                                        .checked(is_multipart)
                                        .on_click(move |_, _, cx| {
                                            ed_mp.update(cx, |state, cx| {
                                                let current_items = match &state.active_request.as_ref().map(|r| &r.body) {
                                                    Some(RequestBody::Multipart(items)) if !items.is_empty() => items.clone(),
                                                    _ => vec![MultipartItem::default()],
                                                };
                                                state.set_body_type(RequestBody::Multipart(current_items), cx);
                                            });
                                        }),
                                );

                                let ed_form = ed_menu.clone();
                                m = m.item(
                                    PopupMenuItem::new("Form URL-Encoded")
                                        .checked(is_form)
                                        .on_click(move |_, _, cx| {
                                            ed_form.update(cx, |state, cx| {
                                                let current_pairs = match &state.active_request.as_ref().map(|r| &r.body) {
                                                    Some(RequestBody::FormUrlEncoded(pairs)) if !pairs.is_empty() => pairs.clone(),
                                                    _ => vec![KeyValuePair::new("", "")],
                                                };
                                                state.set_body_type(RequestBody::FormUrlEncoded(current_pairs), cx);
                                            });
                                        }),
                                );

                                // 4. Binary group: Binary Stream
                                m = m.separator().label("Binary");

                                let ed_bin = ed_menu.clone();
                                m = m.item(
                                    PopupMenuItem::new("Binary Stream")
                                        .checked(is_binary)
                                        .on_click(move |_, _, cx| {
                                            ed_bin.update(cx, |state, cx| {
                                                let current_path = match &state.active_request.as_ref().map(|r| &r.body) {
                                                    Some(RequestBody::Binary { file_path }) => file_path.clone(),
                                                    _ => "".to_string(),
                                                };
                                                state.set_body_type(RequestBody::Binary { file_path: current_path }, cx);
                                            });
                                        }),
                                );

                                m
                            })
                    })
                    // Quick Action button in toolbar (Right side)
                    .child({
                        if is_json {
                            let ed_c = ed_entity.clone();
                            let j_text = self.json_editor.clone();
                            let toast_opt = self.toast_state.clone();
                            div()
                                .flex()
                                .items_center()
                                .child(
                                    div()
                                        .id("btn-format-json-body")
                                        .px(px(8.))
                                        .py(px(3.))
                                        .rounded(px(4.))
                                        .bg(t.bg_input)
                                        .border_1()
                                        .border_color(t.border)
                                        .cursor_pointer()
                                        .hover(|s| s.bg(t.bg_hover))
                                        .text_xs()
                                        .font_weight(gpui::FontWeight::MEDIUM)
                                        .text_color(t.text_secondary)
                                        .on_click(move |_, window, cx| {
                                            let raw = j_text.read(cx).value().to_string();
                                            match serde_json::from_str::<serde_json::Value>(&raw) {
                                                Ok(val) => {
                                                    if let Ok(pretty) = serde_json::to_string_pretty(&val) {
                                                        j_text.update(cx, |txt, cx| {
                                                            txt.set_value(pretty.clone(), window, cx);
                                                        });
                                                        ed_c.update(cx, |state, cx| {
                                                            state.set_body_json(pretty, cx);
                                                        });
                                                    }
                                                }
                                                Err(e) => {
                                                    if let Some(ts) = &toast_opt {
                                                        ts.update(cx, |toast, cx| {
                                                            toast.push_error("JSON Format Error", e.to_string(), cx);
                                                        });
                                                    }
                                                }
                                            }
                                        })
                                        .child("Format JSON"),
                                )
                        } else if is_xml {
                            let ed_c = ed_entity.clone();
                            let xml_tx = self.xml_editor.clone();
                            let toast_opt = self.toast_state.clone();
                            div()
                                .flex()
                                .items_center()
                                .child(
                                    div()
                                        .id("btn-format-xml-body")
                                        .px(px(8.))
                                        .py(px(3.))
                                        .rounded(px(4.))
                                        .bg(t.bg_input)
                                        .border_1()
                                        .border_color(t.border)
                                        .cursor_pointer()
                                        .hover(|s| s.bg(t.bg_hover))
                                        .text_xs()
                                        .font_weight(gpui::FontWeight::MEDIUM)
                                        .text_color(t.text_secondary)
                                        .on_click(move |_, window, cx| {
                                            let raw = xml_tx.read(cx).value().to_string();
                                            match format_xml(&raw) {
                                                Ok(pretty) => {
                                                    xml_tx.update(cx, |txt, cx| {
                                                        txt.set_value(pretty.clone(), window, cx);
                                                    });
                                                    ed_c.update(cx, |state, cx| {
                                                        state.set_raw_body(pretty, "application/xml".to_string(), cx);
                                                    });
                                                }
                                                Err(e) => {
                                                    if let Some(ts) = &toast_opt {
                                                        ts.update(cx, |toast, cx| {
                                                            toast.push_error("XML Format Error", e, cx);
                                                        });
                                                    }
                                                }
                                            }
                                        })
                                        .child("Format XML"),
                                )
                        } else {
                            div()
                        }
                    })
            )
            // Body Content Area
            .child(
                div()
                    .flex_1()
                    .w_full()
                    .overflow_hidden()
                    .child({
                        match body_ref {
                            None | Some(RequestBody::None) => {
                                div()
                                    .size_full()
                                    .flex()
                                    .flex_col()
                                    .items_center()
                                    .justify_center()
                                    .gap(px(8.))
                                    .child(
                                        div()
                                            .text_2xl()
                                            .text_color(t.text_muted)
                                            .child("📄"),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .font_weight(gpui::FontWeight::MEDIUM)
                                            .text_color(t.text_secondary)
                                            .child("This request has no body payload"),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(t.text_muted)
                                            .child("Select a format above (e.g. JSON, Form) to add request body data."),
                                    )
                                    .into_any_element()
                            }
                            Some(RequestBody::Json(_)) => {
                                div()
                                    .size_full()
                                    .py(px(6.))
                                    .px(px(4.))
                                    .child(
                                        Editor::new(&self.json_editor)
                                            .appearance(false)
                                            .bordered(false)
                                            .h(gpui::relative(1.))
                                    )
                                    .into_any_element()
                            }
                            Some(RequestBody::FormUrlEncoded(pairs)) => {
                                let pairs = pairs.clone();
                                let ed_up = ed_entity.clone();
                                let ws_up = self.workspace_state.clone();
                                let ed_add = ed_entity.clone();
                                let ed_del = ed_entity.clone();
                                let req_id_str = req_id_str.clone();

                                div()
                                    .size_full()
                                    .overflow_y_scrollbar()
                                    .child(
                                        KvTable::new(
                                            format!("form-kv-{}", req_id_str),
                                            pairs,
                                            move |idx, k, v, enabled, _, cx| {
                                                ed_up.update(cx, |state, cx| {
                                                    state.update_form_param(idx, k, v, enabled, cx);
                                                });
                                                persist_active_request_without_redraw(&ed_up, ws_up.as_ref(), cx);
                                            },
                                            move |_, cx| {
                                                ed_add.update(cx, |state, cx| {
                                                    state.add_form_param(cx);
                                                });
                                            },
                                            move |idx, _, cx| {
                                                ed_del.update(cx, |state, cx| {
                                                    state.remove_form_param(idx, cx);
                                                });
                                            },
                                        )
                                        .optional_workspace_state(self.workspace_state.clone())
                                    )
                                    .into_any_element()
                            }
                            Some(RequestBody::Multipart(items)) => {
                                let items = items.clone();
                                let ed_c = ed_entity.clone();
                                let req_id_str = req_id_str.clone();
                                div()
                                    .size_full()
                                    .overflow_y_scrollbar()
                                    .p(px(8.))
                                    .flex()
                                    .flex_col()
                                    .gap(px(6.))
                                    // Multipart Table Header
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
                                            .child(div().w(px(160.)).child("Key"))
                                            .child(div().w(px(70.)).child("Type"))
                                            .child(div().flex_1().child("Value / File Path"))
                                            .child(div().w(px(80.)).child("Browse"))
                                            .child(div().w(px(28.)).child(""))
                                    )
                                    // Multipart Rows
                                    .children(items.into_iter().enumerate().map(|(idx, item)| {
                                        let ed_row = ed_c.clone();
                                        let ed_del = ed_c.clone();
                                        let ed_pick = ed_c.clone();
                                        let key_str = item.key.clone();
                                        let val_str = item.value.clone();
                                        let is_file = item.is_file;
                                        let file_path = item.file_path.clone();
                                        let enabled = item.enabled;
                                        let mp_prefix = format!("mp-{}-row-{}", req_id_str, idx);

                                        let ed_up_k = ed_row.clone();
                                        let ed_up_v = ed_row.clone();
                                                let ws_up = self.workspace_state.clone();
                                                let ws_up_v = ws_up.clone();
                                        let fp_k = file_path.clone();
                                        let fp_v = file_path.clone();
                                        let mp_holder = window.use_keyed_state(
                                            gpui::SharedString::from(format!("{}-holder", mp_prefix)),
                                            cx,
                                            |window, cx| {
                                                let init_k = key_str.clone();
                                                let init_v = val_str.clone();
                                                let key_input = cx.new(|cx| {
                                                    let mut s = gpui_component::input::InputState::new(window, cx).placeholder("Key...");
                                                    s.set_value(init_k, window, cx);
                                                    s
                                                });
                                                let val_input = cx.new(|cx| {
                                                    let mut s = gpui_component::input::InputState::new(window, cx).placeholder("Value...");
                                                    s.set_value(init_v, window, cx);
                                                    s
                                                });

                                                let k_in_1 = key_input.clone();
                                                let v_in_1 = val_input.clone();
                                                let sub1 = cx.subscribe_in(&key_input, window, move |_, _, event, _, cx| {
                                                    if let gpui_component::input::InputEvent::Change = event {
                                                        let k = k_in_1.read(cx).value().to_string();
                                                        let v = v_in_1.read(cx).value().to_string();
                                                        ed_up_k.update(cx, |state, cx| {
                                                            state.update_multipart_item(idx, k, v, is_file, fp_k.clone(), true, cx);
                                                        });
                                                        persist_active_request_without_redraw(&ed_up_k, ws_up.as_ref(), cx);
                                                    }
                                                });

                                                let k_in_2 = key_input.clone();
                                                let v_in_2 = val_input.clone();
                                                let sub2 = cx.subscribe_in(&val_input, window, move |_, _, event, _, cx| {
                                                    if let gpui_component::input::InputEvent::Change = event {
                                                        let k = k_in_2.read(cx).value().to_string();
                                                        let v = v_in_2.read(cx).value().to_string();
                                                        ed_up_v.update(cx, |state, cx| {
                                                            state.update_multipart_item(idx, k, v, is_file, fp_v.clone(), true, cx);
                                                        });
                                                        persist_active_request_without_redraw(&ed_up_v, ws_up_v.as_ref(), cx);
                                                    }
                                                });

                                                (key_input, val_input, vec![sub1, sub2])
                                            },
                                        );

                                        let (key_input, val_input) = {
                                            let h = mp_holder.read(cx);
                                            (h.0.clone(), h.1.clone())
                                        };

                                        div()
                                            .flex()
                                            .items_center()
                                            .px(px(6.))
                                            .py(px(4.))
                                            .gap(px(8.))
                                            .rounded(px(4.))
                                            .hover(|s| s.bg(t.bg_hover))
                                            // Checkbox
                                            .child({
                                                let ed_toggle = ed_row.clone();
                                                let k = key_str.clone();
                                                let v = val_str.clone();
                                                let fp = file_path.clone();
                                                div()
                                                    .id(format!("check-mp-{}", idx))
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
                                                    .on_click(move |_, _, cx| {
                                                        ed_toggle.update(cx, |state, cx| {
                                                            state.update_multipart_item(idx, k.clone(), v.clone(), is_file, fp.clone(), !enabled, cx);
                                                        });
                                                    })
                                                    .child(if enabled { "✓" } else { "" })
                                            })
                                            // Key
                                            .child(
                                                div()
                                                    .w(px(160.))
                                                    .h(px(28.))
                                                    .px(px(4.))
                                                    .rounded(px(4.))
                                                    .bg(t.bg_input)
                                                    .border_1()
                                                    .border_color(t.border)
                                                    .flex()
                                                    .items_center()
                                                    .child(
                                                        gpui_component::input::Input::new(&key_input)
                                                            .id(gpui::SharedString::from(format!("{}-key", mp_prefix)))
                                                            .appearance(false)
                                                            .bordered(false)
                                                            .focus_bordered(false),
                                                    )
                                            )
                                            // Type toggle: Text | File
                                            .child({
                                                let ed_toggle_type = ed_row.clone();
                                                let k = key_str.clone();
                                                let v = val_str.clone();
                                                let fp = file_path.clone();
                                                div()
                                                    .id(format!("toggle-mp-type-{}", idx))
                                                    .w(px(70.))
                                                    .h(px(28.))
                                                    .rounded(px(4.))
                                                    .bg(t.bg_card)
                                                    .border_1()
                                                    .border_color(t.border)
                                                    .cursor_pointer()
                                                    .flex()
                                                    .items_center()
                                                    .justify_center()
                                                    .text_xs()
                                                    .text_color(t.text_secondary)
                                                    .on_click(move |_, _, cx| {
                                                        ed_toggle_type.update(cx, |state, cx| {
                                                            state.update_multipart_item(idx, k.clone(), v.clone(), !is_file, fp.clone(), enabled, cx);
                                                        });
                                                    })
                                                    .child(div().flex().items_center().gap(px(3.)).child(Icon::new(if is_file { IconName::File } else { IconName::Type }).size(px(12.))).child(if is_file { "File" } else { "Text" }))
                                            })
                                            // Value or Path
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
                                                    .child(if is_file {
                                                        div()
                                                            .px(px(4.))
                                                            .text_xs()
                                                            .font_family(".SystemMonoFont")
                                                            .text_color(t.text_primary)
                                                            .child(file_path.clone().unwrap_or_else(|| "Select a file...".to_string()))
                                                            .into_any_element()
                                                    } else {
                                                        gpui_component::input::Input::new(&val_input)
                                                            .id(gpui::SharedString::from(format!("{}-val", mp_prefix)))
                                                            .appearance(false)
                                                            .bordered(false)
                                                            .focus_bordered(false)
                                                            .into_any_element()
                                                    })
                                            )
                                            // Browse button (only enabled if is_file)
                                            .child({
                                                let ed_p = ed_pick.clone();
                                                let k = key_str.clone();
                                                let v = val_str.clone();
                                                let btn = div()
                                                    .id(format!("browse-mp-{}", idx))
                                                    .w(px(80.))
                                                    .h(px(28.))
                                                    .rounded(px(4.))
                                                    .bg(if is_file { t.bg_card } else { gpui::transparent_black() })
                                                    .border_1()
                                                    .border_color(if is_file { t.border } else { gpui::transparent_black() })
                                                    .flex()
                                                    .items_center()
                                                    .justify_center()
                                                    .text_xs()
                                                    .text_color(if is_file { t.text_secondary } else { t.text_muted });

                                                if is_file {
                                                    btn.cursor_pointer()
                                                        .hover(|s| s.bg(t.bg_hover))
                                                        .on_click(move |_, _, cx| {
                                                            let ed_bg = ed_p.clone();
                                                            let k_bg = k.clone();
                                                            let v_bg = v.clone();
                                                            if let Some(path) = rfd::FileDialog::new().pick_file() {
                                                                let p = path.to_string_lossy().to_string();
                                                                ed_bg.update(cx, |state, cx| {
                                                                    state.update_multipart_item(idx, k_bg, v_bg, true, Some(p), enabled, cx);
                                                                });
                                                            }
                                                        })
                                                        .child("Browse...")
                                                } else {
                                                    btn.child("")
                                                }
                                            })
                                            // Delete row
                                            .child(
                                                div()
                                                    .id(format!("del-mp-row-{}", idx))
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
                                                    .on_click(move |_, _, cx| {
                                                        ed_del.update(cx, |state, cx| {
                                                            state.remove_multipart_item(idx, cx);
                                                        });
                                                    })
                                                    .child("×")
                                            )
                                    }))
                                    // Add Part button
                                    .child(
                                        div()
                                            .pt(px(4.))
                                            .child({
                                                let ed_add = ed_c.clone();
                                                div()
                                                    .id("btn-add-multipart-part")
                                                    .px(px(8.))
                                                    .py(px(4.))
                                                    .rounded(px(4.))
                                                    .w(px(90.))
                                                    .bg(t.bg_card)
                                                    .border_1()
                                                    .border_color(t.border)
                                                    .cursor_pointer()
                                                    .hover(|s| s.bg(t.bg_hover))
                                                    .flex()
                                                    .items_center()
                                                    .justify_center()
                                                    .gap(px(4.))
                                                    .text_xs()
                                                    .text_color(t.text_secondary)
                                                    .on_click(move |_, _, cx| {
                                                        ed_add.update(cx, |state, cx| {
                                                            state.add_multipart_item(cx);
                                                        });
                                                    })
                                                    .child("+ Add Part")
                                            })
                                    )
                                    .into_any_element()
                            }
                            Some(RequestBody::Raw { content_type, .. })
                                if content_type == "application/xml" || content_type == "text/xml" =>
                            {
                                div()
                                    .size_full()
                                    .py(px(6.))
                                    .px(px(4.))
                                    .child(
                                        Editor::new(&self.xml_editor)
                                            .appearance(false)
                                            .bordered(false)
                                            .h(gpui::relative(1.))
                                    )
                                    .into_any_element()
                            }
                            Some(RequestBody::Raw { .. }) => {
                                div()
                                    .size_full()
                                    .p(px(8.))
                                    .child(
                                        Textarea::new(&self.raw_textarea)
                                            .appearance(false)
                                            .bordered(false)
                                            .h(gpui::relative(1.))
                                    )
                                    .into_any_element()
                            }
                            Some(RequestBody::Binary { file_path }) => {
                                let file_path = file_path.clone();
                                let ed_c = ed_entity.clone();
                                let has_file = !file_path.is_empty();
                                let size_str = format_file_size(&file_path);

                                div()
                                    .size_full()
                                    .p(px(20.))
                                    .flex()
                                    .flex_col()
                                    .items_center()
                                    .justify_center()
                                    .child(
                                        div()
                                            .w(px(460.))
                                            .p(px(20.))
                                            .rounded(px(8.))
                                            .bg(t.bg_card)
                                            .border_1()
                                            .border_color(t.border)
                                            .flex()
                                            .flex_col()
                                            .gap(px(14.))
                                            .child(
                                                div()
                                                    .flex()
                                                    .items_center()
                                                    .gap(px(10.))
                                                    .child(div().text_xl().child("📦"))
                                                    .child(
                                                        div()
                                                            .flex()
                                                            .flex_col()
                                                            .child(
                                                                div()
                                                                    .text_sm()
                                                                    .font_weight(gpui::FontWeight::BOLD)
                                                                    .text_color(t.text_primary)
                                                                    .child("Binary Request Payload")
                                                            )
                                                            .child(
                                                                div()
                                                                    .text_xs()
                                                                    .text_color(t.text_secondary)
                                                                    .child("Select a file from your system to send as the raw binary body.")
                                                            )
                                                    )
                                            )
                                            .child(
                                                div()
                                                    .p(px(10.))
                                                    .rounded(px(6.))
                                                    .bg(t.bg_input)
                                                    .border_1()
                                                    .border_color(t.border)
                                                    .flex()
                                                    .items_center()
                                                    .justify_between()
                                                    .child(
                                                        div()
                                                            .flex()
                                                            .flex_col()
                                                            .gap(px(2.))
                                                            .child(
                                                                div()
                                                                    .text_xs()
                                                                    .font_family(".SystemMonoFont")
                                                                    .text_color(if has_file { t.text_primary } else { t.text_muted })
                                                                    .child(if has_file { file_path.clone() } else { "No file chosen yet".to_string() })
                                                            )
                                                            .when(has_file, |this| {
                                                                this.child(
                                                                    div()
                                                                        .text_xs()
                                                                        .text_color(t.text_muted)
                                                                        .child(format!("Size: {}", size_str))
                                                                )
                                                            })
                                                    )
                                                    .child(
                                                        div()
                                                            .flex()
                                                            .items_center()
                                                            .gap(px(6.))
                                                            .child({
                                                                let ed_pick = ed_c.clone();
                                                                div()
                                                                    .id("btn-browse-binary-file")
                                                                    .px(px(10.))
                                                                    .py(px(4.))
                                                                    .rounded(px(4.))
                                                                    .bg(t.primary)
                                                                    .text_color(t.primary_foreground)
                                                                    .hover(|s| s.bg(t.primary_hover))
                                                                    .cursor_pointer()
                                                                    .text_xs()
                                                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                                                    .on_click(move |_, _, cx| {
                                                                        let ed_bg = ed_pick.clone();
                                                                        if let Some(path) = rfd::FileDialog::new().pick_file() {
                                                                            let p = path.to_string_lossy().to_string();
                                                                            ed_bg.update(cx, |state, cx| {
                                                                                state.set_binary_file(p, cx);
                                                                            });
                                                                        }
                                                                    })
                                                                    .child("Browse File...")
                                                            })
                                                            .when(has_file, |this| {
                                                                let ed_clr = ed_c.clone();
                                                                this.child(
                                                                    div()
                                                                        .id("btn-clear-binary-file")
                                                                        .px(px(8.))
                                                                        .py(px(4.))
                                                                        .rounded(px(4.))
                                                                        .bg(t.bg_card)
                                                                        .border_1()
                                                                        .border_color(t.border)
                                                                        .hover(|s| s.bg(t.bg_hover))
                                                                        .cursor_pointer()
                                                                        .text_xs()
                                                                        .text_color(t.text_secondary)
                                                                        .on_click(move |_, _, cx| {
                                                                            ed_clr.update(cx, |state, cx| {
                                                                                state.set_binary_file("".to_string(), cx);
                                                                            });
                                                                        })
                                                                        .child("Clear")
                                                                )
                                                            })
                                                    )
                                            )
                                    )
                                    .into_any_element()
                            }
                        }
                    })
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct DummyResolver;
    impl HighlightStyleResolver for DummyResolver {
        fn style(&self, _: &str) -> Option<HighlightStyle> {
            None
        }
    }

    #[test]
    fn test_json_highlighter_styles_coverage() {
        let mut highlighter = JsonHighlighter::new();
        let json_text = r#"{"name": "Alice", "age": 30, "admin": true, "extra": null}"#;
        highlighter.text = Rope::from_str(json_text);

        let range = 0..json_text.len();
        let styles = highlighter.styles(&range, &DummyResolver);

        // Verify contiguous coverage
        let mut cursor = 0;
        for (sub_range, _) in &styles {
            assert_eq!(sub_range.start, cursor);
            assert!(sub_range.end > sub_range.start);
            cursor = sub_range.end;
        }
        assert_eq!(cursor, json_text.len());

        // Check that keys and values received colors
        let t = theme();
        let key_highlight = styles.iter().find(|(r, s)| {
            &json_text[r.clone()] == "\"name\"" && s.color == Some(t.json_key)
        });
        assert!(key_highlight.is_some(), "Key should have json_key color");

        let str_highlight = styles.iter().find(|(r, s)| {
            &json_text[r.clone()] == "\"Alice\"" && s.color == Some(t.json_string)
        });
        assert!(str_highlight.is_some(), "String should have json_string color");

        let num_highlight = styles.iter().find(|(r, s)| {
            &json_text[r.clone()] == "30" && s.color == Some(t.json_number)
        });
        assert!(num_highlight.is_some(), "Number should have json_number color");

        let bool_highlight = styles.iter().find(|(r, s)| {
            &json_text[r.clone()] == "true" && s.color == Some(t.json_boolean)
        });
        assert!(bool_highlight.is_some(), "Boolean should have json_boolean color");

        let null_highlight = styles.iter().find(|(r, s)| {
            &json_text[r.clone()] == "null" && s.color == Some(t.json_null)
        });
        assert!(null_highlight.is_some(), "Null should have json_null color");
    }

    #[test]
    fn test_json_highlighter_folding() {
        let highlighter = JsonHighlighter::new();
        let json_text = "{\n  \"items\": [\n    1,\n    2\n  ]\n}";
        let rope = Rope::from_str(json_text);
        let folds = highlighter.fold_ranges(&rope);
        assert!(!folds.is_empty(), "Should detect foldable ranges");
    }

    #[test]
    fn test_json_highlighter_large_file_performance() {
        let mut highlighter = JsonHighlighter::new();
        let items: Vec<String> = (0..500)
            .map(|i| format!(r#"{{"id": {}, "title": "large test item {}", "active": true}}"#, i, i))
            .collect();
        let json_text = format!("[\n{}\n]", items.join(",\n"));
        highlighter.text = Rope::from_str(&json_text);
        highlighter.cached_str = json_text.clone();

        // Highlighting individual line range should be instant and not allocate full string
        let first_line_range = 0..1;
        let styles = highlighter.styles(&first_line_range, &DummyResolver);
        assert!(!styles.is_empty());
    }

    #[test]
    fn test_format_xml_valid() {
        let raw = "<root><item id=\"1\"><name>Alice</name></item></root>";
        let formatted = format_xml(raw).expect("should format valid XML");
        assert!(formatted.contains("<root>\n"));
        assert!(formatted.contains("  <item id=\"1\">\n"));
        assert!(formatted.contains("    <name>Alice</name>\n"));
        assert!(formatted.contains("  </item>\n"));
        assert!(formatted.contains("</root>"));
    }

    #[test]
    fn test_format_xml_with_declaration() {
        let raw = "<?xml version=\"1.0\" encoding=\"UTF-8\"?><root><val>123</val></root>";
        let formatted = format_xml(raw).expect("should format XML with declaration");
        assert!(formatted.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
        assert!(formatted.contains("<root>\n"));
        assert!(formatted.contains("  <val>123</val>\n"));
    }

    #[test]
    fn test_format_xml_invalid_syntax() {
        let raw = "<root><unclosed>test</root>";
        let res = format_xml(raw);
        assert!(res.is_err(), "Invalid unclosed tag should return error");
    }

    #[test]
    fn test_xml_highlighter_folding() {
        let highlighter = XmlHighlighter::new();
        let xml_text = "<?xml version=\"1.0\"?>\n<root>\n  <item id=\"1\">\n    <name>Widget</name>\n  </item>\n  <item id=\"2\" />\n  <!--\n    Multi line\n    comment\n  -->\n</root>";
        let rope = Rope::from_str(xml_text);
        let folds = highlighter.fold_ranges(&rope);
        assert!(!folds.is_empty(), "Should detect foldable ranges in XML");

        // <root> starts at line 1, ends at line 10
        assert!(folds.iter().any(|f| f.start_line == 1 && f.end_line == 10));
        // <item id="1"> starts at line 2, ends at line 4
        assert!(folds.iter().any(|f| f.start_line == 2 && f.end_line == 4));
        // Multi-line comment starts at line 6, ends at line 9
        assert!(folds.iter().any(|f| f.start_line == 6 && f.end_line == 9));
        // Single line <name>Widget</name> on line 3 should NOT be folded
        assert!(!folds.iter().any(|f| f.start_line == 3));
        // Self-closing <item id="2" /> on line 5 should NOT be folded
        assert!(!folds.iter().any(|f| f.start_line == 5));
    }

    #[test]
    fn test_xml_highlighter_styles() {
        let mut highlighter = XmlHighlighter::new();
        let xml_text = "<root id=\"101\">{{custom_var}}</root>";
        highlighter.text = Rope::from_str(xml_text);
        highlighter.cached_str = xml_text.to_string();

        let range = 0..xml_text.len();
        let styles = highlighter.styles(&range, &DummyResolver);
        assert!(!styles.is_empty(), "Styles should tokenize XML tags and variables");

        let t = theme();
        let has_var_highlight = styles.iter().any(|(r, s)| {
            &xml_text[r.clone()] == "{{custom_var}}" && s.color == Some(t.var_resolved)
        });
        assert!(has_var_highlight, "Should highlight {{custom_var}} with var_resolved color");
    }
}

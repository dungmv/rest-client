use gpui::{Hsla, Rgba};
use restclient_core::models::HttpMethod;

#[derive(Debug, Clone, Copy)]
pub struct AppTheme {
    // Backgrounds
    pub bg_app: Hsla,
    pub bg_titlebar: Hsla,
    pub bg_sidebar: Hsla,
    pub bg_editor: Hsla,
    pub bg_response: Hsla,
    pub bg_panel: Hsla,
    pub bg_card: Hsla,
    pub bg_input: Hsla,
    pub bg_selected: Hsla,
    pub bg_hover: Hsla,
    pub bg_active: Hsla,

    // Borders
    pub border: Hsla,
    pub border_subtle: Hsla,
    pub border_focus: Hsla,

    // Text & Foregrounds
    pub text_primary: Hsla,
    pub text_secondary: Hsla,
    pub text_muted: Hsla,
    pub text_disabled: Hsla,

    // Primary action (Vibrant Blue #2563eb matching httpeep)
    pub primary: Hsla,
    pub primary_hover: Hsla,
    pub primary_foreground: Hsla,

    // Method badge colors (httpeep style: GET is green #10b981)
    pub method_get: Hsla,
    pub method_post: Hsla,
    pub method_put: Hsla,
    pub method_delete: Hsla,
    pub method_patch: Hsla,
    pub method_head: Hsla,
    pub method_options: Hsla,

    // Status colors
    pub status_2xx: Hsla,
    pub status_3xx: Hsla,
    pub status_4xx: Hsla,
    pub status_5xx: Hsla,

    // Variable highlights
    pub var_resolved: Hsla,
    pub var_unresolved: Hsla,

    // Transparent
    pub transparent: Hsla,
}

pub fn hex_to_hsla(hex: u32) -> Hsla {
    let r = ((hex >> 16) & 0xFF) as f32 / 255.0;
    let g = ((hex >> 8) & 0xFF) as f32 / 255.0;
    let b = (hex & 0xFF) as f32 / 255.0;
    Hsla::from(Rgba { r, g, b, a: 1.0 })
}

pub fn hex_alpha_to_hsla(hex: u32, alpha: f32) -> Hsla {
    let r = ((hex >> 16) & 0xFF) as f32 / 255.0;
    let g = ((hex >> 8) & 0xFF) as f32 / 255.0;
    let b = (hex & 0xFF) as f32 / 255.0;
    Hsla::from(Rgba { r, g, b, a: alpha })
}

impl Default for AppTheme {
    fn default() -> Self {
        Self {
            // Modern Charcoal / Obsidian Dark Theme matching httpeep: ~#0c0d12
            bg_app: hex_to_hsla(0x0c0d12),
            bg_titlebar: hex_to_hsla(0x0e0f14),
            bg_sidebar: hex_to_hsla(0x101117),
            bg_editor: hex_to_hsla(0x121319),
            bg_response: hex_to_hsla(0x121319),
            bg_panel: hex_to_hsla(0x15161e),
            bg_card: hex_to_hsla(0x181922),
            bg_input: hex_to_hsla(0x13141b),
            bg_selected: hex_to_hsla(0x20222e),
            bg_hover: hex_to_hsla(0x1a1b24),
            bg_active: hex_to_hsla(0x242636),

            // Fine, subtle 1px borders
            border: hex_to_hsla(0x20222d),
            border_subtle: hex_to_hsla(0x181922),
            border_focus: hex_to_hsla(0x2563eb),

            // Typography: crisp, high readability
            text_primary: hex_to_hsla(0xf3f4f6),
            text_secondary: hex_to_hsla(0x9ca3af),
            text_muted: hex_to_hsla(0x525866),
            text_disabled: hex_to_hsla(0x383c48),

            // Professional vibrant blue
            primary: hex_to_hsla(0x2563eb),
            primary_hover: hex_to_hsla(0x1d4ed8),
            primary_foreground: hex_to_hsla(0xffffff),

            // Method colors (httpeep style: GET is clean green, POST is blue, etc.)
            method_get: hex_to_hsla(0x10b981),
            method_post: hex_to_hsla(0x3b82f6),
            method_put: hex_to_hsla(0xf59e0b),
            method_delete: hex_to_hsla(0xef4444),
            method_patch: hex_to_hsla(0x8b5cf6),
            method_head: hex_to_hsla(0x06b6d4),
            method_options: hex_to_hsla(0x64748b),

            // Status colors
            status_2xx: hex_to_hsla(0x10b981),
            status_3xx: hex_to_hsla(0x3b82f6),
            status_4xx: hex_to_hsla(0xf59e0b),
            status_5xx: hex_to_hsla(0xef4444),

            // Interpolation variable highlight
            var_resolved: hex_to_hsla(0x38bdf8),
            var_unresolved: hex_to_hsla(0xef4444),

            transparent: gpui::transparent_black(),
        }
    }
}

pub static THEME: std::sync::LazyLock<AppTheme> = std::sync::LazyLock::new(AppTheme::default);

pub fn theme() -> &'static AppTheme {
    &THEME
}

impl AppTheme {
    pub fn method_color(&self, method: HttpMethod) -> Hsla {
        match method {
            HttpMethod::Get => self.method_get,
            HttpMethod::Post => self.method_post,
            HttpMethod::Put => self.method_put,
            HttpMethod::Delete => self.method_delete,
            HttpMethod::Patch => self.method_patch,
            HttpMethod::Head => self.method_head,
            HttpMethod::Options => self.method_options,
        }
    }

    pub fn status_color(&self, status_code: u16) -> Hsla {
        match status_code {
            200..=299 => self.status_2xx,
            300..=399 => self.status_3xx,
            400..=499 => self.status_4xx,
            500..=599 => self.status_5xx,
            _ => self.text_muted,
        }
    }
}

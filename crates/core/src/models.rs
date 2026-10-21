use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "UPPERCASE")]
pub enum HttpMethod {
    #[default]
    Get,
    Post,
    Put,
    Delete,
    Patch,
    Head,
    Options,
}

impl HttpMethod {
    pub fn as_str(&self) -> &'static str {
        match self {
            HttpMethod::Get => "GET",
            HttpMethod::Post => "POST",
            HttpMethod::Put => "PUT",
            HttpMethod::Delete => "DELETE",
            HttpMethod::Patch => "PATCH",
            HttpMethod::Head => "HEAD",
            HttpMethod::Options => "OPTIONS",
        }
    }
}

impl std::fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for HttpMethod {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "GET" => Ok(HttpMethod::Get),
            "POST" => Ok(HttpMethod::Post),
            "PUT" => Ok(HttpMethod::Put),
            "DELETE" => Ok(HttpMethod::Delete),
            "PATCH" => Ok(HttpMethod::Patch),
            "HEAD" => Ok(HttpMethod::Head),
            "OPTIONS" => Ok(HttpMethod::Options),
            _ => Err(format!("Unknown HTTP method: {}", s)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct KeyValuePair {
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub key: String,
    pub value: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

fn default_true() -> bool {
    true
}

impl KeyValuePair {
    pub fn new(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            enabled: true,
            key: key.into(),
            value: value.into(),
            description: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Auth {
    #[default]
    Inherit,
    None,
    Bearer {
        token: String,
    },
    Basic {
        username: String,
        password: String,
    },
    ApiKey {
        key: String,
        value: String,
        #[serde(default = "default_true")]
        in_header: bool,
    },
}

impl Auth {
    pub fn is_inherit(&self) -> bool {
        matches!(self, Auth::Inherit)
    }

    pub fn is_none(&self) -> bool {
        matches!(self, Auth::None)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MultipartItem {
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub key: String,
    pub value: String,
    #[serde(default)]
    pub is_file: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_path: Option<String>,
}

impl Default for MultipartItem {
    fn default() -> Self {
        Self {
            enabled: true,
            key: String::new(),
            value: String::new(),
            is_file: false,
            file_path: None,
        }
    }
}

impl MultipartItem {
    pub fn new_text(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            enabled: true,
            key: key.into(),
            value: value.into(),
            is_file: false,
            file_path: None,
        }
    }

    pub fn new_file(key: impl Into<String>, file_path: impl Into<String>) -> Self {
        let path = file_path.into();
        Self {
            enabled: true,
            key: key.into(),
            value: path.clone(),
            is_file: true,
            file_path: Some(path),
        }
    }
}

impl From<KeyValuePair> for MultipartItem {
    fn from(kv: KeyValuePair) -> Self {
        Self {
            enabled: kv.enabled,
            key: kv.key,
            value: kv.value,
            is_file: false,
            file_path: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RequestBody {
    #[default]
    None,
    Json(String),
    FormUrlEncoded(Vec<KeyValuePair>),
    Multipart(Vec<MultipartItem>),
    Raw {
        content: String,
        content_type: String,
    },
    Binary {
        file_path: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestSettings {
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
    #[serde(default = "default_true")]
    pub follow_redirects: bool,
    #[serde(default = "default_true")]
    pub validate_tls: bool,
}

fn default_timeout() -> u64 {
    30
}

impl Default for RequestSettings {
    fn default() -> Self {
        Self {
            timeout_secs: default_timeout(),
            follow_redirects: true,
            validate_tls: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScriptConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pre_request: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub post_response: Option<String>,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

impl Default for ScriptConfig {
    fn default() -> Self {
        Self {
            pre_request: None,
            post_response: None,
            enabled: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestResult {
    pub name: String,
    pub passed: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HttpRequestItem {
    pub id: String,
    pub workspace_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub folder_id: Option<String>,
    pub name: String,
    pub method: HttpMethod,
    pub url: String,
    #[serde(default)]
    pub params: Vec<KeyValuePair>,
    #[serde(default)]
    pub headers: Vec<KeyValuePair>,
    #[serde(default)]
    pub auth: Auth,
    #[serde(default)]
    pub body: RequestBody,
    #[serde(default)]
    pub settings: RequestSettings,
    #[serde(default)]
    pub sort_order: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_status_code: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub script: Option<ScriptConfig>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub examples: Vec<crate::storage::models::RequestExampleFile>,
}

impl HttpRequestItem {
    pub fn new(
        workspace_id: impl Into<String>,
        name: impl Into<String>,
        method: HttpMethod,
        url: impl Into<String>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            workspace_id: workspace_id.into(),
            folder_id: None,
            name: name.into(),
            method,
            url: url.into(),
            params: Vec::new(),
            headers: Vec::new(),
            auth: Auth::Inherit,
            body: RequestBody::None,
            settings: RequestSettings::default(),
            sort_order: 0,
            last_status_code: None,
            script: None,
            examples: Vec::new(),
        }
    }
}


#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Folder {
    pub id: String,
    pub workspace_id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
    #[serde(default)]
    pub sort_order: i32,
    #[serde(default)]
    pub headers: Vec<KeyValuePair>,
    #[serde(default)]
    pub auth: Auth,
}

impl Folder {
    pub fn new(workspace_id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            workspace_id: workspace_id.into(),
            name: name.into(),
            parent_id: None,
            sort_order: 0,
            headers: Vec::new(),
            auth: Auth::Inherit,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct EnvironmentVariable {
    pub key: String,
    #[serde(default)]
    pub initial_value: String,
    #[serde(default)]
    pub current_value: String,
    #[serde(default)]
    pub secret: bool,
}

impl EnvironmentVariable {
    pub fn resolved_value(&self) -> &str {
        if !self.current_value.is_empty() {
            &self.current_value
        } else {
            &self.initial_value
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Environment {
    pub id: String,
    pub workspace_id: String,
    pub name: String,
    #[serde(default)]
    pub variables: Vec<EnvironmentVariable>,
}

impl Environment {
    pub fn new(workspace_id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            workspace_id: workspace_id.into(),
            name: name.into(),
            variables: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Workspace {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_environment_id: Option<String>,
    #[serde(default)]
    pub headers: Vec<KeyValuePair>,
    #[serde(default)]
    pub auth: Auth,
}

impl Workspace {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            description: String::new(),
            active_environment_id: None,
            headers: Vec::new(),
            auth: Auth::None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TimingBreakdown {
    pub dns_ms: Option<u64>,
    pub connect_ms: Option<u64>,
    pub tls_ms: Option<u64>,
    pub ttfb_ms: Option<u64>,
    pub download_ms: Option<u64>,
    pub total_ms: u64,
    #[serde(default)]
    pub connection_reused: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct RequestSentInfo {
    pub method: HttpMethod,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body_size_bytes: usize,
    pub body_preview: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParsedCookie {
    pub name: String,
    pub value: String,
    pub domain: Option<String>,
    pub path: Option<String>,
    pub expires: Option<String>,
    pub max_age: Option<i64>,
    pub secure: bool,
    pub http_only: bool,
    pub same_site: Option<String>,
}

impl ParsedCookie {
    pub fn parse(raw: &str) -> Self {
        let mut parts = raw.split(';').map(|s| s.trim());
        let first = parts.next().unwrap_or("");
        let (name, value) = if let Some((n, v)) = first.split_once('=') {
            (n.trim().to_string(), v.trim().to_string())
        } else {
            (first.to_string(), String::new())
        };

        let mut domain = None;
        let mut path = None;
        let mut expires = None;
        let mut max_age = None;
        let mut secure = false;
        let mut http_only = false;
        let mut same_site = None;

        for part in parts {
            if part.eq_ignore_ascii_case("secure") {
                secure = true;
            } else if part.eq_ignore_ascii_case("httponly") {
                http_only = true;
            } else if let Some((k, v)) = part.split_once('=') {
                let k_trim = k.trim();
                let v_trim = v.trim();
                if k_trim.eq_ignore_ascii_case("domain") {
                    domain = Some(v_trim.to_string());
                } else if k_trim.eq_ignore_ascii_case("path") {
                    path = Some(v_trim.to_string());
                } else if k_trim.eq_ignore_ascii_case("expires") {
                    expires = Some(v_trim.to_string());
                } else if k_trim.eq_ignore_ascii_case("max-age") {
                    max_age = v_trim.parse::<i64>().ok();
                } else if k_trim.eq_ignore_ascii_case("samesite") {
                    same_site = Some(v_trim.to_string());
                }
            }
        }

        Self {
            name,
            value,
            domain,
            path,
            expires,
            max_age,
            secure,
            http_only,
            same_site,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConsoleLogSource {
    PreRequest,
    PostResponse,
    System,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConsoleLogLevel {
    Log,
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsoleLogEntry {
    pub source: ConsoleLogSource,
    pub level: ConsoleLogLevel,
    pub message: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HttpResponseRecord {
    pub id: String,
    pub request_id: String,
    pub status_code: u16,
    pub status_text: String,
    pub duration_ms: u64,
    pub size_bytes: usize,
    pub headers: Vec<(String, String)>,
    pub cookies: Vec<String>,
    pub body: Vec<u8>,
    pub timing: TimingBreakdown,
    #[serde(default)]
    pub request_sent: Option<RequestSentInfo>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub test_results: Vec<TestResult>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub console_logs: Vec<ConsoleLogEntry>,
    pub created_at: DateTime<Utc>,
}

pub fn status_code_to_reason(code: u16) -> &'static str {
    reqwest::StatusCode::from_u16(code)
        .ok()
        .and_then(|s| s.canonical_reason())
        .unwrap_or("")
}

impl HttpResponseRecord {
    pub fn body_as_str(&self) -> Result<&str, std::str::Utf8Error> {
        std::str::from_utf8(&self.body)
    }

    pub fn formatted_size(&self) -> String {
        let bytes = self.size_bytes;
        if bytes < 1024 {
            format!("{} B", bytes)
        } else if bytes < 1024 * 1024 {
            format!("{:.1} KB", bytes as f64 / 1024.0)
        } else {
            format!("{:.2} MB", bytes as f64 / (1024.0 * 1024.0))
        }
    }

    pub fn formatted_duration(&self) -> String {
        if self.duration_ms < 1000 {
            format!("{} ms", self.duration_ms)
        } else {
            format!("{:.2} s", self.duration_ms as f64 / 1000.0)
        }
    }

    pub fn parsed_cookies(&self) -> Vec<ParsedCookie> {
        self.cookies.iter().map(|c| ParsedCookie::parse(c)).collect()
    }

    pub fn is_json(&self) -> bool {
        self.headers
            .iter()
            .any(|(k, v)| k.eq_ignore_ascii_case("content-type") && v.to_lowercase().contains("json"))
    }

    pub fn is_xml(&self) -> bool {
        self.headers
            .iter()
            .any(|(k, v)| {
                k.eq_ignore_ascii_case("content-type")
                    && (v.to_lowercase().contains("xml") || v.to_lowercase().contains("application/xml") || v.to_lowercase().contains("text/xml"))
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_code_to_reason() {
        assert_eq!(status_code_to_reason(200), "OK");
        assert_eq!(status_code_to_reason(201), "Created");
        assert_eq!(status_code_to_reason(404), "Not Found");
        assert_eq!(status_code_to_reason(500), "Internal Server Error");
        assert_eq!(status_code_to_reason(999), "");
    }
}

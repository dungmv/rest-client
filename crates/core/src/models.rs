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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RequestBody {
    #[default]
    None,
    Json(String),
    FormUrlEncoded(Vec<KeyValuePair>),
    Multipart(Vec<KeyValuePair>),
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
            auth: Auth::None,
            body: RequestBody::None,
            settings: RequestSettings::default(),
            sort_order: 0,
            last_status_code: None,
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
}

impl Folder {
    pub fn new(workspace_id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            workspace_id: workspace_id.into(),
            name: name.into(),
            parent_id: None,
            sort_order: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct EnvironmentVariable {
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub key: String,
    pub value: String,
    #[serde(default)]
    pub secret: bool,
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
}

impl Workspace {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            description: String::new(),
            active_environment_id: None,
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
    pub created_at: DateTime<Utc>,
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
}

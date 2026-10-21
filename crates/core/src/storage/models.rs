use crate::models::{
    Auth, Environment, EnvironmentVariable, HttpMethod, HttpRequestItem, HttpResponseRecord,
    KeyValuePair, MultipartItem, RequestBody, RequestSentInfo, RequestSettings, ScriptConfig,
    TimingBreakdown,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

fn default_true() -> bool {
    true
}

fn default_timeout_ms() -> u64 {
    30000
}

fn default_version() -> u32 {
    1
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceMeta {
    #[serde(default = "default_version")]
    pub version: u32,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_environment: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub headers: Vec<RequestKvItem>,
    #[serde(default, skip_serializing_if = "RequestAuth::is_none")]
    pub auth: RequestAuth,
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_yaml::Value>,
}

impl Default for WorkspaceMeta {
    fn default() -> Self {
        Self {
            version: 1,
            name: "My Workspace".to_string(),
            default_environment: Some("local".to_string()),
            headers: Vec::new(),
            auth: RequestAuth::None,
            extra: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct FolderFile {
    pub name: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub order: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub headers: Vec<RequestKvItem>,
    #[serde(default, skip_serializing_if = "RequestAuth::is_inherit")]
    pub auth: RequestAuth,
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_yaml::Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestKvItem {
    #[serde(alias = "key")]
    pub name: String,
    #[serde(default)]
    pub value: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_yaml::Value>,
}

impl RequestKvItem {
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
            enabled: true,
            description: None,
            extra: BTreeMap::new(),
        }
    }
}

impl From<KeyValuePair> for RequestKvItem {
    fn from(kv: KeyValuePair) -> Self {
        Self {
            name: kv.key,
            value: kv.value,
            enabled: kv.enabled,
            description: kv.description,
            extra: BTreeMap::new(),
        }
    }
}

impl From<RequestKvItem> for KeyValuePair {
    fn from(item: RequestKvItem) -> Self {
        Self {
            enabled: item.enabled,
            key: item.name,
            value: item.value,
            description: item.description,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum RequestAuth {
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

impl RequestAuth {
    pub fn is_inherit(&self) -> bool {
        matches!(self, RequestAuth::Inherit)
    }

    pub fn is_none(&self) -> bool {
        matches!(self, RequestAuth::None)
    }
}

impl From<Auth> for RequestAuth {
    fn from(auth: Auth) -> Self {
        match auth {
            Auth::Inherit => RequestAuth::Inherit,
            Auth::None => RequestAuth::None,
            Auth::Bearer { token } => RequestAuth::Bearer { token },
            Auth::Basic { username, password } => RequestAuth::Basic { username, password },
            Auth::ApiKey { key, value, in_header } => RequestAuth::ApiKey { key, value, in_header },
        }
    }
}

impl From<RequestAuth> for Auth {
    fn from(auth: RequestAuth) -> Self {
        match auth {
            RequestAuth::Inherit => Auth::Inherit,
            RequestAuth::None => Auth::None,
            RequestAuth::Bearer { token } => Auth::Bearer { token },
            RequestAuth::Basic { username, password } => Auth::Basic { username, password },
            RequestAuth::ApiKey { key, value, in_header } => Auth::ApiKey { key, value, in_header },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MultipartItemFile {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(alias = "key")]
    pub name: String,
    pub value: String,
    #[serde(default)]
    pub is_file: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_path: Option<String>,
}

impl From<MultipartItem> for MultipartItemFile {
    fn from(m: MultipartItem) -> Self {
        Self {
            enabled: m.enabled,
            name: m.key,
            value: m.value,
            is_file: m.is_file,
            file_path: m.file_path,
        }
    }
}

impl From<MultipartItemFile> for MultipartItem {
    fn from(m: MultipartItemFile) -> Self {
        Self {
            enabled: m.enabled,
            key: m.name,
            value: m.value,
            is_file: m.is_file,
            file_path: m.file_path,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum RequestBodyFile {
    #[default]
    None,
    Json {
        #[serde(default)]
        content: String,
    },
    Form {
        #[serde(default)]
        params: Vec<RequestKvItem>,
    },
    Multipart {
        #[serde(default)]
        items: Vec<MultipartItemFile>,
    },
    Raw {
        #[serde(default)]
        content: String,
        #[serde(default = "default_raw_type")]
        content_type: String,
    },
    Binary {
        #[serde(default)]
        file_path: String,
    },
}

fn default_raw_type() -> String {
    "text/plain".to_string()
}

impl From<RequestBody> for RequestBodyFile {
    fn from(body: RequestBody) -> Self {
        match body {
            RequestBody::None => RequestBodyFile::None,
            RequestBody::Json(content) => RequestBodyFile::Json { content },
            RequestBody::FormUrlEncoded(params) => RequestBodyFile::Form {
                params: params.into_iter().map(Into::into).collect(),
            },
            RequestBody::Multipart(items) => RequestBodyFile::Multipart {
                items: items.into_iter().map(Into::into).collect(),
            },
            RequestBody::Raw { content, content_type } => RequestBodyFile::Raw { content, content_type },
            RequestBody::Binary { file_path } => RequestBodyFile::Binary { file_path },
        }
    }
}

impl From<RequestBodyFile> for RequestBody {
    fn from(body: RequestBodyFile) -> Self {
        match body {
            RequestBodyFile::None => RequestBody::None,
            RequestBodyFile::Json { content } => RequestBody::Json(content),
            RequestBodyFile::Form { params } => RequestBody::FormUrlEncoded(
                params.into_iter().map(Into::into).collect(),
            ),
            RequestBodyFile::Multipart { items } => RequestBody::Multipart(
                items.into_iter().map(Into::into).collect(),
            ),
            RequestBodyFile::Raw { content, content_type } => RequestBody::Raw { content, content_type },
            RequestBodyFile::Binary { file_path } => RequestBody::Binary { file_path },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestSettingsFile {
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
    #[serde(default = "default_true")]
    pub follow_redirects: bool,
    #[serde(default = "default_true")]
    pub validate_tls: bool,
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_yaml::Value>,
}

impl Default for RequestSettingsFile {
    fn default() -> Self {
        Self {
            timeout_ms: default_timeout_ms(),
            follow_redirects: true,
            validate_tls: true,
            extra: BTreeMap::new(),
        }
    }
}

impl From<RequestSettings> for RequestSettingsFile {
    fn from(s: RequestSettings) -> Self {
        Self {
            timeout_ms: s.timeout_secs * 1000,
            follow_redirects: s.follow_redirects,
            validate_tls: s.validate_tls,
            extra: BTreeMap::new(),
        }
    }
}

impl From<RequestSettingsFile> for RequestSettings {
    fn from(s: RequestSettingsFile) -> Self {
        Self {
            timeout_secs: s.timeout_ms / 1000,
            follow_redirects: s.follow_redirects,
            validate_tls: s.validate_tls,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct RequestScriptFile {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pre_request: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub post_response: Option<String>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_yaml::Value>,
}

impl From<ScriptConfig> for RequestScriptFile {
    fn from(s: ScriptConfig) -> Self {
        Self {
            pre_request: s.pre_request,
            post_response: s.post_response,
            extra: BTreeMap::new(),
        }
    }
}

impl From<RequestScriptFile> for ScriptConfig {
    fn from(s: RequestScriptFile) -> Self {
        Self {
            pre_request: s.pre_request,
            post_response: s.post_response,
            enabled: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestExampleFile {
    pub name: String,
    pub status: u16,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub headers: Vec<RequestKvItem>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_yaml::Value>,
}

/// Request YAML model:
/// Fixed key order:
/// 1. name
/// 2. method
/// 3. url
/// 4. params
/// 5. headers
/// 6. auth
/// 7. body
/// 8. settings
/// 9. script
/// 10. extra (unknown keys preserved)
/// 11. examples (ALWAYS last)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestFile {
    pub name: String,
    pub method: HttpMethod,
    pub url: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub params: Vec<RequestKvItem>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub headers: Vec<RequestKvItem>,
    #[serde(default)]
    pub auth: RequestAuth,
    #[serde(default)]
    pub body: RequestBodyFile,
    #[serde(default)]
    pub settings: RequestSettingsFile,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub script: Option<RequestScriptFile>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_yaml::Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub examples: Vec<RequestExampleFile>,
}

impl Default for RequestFile {
    fn default() -> Self {
        Self {
            name: "New Request".to_string(),
            method: HttpMethod::Get,
            url: String::new(),
            params: Vec::new(),
            headers: Vec::new(),
            auth: RequestAuth::None,
            body: RequestBodyFile::None,
            settings: RequestSettingsFile::default(),
            script: None,
            extra: BTreeMap::new(),
            examples: Vec::new(),
        }
    }
}

impl RequestFile {
    pub fn from_item(item: &HttpRequestItem) -> Self {
        let mut file = Self::default();
        file.update_from_item(item);
        file
    }

    pub fn to_item(&self, id: impl Into<String>, workspace_id: impl Into<String>, folder_id: Option<String>) -> HttpRequestItem {
        HttpRequestItem {
            id: id.into(),
            workspace_id: workspace_id.into(),
            folder_id,
            name: self.name.clone(),
            method: self.method,
            url: self.url.clone(),
            params: self.params.iter().cloned().map(Into::into).collect(),
            headers: self.headers.iter().cloned().map(Into::into).collect(),
            auth: self.auth.clone().into(),
            body: self.body.clone().into(),
            settings: self.settings.clone().into(),
            sort_order: 0,
            last_status_code: None,
            script: self.script.clone().map(Into::into),
            examples: self.examples.clone(),
        }
    }

    pub fn update_from_item(&mut self, item: &HttpRequestItem) {
        self.name = item.name.clone();
        self.method = item.method;
        self.url = item.url.clone();
        self.params = item.params.iter().cloned().map(Into::into).collect();
        self.headers = item.headers.iter().cloned().map(Into::into).collect();
        self.auth = item.auth.clone().into();
        self.body = item.body.clone().into();
        self.settings = item.settings.clone().into();
        self.script = item.script.clone().map(Into::into);
        self.examples = item.examples.clone();
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnvVarFile {
    #[serde(alias = "key")]
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(default)]
    pub secret: bool,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_yaml::Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct EnvironmentFile {
    #[serde(default)]
    pub variables: Vec<EnvVarFile>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_yaml::Value>,
}

impl EnvironmentFile {
    pub fn to_environment(&self, id: impl Into<String>, workspace_id: impl Into<String>, name: impl Into<String>, secrets: &std::collections::HashMap<String, String>) -> Environment {
        let mut env = Environment::new(workspace_id, name);
        env.id = id.into();
        for v in &self.variables {
            let (init_val, cur_val) = if v.secret {
                let init = secrets.get(&format!("{}:init", v.name))
                    .cloned()
                    .or_else(|| secrets.get(&v.name).cloned())
                    .unwrap_or_default();
                let cur = secrets.get(&v.name).cloned().unwrap_or_default();
                (init, cur)
            } else {
                let init = v.value.clone().unwrap_or_default();
                let cur = secrets.get(&v.name).cloned().unwrap_or_default();
                (init, cur)
            };
            env.variables.push(EnvironmentVariable {
                key: v.name.clone(),
                initial_value: init_val,
                current_value: cur_val,
                secret: v.secret,
            });
        }
        env
    }

    pub fn from_environment(env: &Environment) -> (Self, std::collections::HashMap<String, String>) {
        let mut env_file = EnvironmentFile::default();
        let mut secrets = std::collections::HashMap::new();

        for v in &env.variables {
            if v.secret {
                env_file.variables.push(EnvVarFile {
                    name: v.key.clone(),
                    value: None,
                    secret: true,
                    enabled: true,
                    extra: BTreeMap::new(),
                });
                if !v.current_value.is_empty() {
                    secrets.insert(v.key.clone(), v.current_value.clone());
                    if !v.initial_value.is_empty() && v.initial_value != v.current_value {
                        secrets.insert(format!("{}:init", v.key), v.initial_value.clone());
                    }
                } else if !v.initial_value.is_empty() {
                    secrets.insert(v.key.clone(), v.initial_value.clone());
                }
            } else {
                env_file.variables.push(EnvVarFile {
                    name: v.key.clone(),
                    value: Some(v.initial_value.clone()),
                    secret: false,
                    enabled: true,
                    extra: BTreeMap::new(),
                });
                if !v.current_value.is_empty() {
                    secrets.insert(v.key.clone(), v.current_value.clone());
                }
            }
        }
        (env_file, secrets)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocRequestInfo {
    pub method: HttpMethod,
    pub url: String,
    #[serde(default)]
    pub headers: Vec<(String, String)>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocResponseInfo {
    pub status: u16,
    pub duration_ms: u64,
    pub size: usize,
    #[serde(default)]
    pub headers: Vec<(String, String)>,
    pub body: Option<String>,
    pub truncated: bool,
    pub binary: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocFile {
    pub sent_at: String,
    pub request: LocRequestInfo,
    pub response: LocResponseInfo,
}

impl LocFile {
    pub fn from_record(record: &HttpResponseRecord) -> Self {
        let sent_req = record.request_sent.as_ref();
        let (method, url, req_headers, req_body) = if let Some(sent) = sent_req {
            (
                sent.method,
                sent.url.clone(),
                sent.headers.clone(),
                sent.body_preview.clone(),
            )
        } else {
            (
                HttpMethod::Get,
                String::new(),
                Vec::new(),
                None,
            )
        };

        let is_binary = record.body.iter().take(1024).any(|&b| b == 0);
        let max_body_bytes = 512 * 1024;
        let (body_str, truncated) = if is_binary {
            (None, false)
        } else if record.body.len() > max_body_bytes {
            (
                Some(String::from_utf8_lossy(&record.body[..max_body_bytes]).to_string()),
                true,
            )
        } else {
            (
                Some(String::from_utf8_lossy(&record.body).to_string()),
                false,
            )
        };

        Self {
            sent_at: record.created_at.to_rfc3339(),
            request: LocRequestInfo {
                method,
                url,
                headers: req_headers,
                body: req_body,
            },
            response: LocResponseInfo {
                status: record.status_code,
                duration_ms: record.duration_ms,
                size: record.size_bytes,
                headers: record.headers.clone(),
                body: body_str,
                truncated,
                binary: is_binary,
            },
        }
    }

    pub fn to_record(&self, request_id: &str) -> HttpResponseRecord {
        let body_bytes = self.response.body.as_deref().unwrap_or("").as_bytes().to_vec();
        let created_at = chrono::DateTime::parse_from_rfc3339(&self.sent_at)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now());

        let reason = crate::models::status_code_to_reason(self.response.status);
        let status_text = if reason.is_empty() {
            format!("HTTP {}", self.response.status)
        } else {
            reason.to_string()
        };

        HttpResponseRecord {
            id: format!("loc-{}", request_id),
            request_id: request_id.to_string(),
            status_code: self.response.status,
            status_text,
            duration_ms: self.response.duration_ms,
            size_bytes: self.response.size,
            headers: self.response.headers.clone(),
            cookies: Vec::new(),
            body: body_bytes,
            timing: TimingBreakdown {
                total_ms: self.response.duration_ms,
                ..Default::default()
            },
            request_sent: Some(RequestSentInfo {
                method: self.request.method,
                url: self.request.url.clone(),
                headers: self.request.headers.clone(),
                body_size_bytes: self.request.body.as_ref().map(|b| b.len()).unwrap_or(0),
                body_preview: self.request.body.clone(),
            }),
            test_results: Vec::new(),
            console_logs: Vec::new(),
            created_at,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PinsFile {
    #[serde(default)]
    pub pins: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SidebarState {
    pub width: f32,
    pub collapsed: bool,
}

impl Default for SidebarState {
    fn default() -> Self {
        Self {
            width: 260.0,
            collapsed: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LayoutState {
    pub split: String,
    pub request_pane: String,
}

impl Default for LayoutState {
    fn default() -> Self {
        Self {
            split: "horizontal".to_string(),
            request_pane: "editor".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct PerRequestState {
    pub tab: String,
    pub response_scroll: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct LocalState {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_request: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_environment: Option<String>,
    #[serde(default)]
    pub recent: Vec<String>,
    #[serde(default)]
    pub expanded_folders: Vec<String>,
    #[serde(default)]
    pub sidebar: SidebarState,
    #[serde(default)]
    pub layout: LayoutState,
    #[serde(default)]
    pub per_request: BTreeMap<String, PerRequestState>,
}

#[derive(Debug, Clone)]
pub struct RequestNode {
    pub slug: String,
    pub rel_path: String,
    pub file: RequestFile,
    pub last_status_code: Option<u16>,
    pub parse_error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct FolderNode {
    pub dir_name: String,
    pub rel_path: String,
    pub file: FolderFile,
    pub requests: Vec<RequestNode>,
    pub subfolders: Vec<FolderNode>,
}

#[derive(Debug, Clone, Default)]
pub struct WorkspaceTree {
    pub root_requests: Vec<RequestNode>,
    pub folders: Vec<FolderNode>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Environment, EnvironmentVariable};

    #[test]
    fn test_environment_file_conversion_and_secrets() {
        let mut env = Environment::new("ws1", "Development");
        env.id = "env-dev".to_string();
        env.variables.push(EnvironmentVariable {
            key: "BASE_URL".to_string(),
            initial_value: "https://api.example.com".to_string(),
            current_value: "http://localhost:8000".to_string(),
            secret: false,
        });
        env.variables.push(EnvironmentVariable {
            key: "TIMEOUT".to_string(),
            initial_value: "30".to_string(),
            current_value: "".to_string(),
            secret: false,
        });
        env.variables.push(EnvironmentVariable {
            key: "API_KEY".to_string(),
            initial_value: "prod-secret".to_string(),
            current_value: "local-dev-secret".to_string(),
            secret: true,
        });

        let (env_file, secrets) = EnvironmentFile::from_environment(&env);

        // Env file only holds initial_value for non-secrets, None for secrets
        assert_eq!(env_file.variables.len(), 3);
        assert_eq!(env_file.variables[0].name, "BASE_URL");
        assert_eq!(env_file.variables[0].value.as_deref(), Some("https://api.example.com"));
        assert!(!env_file.variables[0].secret);

        assert_eq!(env_file.variables[1].name, "TIMEOUT");
        assert_eq!(env_file.variables[1].value.as_deref(), Some("30"));
        assert!(!env_file.variables[1].secret);

        assert_eq!(env_file.variables[2].name, "API_KEY");
        assert_eq!(env_file.variables[2].value, None);
        assert!(env_file.variables[2].secret);

        // Secrets hold current values and secret values
        assert_eq!(secrets.get("BASE_URL").map(|s| s.as_str()), Some("http://localhost:8000"));
        assert!(!secrets.contains_key("TIMEOUT")); // empty current value not stored
        assert_eq!(secrets.get("API_KEY").map(|s| s.as_str()), Some("local-dev-secret"));

        // Converting back from EnvironmentFile + secrets
        let restored = env_file.to_environment("env-dev", "ws1", "Development", &secrets);
        assert_eq!(restored.variables.len(), 3);

        let v0 = &restored.variables[0];
        assert_eq!(v0.key, "BASE_URL");
        assert_eq!(v0.initial_value, "https://api.example.com");
        assert_eq!(v0.current_value, "http://localhost:8000");
        assert_eq!(v0.resolved_value(), "http://localhost:8000");

        let v1 = &restored.variables[1];
        assert_eq!(v1.key, "TIMEOUT");
        assert_eq!(v1.initial_value, "30");
        assert_eq!(v1.current_value, "");
        assert_eq!(v1.resolved_value(), "30");

        let v2 = &restored.variables[2];
        assert_eq!(v2.key, "API_KEY");
        assert_eq!(v2.current_value, "local-dev-secret");
        assert_eq!(v2.resolved_value(), "local-dev-secret");
        assert!(v2.secret);
    }

    #[test]
    fn test_loc_file_to_record_status_text() {
        let loc = LocFile {
            sent_at: "2026-10-21T10:00:00Z".to_string(),
            request: LocRequestInfo {
                method: crate::models::HttpMethod::Get,
                url: "https://api.example.com".to_string(),
                headers: vec![],
                body: None,
            },
            response: LocResponseInfo {
                status: 200,
                duration_ms: 50,
                size: 10,
                headers: vec![],
                body: Some("{}".to_string()),
                truncated: false,
                binary: false,
            },
        };
        let rec = loc.to_record("req-1");
        assert_eq!(rec.status_code, 200);
        assert_eq!(rec.status_text, "OK");

        let mut loc_404 = loc;
        loc_404.response.status = 404;
        let rec_404 = loc_404.to_record("req-2");
        assert_eq!(rec_404.status_code, 404);
        assert_eq!(rec_404.status_text, "Not Found");
    }
}

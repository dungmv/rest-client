use crate::models::{
    ConsoleLogEntry, Environment, EnvironmentVariable, HttpRequestItem, HttpResponseRecord,
    KeyValuePair, RequestBody, TestResult,
};

#[derive(Debug, Clone, Default)]
pub struct PreRequestContext {
    pub env_vars: Vec<EnvironmentVariable>,
    pub request_headers: Vec<KeyValuePair>,
    pub request_body_json: Option<String>,
}

impl PreRequestContext {
    pub fn from_request_and_env(item: &HttpRequestItem, env: Option<&Environment>) -> Self {
        let env_vars = env.map(|e| e.variables.clone()).unwrap_or_default();
        let request_body_json = match &item.body {
            RequestBody::Json(raw_json) => Some(raw_json.clone()),
            _ => None,
        };

        Self {
            env_vars,
            request_headers: item.headers.clone(),
            request_body_json,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct PreRequestOutput {
    /// Danh sách (key, Some(val)) nếu set, hoặc (key, None) nếu unset
    pub env_mutations: Vec<(String, Option<String>)>,
    /// Headers thêm mới hoặc override
    pub header_mutations: Vec<(String, String)>,
    /// Nếu script gọi client.request.body.setJson(...)
    pub body_override: Option<String>,
    /// Lỗi khi thực thi script (nếu có, không làm crash app)
    pub error: Option<String>,
    /// Log từ console.log(...)
    pub console_logs: Vec<String>,
    /// Danh sách console logs có cấu trúc (source, level, timestamp)
    pub console_entries: Vec<ConsoleLogEntry>,
}

#[derive(Debug, Clone)]
pub struct PostResponseContext {
    pub env_vars: Vec<EnvironmentVariable>,
    pub status_code: u16,
    pub response_time_ms: u64,
    pub response_headers: Vec<(String, String)>,
    /// Body dạng chuỗi UTF-8 (giới hạn an toàn tối đa 10MB)
    pub response_body: String,
}

impl PostResponseContext {
    pub fn from_response_and_env(record: &HttpResponseRecord, env: Option<&Environment>) -> Self {
        let env_vars = env.map(|e| e.variables.clone()).unwrap_or_default();
        let response_body = match record.body_as_str() {
            Ok(s) => {
                const MAX_STR_LEN: usize = 10 * 1024 * 1024;
                if s.len() > MAX_STR_LEN {
                    s[..MAX_STR_LEN].to_string()
                } else {
                    s.to_string()
                }
            }
            Err(_) => String::new(),
        };

        Self {
            env_vars,
            status_code: record.status_code,
            response_time_ms: record.duration_ms,
            response_headers: record.headers.clone(),
            response_body,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct PostResponseOutput {
    pub env_mutations: Vec<(String, Option<String>)>,
    pub test_results: Vec<TestResult>,
    pub console_logs: Vec<String>,
    pub console_entries: Vec<ConsoleLogEntry>,
    pub error: Option<String>,
}

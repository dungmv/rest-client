use crate::http::timing::probe_connection;
use crate::interpolation::interpolate;
use crate::models::{
    Auth, Environment, HttpMethod, HttpRequestItem, HttpResponseRecord, KeyValuePair,
    MultipartItem, RequestBody, RequestSentInfo, RequestSettings, TimingBreakdown,
};
use chrono::Utc;
use cookie_store::CookieStore;
use futures_util::StreamExt;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest_cookie_store::CookieStoreMutex;
use std::str::FromStr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use thiserror::Error;
use tokio::sync::mpsc::UnboundedSender;

/// Extracts base URL and query parameters from a URL string
pub fn parse_query_from_url(raw_url: &str) -> (String, Vec<KeyValuePair>) {
    if let Some((base, query)) = raw_url.split_once('?') {
        let mut pairs = Vec::new();
        for part in query.split('&') {
            if part.is_empty() {
                continue;
            }
            if let Some((k, v)) = part.split_once('=') {
                let k_norm = k.replace('+', " ");
                let v_norm = v.replace('+', " ");
                let dec_k = urlencoding::decode(&k_norm).unwrap_or(std::borrow::Cow::Borrowed(&k_norm)).to_string();
                let dec_v = urlencoding::decode(&v_norm).unwrap_or(std::borrow::Cow::Borrowed(&v_norm)).to_string();
                pairs.push(KeyValuePair::new(dec_k, dec_v));
            } else {
                let part_norm = part.replace('+', " ");
                let dec_k = urlencoding::decode(&part_norm).unwrap_or(std::borrow::Cow::Borrowed(&part_norm)).to_string();
                pairs.push(KeyValuePair::new(dec_k, ""));
            }
        }
        (base.to_string(), pairs)
    } else {
        (raw_url.to_string(), Vec::new())
    }
}

/// Rebuilds a URL with the given enabled query parameters
pub fn build_url_with_query(base_url: &str, params: &[KeyValuePair]) -> String {
    let base = if let Some((b, _)) = base_url.split_once('?') {
        b
    } else {
        base_url
    };

    let active_params: Vec<_> = params
        .iter()
        .filter(|p| p.enabled && !p.key.trim().is_empty())
        .map(|p| {
            format!(
                "{}={}",
                urlencoding::encode(&p.key),
                urlencoding::encode(&p.value)
            )
        })
        .collect();

    if active_params.is_empty() {
        base.to_string()
    } else {
        format!("{}?{}", base, active_params.join("&"))
    }
}

#[derive(Error, Debug)]
pub enum HttpError {
    #[error("Request cancelled")]
    Cancelled,
    #[error("Network error: {0}")]
    Network(String),
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),
    #[error("Invalid Header: {0}")]
    InvalidHeader(String),
    #[error("Timeout after {0} seconds")]
    Timeout(u64),
    #[error("Pre-request script error: {0}")]
    ScriptError(String),
    #[error("Execution error: {0}")]
    Other(String),
}

#[derive(Debug, Clone)]
pub enum HttpProgress {
    Connecting,
    Sending,
    WaitingResponse,
    Downloading {
        received_bytes: usize,
        total_bytes: Option<usize>,
    },
}

#[derive(Debug, Clone)]
pub struct ResolvedRequest {
    pub request_id: String,
    pub method: HttpMethod,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<Vec<u8>>,
    pub multipart: Option<Vec<MultipartItem>>,
    pub sent_info: RequestSentInfo,
    pub settings: RequestSettings,
}

impl ResolvedRequest {
    pub fn from_item(item: &HttpRequestItem, env: Option<&Environment>) -> Result<Self, HttpError> {
        let raw_url = item.url.trim();
        let interpolated_url = interpolate(raw_url, env).text;

        // Parse and assemble query params
        let mut full_url = if interpolated_url.starts_with("http://")
            || interpolated_url.starts_with("https://")
        {
            interpolated_url
        } else {
            format!("http://{}", interpolated_url)
        };

        // Append query params
        let mut enabled_params = Vec::new();
        for p in &item.params {
            if p.enabled && !p.key.trim().is_empty() {
                let k = interpolate(&p.key, env).text;
                let v = interpolate(&p.value, env).text;
                enabled_params.push((k, v));
            }
        }

        // Check if auth is ApiKey in query
        if let Auth::ApiKey { key, value, in_header } = &item.auth {
            if !in_header && !key.trim().is_empty() {
                let k = interpolate(key, env).text;
                let v = interpolate(value, env).text;
                enabled_params.push((k, v));
            }
        }

        if !enabled_params.is_empty() {
            let delimiter = if full_url.contains('?') { "&" } else { "?" };
            let query_str = enabled_params
                .into_iter()
                .map(|(k, v)| {
                    format!(
                        "{}={}",
                        urlencoding::encode(&k),
                        urlencoding::encode(&v)
                    )
                })
                .collect::<Vec<_>>()
                .join("&");
            full_url.push_str(delimiter);
            full_url.push_str(&query_str);
        }

        // Headers
        let mut headers = Vec::new();
        let mut content_type_set = false;

        for h in &item.headers {
            if h.enabled && !h.key.trim().is_empty() {
                let k = interpolate(&h.key, env).text;
                let v = interpolate(&h.value, env).text;
                if k.eq_ignore_ascii_case("content-type") {
                    content_type_set = true;
                }
                headers.push((k, v));
            }
        }

        // Auth handling
        match &item.auth {
            Auth::None => {}
            Auth::Bearer { token } => {
                let t = interpolate(token, env).text;
                headers.push(("Authorization".to_string(), format!("Bearer {}", t)));
            }
            Auth::Basic { username, password } => {
                let u = interpolate(username, env).text;
                let p = interpolate(password, env).text;
                use base64::Engine as _;
                let creds = format!("{}:{}", u, p);
                let encoded = base64::engine::general_purpose::STANDARD.encode(creds.as_bytes());
                headers.push(("Authorization".to_string(), format!("Basic {}", encoded)));
            }
            Auth::ApiKey { key, value, in_header } => {
                if *in_header && !key.trim().is_empty() {
                    let k = interpolate(key, env).text;
                    let v = interpolate(value, env).text;
                    headers.push((k, v));
                }
            }
        }

        // Body handling
        let mut multipart_data = None;
        let body_bytes = match &item.body {
            RequestBody::None => None,
            RequestBody::Json(raw_json) => {
                if !content_type_set {
                    headers.push(("Content-Type".to_string(), "application/json".to_string()));
                }
                let interpolated = interpolate(raw_json, env).text;
                Some(interpolated.into_bytes())
            }
            RequestBody::FormUrlEncoded(pairs) => {
                if !content_type_set {
                    headers.push((
                        "Content-Type".to_string(),
                        "application/x-www-form-urlencoded".to_string(),
                    ));
                }
                let form_str = pairs
                    .iter()
                    .filter(|p| p.enabled && !p.key.trim().is_empty())
                    .map(|p| {
                        let k = interpolate(&p.key, env).text;
                        let v = interpolate(&p.value, env).text;
                        format!("{}={}", urlencoding::encode(&k), urlencoding::encode(&v))
                    })
                    .collect::<Vec<_>>()
                    .join("&");
                Some(form_str.into_bytes())
            }
            RequestBody::Multipart(items) => {
                let interpolated_items: Vec<MultipartItem> = items
                    .iter()
                    .filter(|p| p.enabled && !p.key.trim().is_empty())
                    .map(|p| {
                        let k = interpolate(&p.key, env).text;
                        let v = interpolate(&p.value, env).text;
                        let file_path = p.file_path.as_ref().map(|path| interpolate(path, env).text);
                        MultipartItem {
                            enabled: true,
                            key: k,
                            value: v,
                            is_file: p.is_file,
                            file_path,
                        }
                    })
                    .collect();
                multipart_data = Some(interpolated_items);
                None
            }
            RequestBody::Raw { content, content_type } => {
                if !content_type_set && !content_type.trim().is_empty() {
                    headers.push(("Content-Type".to_string(), content_type.clone()));
                }
                let interpolated = interpolate(content, env).text;
                Some(interpolated.into_bytes())
            }
            RequestBody::Binary { file_path } => {
                if !content_type_set {
                    headers.push(("Content-Type".to_string(), "application/octet-stream".to_string()));
                }
                let interpolated_path = interpolate(file_path, env).text;
                std::fs::read(&interpolated_path).ok()
            }
        };

        let body_preview = if let Some(b) = &body_bytes {
            if let Ok(s) = std::str::from_utf8(b) {
                Some(s.chars().take(2000).collect())
            } else {
                Some(format!("[Binary data: {} bytes]", b.len()))
            }
        } else if let Some(parts) = &multipart_data {
            let desc = parts
                .iter()
                .map(|p| {
                    if p.is_file {
                        format!("{} = @{}", p.key, p.file_path.as_deref().unwrap_or("file"))
                    } else {
                        format!("{} = {}", p.key, p.value)
                    }
                })
                .collect::<Vec<_>>()
                .join(", ");
            Some(format!("[Multipart: {}]", desc))
        } else {
            None
        };

        let body_size_bytes = body_bytes.as_ref().map(|b| b.len()).unwrap_or(0);

        let sent_info = RequestSentInfo {
            method: item.method,
            url: full_url.clone(),
            headers: headers.clone(),
            body_size_bytes,
            body_preview,
        };

        Ok(ResolvedRequest {
            request_id: item.id.clone(),
            method: item.method,
            url: full_url,
            headers,
            body: body_bytes,
            multipart: multipart_data,
            sent_info,
            settings: item.settings.clone(),
        })
    }

    /// Áp dụng các thay đổi từ Pre-request Script (headers, body JSON override)
    pub fn apply_pre_script_mutations(&mut self, pre_output: &crate::scripting::PreRequestOutput) {
        // Ghi đè hoặc thêm headers mới
        for (k, v) in &pre_output.header_mutations {
            self.headers.retain(|(name, _)| !name.eq_ignore_ascii_case(k));
            self.headers.push((k.clone(), v.clone()));
            self.sent_info.headers.retain(|(name, _)| !name.eq_ignore_ascii_case(k));
            self.sent_info.headers.push((k.clone(), v.clone()));
        }

        // Ghi đè body nếu có
        if let Some(new_body_str) = &pre_output.body_override {
            let bytes = new_body_str.clone().into_bytes();
            self.sent_info.body_size_bytes = bytes.len();
            self.sent_info.body_preview = Some(new_body_str.chars().take(2000).collect());
            self.body = Some(bytes);
        }
    }
}

pub trait HttpExecutor: Send + Sync {
    fn send(
        &self,
        req: ResolvedRequest,
        progress_tx: Option<UnboundedSender<HttpProgress>>,
    ) -> impl std::future::Future<Output = Result<HttpResponseRecord, HttpError>> + Send;
}

pub struct ReqwestExecutor {
    cookie_store: Arc<CookieStoreMutex>,
}

impl ReqwestExecutor {
    pub fn new() -> Self {
        let cookie_store = Arc::new(CookieStoreMutex::new(CookieStore::default()));
        Self { cookie_store }
    }

    pub fn cookie_store(&self) -> Arc<CookieStoreMutex> {
        self.cookie_store.clone()
    }

    pub fn get_cookies_for_url(&self, url: &str) -> Vec<String> {
        let Ok(parsed_url) = url::Url::parse(url) else {
            return Vec::new();
        };
        let store = self.cookie_store.lock().unwrap();
        store
            .matches(&parsed_url)
            .into_iter()
            .map(|c| c.to_string())
            .collect()
    }

    pub fn get_all_cookies(&self) -> Vec<String> {
        let store = self.cookie_store.lock().unwrap();
        store.iter_any().map(|c| c.to_string()).collect()
    }

    pub fn clear_cookies(&self) {
        let mut store = self.cookie_store.lock().unwrap();
        *store = CookieStore::default();
    }
}

impl Default for ReqwestExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpExecutor for ReqwestExecutor {
    async fn send(
        &self,
        req: ResolvedRequest,
        progress_tx: Option<UnboundedSender<HttpProgress>>,
    ) -> Result<HttpResponseRecord, HttpError> {
        if let Some(tx) = &progress_tx {
            let _ = tx.send(HttpProgress::Connecting);
        }

        let start_time = Instant::now();

        let reqwest_method = match req.method {
            HttpMethod::Get => reqwest::Method::GET,
            HttpMethod::Post => reqwest::Method::POST,
            HttpMethod::Put => reqwest::Method::PUT,
            HttpMethod::Delete => reqwest::Method::DELETE,
            HttpMethod::Patch => reqwest::Method::PATCH,
            HttpMethod::Head => reqwest::Method::HEAD,
            HttpMethod::Options => reqwest::Method::OPTIONS,
        };

        let mut client_builder = reqwest::Client::builder()
            .cookie_provider(self.cookie_store.clone())
            .timeout(Duration::from_secs(req.settings.timeout_secs))
            .danger_accept_invalid_certs(!req.settings.validate_tls);

        if !req.settings.follow_redirects {
            client_builder = client_builder.redirect(reqwest::redirect::Policy::none());
        }

        let client = client_builder
            .build()
            .map_err(|e| HttpError::Network(e.to_string()))?;

        let mut request_builder = client.request(reqwest_method, &req.url);

        let mut header_map = HeaderMap::new();
        for (k, v) in &req.headers {
            if let (Ok(h_name), Ok(h_val)) = (
                HeaderName::from_str(k),
                HeaderValue::from_str(v),
            ) {
                header_map.insert(h_name, h_val);
            }
        }
        request_builder = request_builder.headers(header_map);

        if let Some(multipart_items) = req.multipart {
            let mut form = reqwest::multipart::Form::new();
            for item in multipart_items {
                if item.is_file {
                    if let Some(path) = &item.file_path {
                        if let Ok(file_bytes) = std::fs::read(path) {
                            let file_name = std::path::Path::new(path)
                                .file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("file")
                                .to_string();
                            let part = reqwest::multipart::Part::bytes(file_bytes).file_name(file_name);
                            form = form.part(item.key.clone(), part);
                        }
                    }
                } else {
                    form = form.text(item.key.clone(), item.value.clone());
                }
            }
            request_builder = request_builder.multipart(form);
        } else if let Some(body_data) = req.body {
            request_builder = request_builder.body(body_data);
        }

        if let Some(tx) = &progress_tx {
            let _ = tx.send(HttpProgress::Sending);
        }

        let probe = probe_connection(&req.url, req.settings.validate_tls).await;

        let ttfb_start = Instant::now();
        let response = request_builder
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    HttpError::Timeout(req.settings.timeout_secs)
                } else {
                    HttpError::Network(e.to_string())
                }
            })?;

        let ttfb_ms = ttfb_start.elapsed().as_millis() as u64;

        if let Some(tx) = &progress_tx {
            let _ = tx.send(HttpProgress::WaitingResponse);
        }

        let status = response.status();
        let status_code = status.as_u16();
        let status_text = status.canonical_reason().unwrap_or("").to_string();

        let mut resp_headers = Vec::new();
        for (name, val) in response.headers() {
            if let Ok(v_str) = val.to_str() {
                resp_headers.push((name.as_str().to_string(), v_str.to_string()));
            }
        }

        let total_size = response.content_length().map(|s| s as usize);

        let download_start = Instant::now();
        let mut body_bytes = Vec::new();
        let mut stream = response.bytes_stream();

        while let Some(chunk_res) = stream.next().await {
            let chunk = chunk_res.map_err(|e| HttpError::Network(e.to_string()))?;
            body_bytes.extend_from_slice(&chunk);
            if let Some(tx) = &progress_tx {
                let _ = tx.send(HttpProgress::Downloading {
                    received_bytes: body_bytes.len(),
                    total_bytes: total_size,
                });
            }
        }
        let download_ms = download_start.elapsed().as_millis() as u64;

        let total_ms = start_time.elapsed().as_millis() as u64;

        let (dns_ms, connect_ms, tls_ms) = if let Some(p) = probe {
            (Some(p.dns_ms), Some(p.connect_ms), p.tls_ms)
        } else {
            let connect_approx = total_ms.saturating_sub(ttfb_ms + download_ms);
            (Some(connect_approx / 3), Some(connect_approx * 2 / 3), None)
        };
        let connection_reused = dns_ms.unwrap_or(0) == 0 && connect_ms.unwrap_or(0) == 0;
        let timing = TimingBreakdown {
            dns_ms,
            connect_ms,
            tls_ms,
            ttfb_ms: Some(ttfb_ms),
            download_ms: Some(download_ms),
            total_ms,
            connection_reused,
        };

        // Extract cookies
        let cookies = self.get_cookies_for_url(&req.url);

        Ok(HttpResponseRecord {
            id: uuid::Uuid::new_v4().to_string(),
            request_id: req.request_id,
            status_code,
            status_text,
            duration_ms: total_ms,
            size_bytes: body_bytes.len(),
            headers: resp_headers,
            cookies,
            body: body_bytes,
            timing,
            request_sent: Some(req.sent_info),
            test_results: Vec::new(),
            console_logs: Vec::new(),
            created_at: Utc::now(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{HttpMethod, HttpRequestItem, KeyValuePair, MultipartItem, RequestBody};

    #[test]
    fn test_parse_query_from_url() {
        let url = "https://api.example.com/search?q=rust+lang&page=1&empty=&flag";
        let (base, params) = parse_query_from_url(url);
        assert_eq!(base, "https://api.example.com/search");
        assert_eq!(params.len(), 4);
        assert_eq!(params[0].key, "q");
        assert_eq!(params[0].value, "rust lang");
        assert_eq!(params[1].key, "page");
        assert_eq!(params[1].value, "1");
        assert_eq!(params[2].key, "empty");
        assert_eq!(params[2].value, "");
        assert_eq!(params[3].key, "flag");
        assert_eq!(params[3].value, "");
    }

    #[test]
    fn test_build_url_with_query() {
        let base = "https://api.example.com/items";
        let params = vec![
            KeyValuePair::new("type", "tools"),
            KeyValuePair {
                enabled: false,
                key: "disabled".to_string(),
                value: "val".to_string(),
                description: None,
            },
            KeyValuePair::new("limit", "25"),
        ];
        let built = build_url_with_query(base, &params);
        assert_eq!(built, "https://api.example.com/items?type=tools&limit=25");
    }

    #[test]
    fn test_resolved_request_multipart() {
        let mut item = HttpRequestItem::new("ws-1", "Test Multipart", HttpMethod::Post, "http://localhost:8080/upload");
        item.body = RequestBody::Multipart(vec![
            MultipartItem::new_text("description", "A test file upload"),
            MultipartItem::new_file("file", "C:/dummy/path.png"),
        ]);

        let resolved = ResolvedRequest::from_item(&item, None).unwrap();
        assert!(resolved.multipart.is_some());
        let parts = resolved.multipart.unwrap();
        assert_eq!(parts.len(), 2);
        assert!(!parts[0].is_file);
        assert_eq!(parts[0].key, "description");
        assert!(parts[1].is_file);
        assert_eq!(parts[1].key, "file");
        assert!(resolved.sent_info.body_preview.is_some());
    }
}

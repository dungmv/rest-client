use crate::interpolation::interpolate;
use crate::models::{
    Auth, Environment, HttpMethod, HttpRequestItem, HttpResponseRecord, RequestBody,
    RequestSettings, TimingBreakdown,
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
            RequestBody::Multipart(_) => {
                // Simplified text multipart or raw
                None
            }
            RequestBody::Raw { content, content_type } => {
                if !content_type_set && !content_type.trim().is_empty() {
                    headers.push(("Content-Type".to_string(), content_type.clone()));
                }
                let interpolated = interpolate(content, env).text;
                Some(interpolated.into_bytes())
            }
            RequestBody::Binary { file_path } => std::fs::read(file_path).ok(),
        };

        Ok(ResolvedRequest {
            request_id: item.id.clone(),
            method: item.method,
            url: full_url,
            headers,
            body: body_bytes,
            settings: item.settings.clone(),
        })
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

        if let Some(body_data) = req.body {
            request_builder = request_builder.body(body_data);
        }

        if let Some(tx) = &progress_tx {
            let _ = tx.send(HttpProgress::Sending);
        }

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

        // Timing breakdown approximation
        let connect_ms = total_ms.saturating_sub(ttfb_ms + download_ms);
        let timing = TimingBreakdown {
            dns_ms: Some(connect_ms / 3),
            connect_ms: Some(connect_ms * 2 / 3),
            tls_ms: None,
            ttfb_ms: Some(ttfb_ms),
            download_ms: Some(download_ms),
            total_ms,
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
            created_at: Utc::now(),
        })
    }
}

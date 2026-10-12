use crate::interpolation::interpolate;
use crate::models::{
    Auth, Environment, HttpMethod, HttpRequestItem, KeyValuePair, RequestBody,
};
use anyhow::{anyhow, Result};
use std::str::FromStr;

/// Parse a cURL command into an HttpRequestItem
pub fn curl_to_request(workspace_id: &str, curl_str: &str) -> Result<HttpRequestItem> {
    let clean = curl_str.trim().replace("\\\n", " ").replace("\\\r\n", " ");
    let tokens = tokenize_command(&clean)?;

    if tokens.is_empty() {
        return Err(anyhow!("Empty command"));
    }

    let mut start_idx = 0;
    if tokens[0].eq_ignore_ascii_case("curl") {
        start_idx = 1;
    }

    let mut method = HttpMethod::Get;
    let mut method_explicit = false;
    let mut url = String::new();
    let mut headers = Vec::new();
    let mut body_data = None;
    let mut auth = Auth::None;

    let mut idx = start_idx;
    while idx < tokens.len() {
        let token = &tokens[idx];

        if token == "-X" || token == "--request" {
            idx += 1;
            if idx < tokens.len() {
                if let Ok(m) = HttpMethod::from_str(&tokens[idx]) {
                    method = m;
                    method_explicit = true;
                }
            }
        } else if token == "-H" || token == "--header" {
            idx += 1;
            if idx < tokens.len() {
                let h_str = &tokens[idx];
                if let Some((k, v)) = h_str.split_once(':') {
                    let k = k.trim().to_string();
                    let v = v.trim().to_string();
                    if k.eq_ignore_ascii_case("authorization") && v.starts_with("Bearer ") {
                        auth = Auth::Bearer {
                            token: v.trim_start_matches("Bearer ").trim().to_string(),
                        };
                    } else {
                        headers.push(KeyValuePair::new(k, v));
                    }
                }
            }
        } else if token == "-d" || token == "--data" || token == "--data-raw" || token == "--data-binary" {
            idx += 1;
            if idx < tokens.len() {
                body_data = Some(tokens[idx].clone());
                if !method_explicit {
                    method = HttpMethod::Post;
                }
            }
        } else if token == "-u" || token == "--user" {
            idx += 1;
            if idx < tokens.len() {
                let u_str = &tokens[idx];
                let (user, pass) = u_str.split_once(':').unwrap_or((u_str, ""));
                auth = Auth::Basic {
                    username: user.to_string(),
                    password: pass.to_string(),
                };
            }
        } else if !token.starts_with('-') && url.is_empty() {
            url = token.clone();
        }

        idx += 1;
    }

    if url.is_empty() {
        return Err(anyhow!("No URL found in cURL command"));
    }

    // Determine request body
    let body = if let Some(data) = body_data {
        if data.trim().starts_with('{') && data.trim().ends_with('}') {
            RequestBody::Json(data)
        } else {
            RequestBody::Raw {
                content: data,
                content_type: "text/plain".to_string(),
            }
        }
    } else {
        RequestBody::None
    };

    let name = url
        .split('/')
        .rfind(|s| !s.is_empty() && !s.contains(':'))
        .unwrap_or("Imported Request")
        .to_string();

    let mut req = HttpRequestItem::new(workspace_id, name, method, url);
    req.headers = headers;
    req.auth = auth;
    req.body = body;

    Ok(req)
}

/// Export an HttpRequestItem to a standard cURL command string
pub fn request_to_curl(item: &HttpRequestItem, env: Option<&Environment>) -> String {
    let mut parts = vec!["curl".to_string()];

    // Method
    parts.push("-X".to_string());
    parts.push(item.method.as_str().to_string());

    // URL
    let url = interpolate(&item.url, env).text;
    parts.push(format!("'{}'", url.replace('\'', "'\\''")));

    // Headers
    for h in &item.headers {
        if h.enabled && !h.key.trim().is_empty() {
            let k = interpolate(&h.key, env).text;
            let v = interpolate(&h.value, env).text;
            parts.push("-H".to_string());
            parts.push(format!("'{}: {}'", k.replace('\'', "'\\''"), v.replace('\'', "'\\''")));
        }
    }

    // Auth
    match &item.auth {
        Auth::None => {}
        Auth::Bearer { token } => {
            let t = interpolate(token, env).text;
            parts.push("-H".to_string());
            parts.push(format!("'Authorization: Bearer {}'", t.replace('\'', "'\\''")));
        }
        Auth::Basic { username, password } => {
            let u = interpolate(username, env).text;
            let p = interpolate(password, env).text;
            parts.push("-u".to_string());
            parts.push(format!("'{}:{}'", u.replace('\'', "'\\''"), p.replace('\'', "'\\''")));
        }
        Auth::ApiKey { key, value, in_header } => {
            if *in_header && !key.trim().is_empty() {
                let k = interpolate(key, env).text;
                let v = interpolate(value, env).text;
                parts.push("-H".to_string());
                parts.push(format!("'{}: {}'", k.replace('\'', "'\\''"), v.replace('\'', "'\\''")));
            }
        }
    }

    // Body
    match &item.body {
        RequestBody::None => {}
        RequestBody::Json(raw_json) => {
            let interpolated = interpolate(raw_json, env).text;
            parts.push("-H".to_string());
            parts.push("'Content-Type: application/json'".to_string());
            parts.push("-d".to_string());
            parts.push(format!("'{}'", interpolated.replace('\'', "'\\''")));
        }
        RequestBody::Raw { content, content_type } => {
            let interpolated = interpolate(content, env).text;
            if !content_type.trim().is_empty() {
                parts.push("-H".to_string());
                parts.push(format!("'Content-Type: {}'", content_type.replace('\'', "'\\''")));
            }
            parts.push("-d".to_string());
            parts.push(format!("'{}'", interpolated.replace('\'', "'\\''")));
        }
        RequestBody::FormUrlEncoded(pairs) => {
            for p in pairs {
                if p.enabled && !p.key.trim().is_empty() {
                    let k = interpolate(&p.key, env).text;
                    let v = interpolate(&p.value, env).text;
                    parts.push("--data-urlencode".to_string());
                    parts.push(format!("'{}={}'", k.replace('\'', "'\\''"), v.replace('\'', "'\\''")));
                }
            }
        }
        _ => {}
    }

    parts.join(" ")
}

fn tokenize_command(cmd: &str) -> Result<Vec<String>> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut chars = cmd.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '\\' => {
                if let Some(next) = chars.next() {
                    current.push(next);
                }
            }
            '\'' if !in_double_quote => {
                in_single_quote = !in_single_quote;
            }
            '"' if !in_single_quote => {
                in_double_quote = !in_double_quote;
            }
            ' ' | '\t' | '\n' | '\r' if !in_single_quote && !in_double_quote => {
                if !current.is_empty() {
                    tokens.push(current.clone());
                    current.clear();
                }
            }
            _ => {
                current.push(ch);
            }
        }
    }

    if !current.is_empty() {
        tokens.push(current);
    }

    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_curl_post() {
        let curl = r#"curl -X POST "https://api.example.com/v1/users" -H "Content-Type: application/json" -H "Authorization: Bearer mytoken" -d '{"name": "Alice"}'"#;
        let req = curl_to_request("ws1", curl).unwrap();
        assert_eq!(req.method, HttpMethod::Post);
        assert_eq!(req.url, "https://api.example.com/v1/users");
        assert_eq!(req.auth, Auth::Bearer { token: "mytoken".to_string() });
        assert_eq!(req.body, RequestBody::Json("{\"name\": \"Alice\"}".to_string()));
    }

    #[test]
    fn test_export_curl() {
        let mut req = HttpRequestItem::new("ws1", "Test", HttpMethod::Get, "https://httpbin.org/get");
        req.headers.push(KeyValuePair::new("Accept", "application/json"));
        let exported = request_to_curl(&req, None);
        assert!(exported.contains("curl -X GET 'https://httpbin.org/get'"));
        assert!(exported.contains("-H 'Accept: application/json'"));
    }
}

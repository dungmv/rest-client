use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::Instant;

use rquickjs::function::{Opt, Rest};
use rquickjs::{Context, Function, Object, Runtime, Value};

use crate::models::{
    ConsoleLogEntry, ConsoleLogLevel, ConsoleLogSource, EnvironmentVariable, TestResult,
};
use crate::scripting::api::{
    PostResponseContext, PostResponseOutput, PreRequestContext, PreRequestOutput,
};

type EnvMutations = Rc<RefCell<Vec<(String, Option<String>)>>>;

pub struct ScriptSandbox;

impl ScriptSandbox {
    /// Execute Pre-request Script in an isolated QuickJS runtime
    pub fn run_pre_request(code: &str, ctx_data: PreRequestContext) -> PreRequestOutput {
        let rt = match Runtime::new() {
            Ok(r) => r,
            Err(e) => {
                return PreRequestOutput {
                    error: Some(format!("Failed to initialize QuickJS runtime: {}", e)),
                    ..Default::default()
                }
            }
        };

        let ctx = match Context::full(&rt) {
            Ok(c) => c,
            Err(e) => {
                return PreRequestOutput {
                    error: Some(format!("Failed to initialize QuickJS context: {}", e)),
                    ..Default::default()
                }
            }
        };

        let env_mutations: EnvMutations = Default::default();
        let header_mutations: Rc<RefCell<Vec<(String, String)>>> = Default::default();
        let body_override: Rc<RefCell<Option<String>>> = Default::default();
        let console_entries: Rc<RefCell<Vec<ConsoleLogEntry>>> = Default::default();

        let eval_res = ctx.with(|ctx| -> Result<(), String> {
            let env_obj = build_environment_object(&ctx, &ctx_data.env_vars, env_mutations.clone())
                .map_err(|e| format!("{:?}", e))?;
            let vars_obj =
                build_variables_object(&ctx, &ctx_data.env_vars).map_err(|e| format!("{:?}", e))?;
            let req_obj = build_request_object(
                &ctx,
                &ctx_data,
                header_mutations.clone(),
                body_override.clone(),
            )
            .map_err(|e| format!("{:?}", e))?;
            let crypto_obj = build_crypto_object(&ctx).map_err(|e| format!("{:?}", e))?;

            let client = Object::new(ctx.clone()).map_err(|e| format!("{:?}", e))?;
            client
                .set("environment", env_obj.clone())
                .map_err(|e| format!("{:?}", e))?;
            client
                .set("global", env_obj.clone())
                .map_err(|e| format!("{:?}", e))?;
            client
                .set("globals", env_obj.clone())
                .map_err(|e| format!("{:?}", e))?;
            client
                .set("variables", vars_obj)
                .map_err(|e| format!("{:?}", e))?;
            client
                .set("request", req_obj)
                .map_err(|e| format!("{:?}", e))?;
            client
                .set("crypto", crypto_obj)
                .map_err(|e| format!("{:?}", e))?;

            // Dummy test/assert trong pre-request
            let test_fn = Function::new(ctx.clone(), |_name: String, _f: Value| {})
                .map_err(|e| format!("{:?}", e))?;
            client
                .set("test", test_fn)
                .map_err(|e| format!("{:?}", e))?;

            let assert_fn = Function::new(
                ctx.clone(),
                |ctx: rquickjs::Ctx<'_>,
                 condition: bool,
                 msg: Opt<String>|
                 -> rquickjs::Result<()> {
                    if !condition {
                        let m = msg.0.unwrap_or_else(|| "Assertion failed".to_string());
                        let js_str = rquickjs::String::from_str(ctx.clone(), &m)?;
                        Err(ctx.throw(js_str.into()))
                    } else {
                        Ok(())
                    }
                },
            )
            .map_err(|e| format!("{:?}", e))?;
            client
                .set("assert", assert_fn)
                .map_err(|e| format!("{:?}", e))?;

            ctx.globals()
                .set("z", client)
                .map_err(|e| format!("{:?}", e))?;

            inject_console(&ctx, console_entries.clone(), ConsoleLogSource::PreRequest)
                .map_err(|e| format!("{:?}", e))?;

            let eval_result: Result<Value, _> = ctx.eval(code.as_bytes());
            if let Err(e) = eval_result {
                return Err(format_js_error(&ctx, e));
            }

            Ok(())
        });

        let error = eval_res.err();
        let env_mutations = env_mutations.borrow().clone();
        let header_mutations = header_mutations.borrow().clone();
        let body_override = body_override.borrow().clone();
        let mut console_entries = console_entries.borrow().clone();
        if let Some(err) = &error {
            console_entries.push(ConsoleLogEntry {
                source: ConsoleLogSource::PreRequest,
                level: ConsoleLogLevel::Error,
                message: err.clone(),
                timestamp: chrono::Local::now().format("%H:%M:%S%.3f").to_string(),
            });
        }
        let console_logs = console_entries.iter().map(|e| e.message.clone()).collect();

        PreRequestOutput {
            env_mutations,
            header_mutations,
            body_override,
            console_logs,
            console_entries,
            error,
        }
    }

    /// Execute Post-response / Test Script after server response is received
    pub fn run_post_response(code: &str, ctx_data: PostResponseContext) -> PostResponseOutput {
        let rt = match Runtime::new() {
            Ok(r) => r,
            Err(e) => {
                return PostResponseOutput {
                    error: Some(format!("Failed to initialize QuickJS runtime: {}", e)),
                    ..Default::default()
                }
            }
        };

        let ctx = match Context::full(&rt) {
            Ok(c) => c,
            Err(e) => {
                return PostResponseOutput {
                    error: Some(format!("Failed to initialize QuickJS context: {}", e)),
                    ..Default::default()
                }
            }
        };

        let env_mutations: EnvMutations = Default::default();
        let test_results: Rc<RefCell<Vec<TestResult>>> = Default::default();
        let console_entries: Rc<RefCell<Vec<ConsoleLogEntry>>> = Default::default();

        let eval_res = ctx.with(|ctx| -> Result<(), String> {
            let env_obj = build_environment_object(&ctx, &ctx_data.env_vars, env_mutations.clone())
                .map_err(|e| format!("{:?}", e))?;
            let vars_obj =
                build_variables_object(&ctx, &ctx_data.env_vars).map_err(|e| format!("{:?}", e))?;
            let resp_obj =
                build_response_object(&ctx, &ctx_data).map_err(|e| format!("{:?}", e))?;
            let crypto_obj = build_crypto_object(&ctx).map_err(|e| format!("{:?}", e))?;

            let client = Object::new(ctx.clone()).map_err(|e| format!("{:?}", e))?;
            client
                .set("environment", env_obj.clone())
                .map_err(|e| format!("{:?}", e))?;
            client
                .set("global", env_obj.clone())
                .map_err(|e| format!("{:?}", e))?;
            client
                .set("globals", env_obj.clone())
                .map_err(|e| format!("{:?}", e))?;
            client
                .set("variables", vars_obj)
                .map_err(|e| format!("{:?}", e))?;
            client
                .set("response", resp_obj)
                .map_err(|e| format!("{:?}", e))?;
            client
                .set("crypto", crypto_obj)
                .map_err(|e| format!("{:?}", e))?;

            // z.test(name, callback)
            let tr_store = test_results.clone();
            let test_fn = Function::new(
                ctx.clone(),
                move |ctx: rquickjs::Ctx<'_>, name: String, cb: Function<'_>| {
                    let start = Instant::now();
                    let res: rquickjs::Result<Value> = cb.call(());
                    let duration_ms = start.elapsed().as_millis() as u64;

                    let (passed, error_message) = match res {
                        Ok(_) => (true, None),
                        Err(e) => {
                            let err_msg = format_js_error(&ctx, e);
                            (false, Some(err_msg))
                        }
                    };

                    tr_store.borrow_mut().push(TestResult {
                        name,
                        passed,
                        error_message,
                        duration_ms,
                    });
                },
            )
            .map_err(|e| format!("{:?}", e))?;
            client
                .set("test", test_fn)
                .map_err(|e| format!("{:?}", e))?;

            // z.assert(condition, msg?)
            let assert_fn = Function::new(
                ctx.clone(),
                |ctx: rquickjs::Ctx<'_>,
                 condition: bool,
                 msg: Opt<String>|
                 -> rquickjs::Result<()> {
                    if !condition {
                        let m = msg.0.unwrap_or_else(|| "Assertion failed".to_string());
                        let js_str = rquickjs::String::from_str(ctx.clone(), &m)?;
                        Err(ctx.throw(js_str.into()))
                    } else {
                        Ok(())
                    }
                },
            )
            .map_err(|e| format!("{:?}", e))?;
            client
                .set("assert", assert_fn)
                .map_err(|e| format!("{:?}", e))?;

            ctx.globals()
                .set("z", client)
                .map_err(|e| format!("{:?}", e))?;

            inject_console(
                &ctx,
                console_entries.clone(),
                ConsoleLogSource::PostResponse,
            )
            .map_err(|e| format!("{:?}", e))?;

            let eval_result: Result<Value, _> = ctx.eval(code.as_bytes());
            if let Err(e) = eval_result {
                return Err(format_js_error(&ctx, e));
            }

            Ok(())
        });

        let error = eval_res.err();
        let env_mutations = env_mutations.borrow().clone();
        let test_results = test_results.borrow().clone();
        let mut console_entries = console_entries.borrow().clone();
        if let Some(err) = &error {
            console_entries.push(ConsoleLogEntry {
                source: ConsoleLogSource::PostResponse,
                level: ConsoleLogLevel::Error,
                message: err.clone(),
                timestamp: chrono::Local::now().format("%H:%M:%S%.3f").to_string(),
            });
        }
        let console_logs = console_entries.iter().map(|e| e.message.clone()).collect();

        PostResponseOutput {
            env_mutations,
            test_results,
            console_logs,
            console_entries,
            error,
        }
    }
}

fn build_environment_object<'js>(
    ctx: &rquickjs::Ctx<'js>,
    env_vars: &[EnvironmentVariable],
    mutations: EnvMutations,
) -> rquickjs::Result<Object<'js>> {
    let mut initial_map = HashMap::new();
    for v in env_vars {
        initial_map.insert(v.key.clone(), v.resolved_value().to_string());
    }
    let map_cell = Rc::new(RefCell::new(initial_map));

    let obj = Object::new(ctx.clone())?;

    // z.environment.get(key)
    let map_for_get = map_cell.clone();
    let get_fn = Function::new(ctx.clone(), move |key: String| -> Option<String> {
        map_for_get.borrow().get(&key).cloned()
    })?;
    obj.set("get", get_fn)?;

    // z.environment.set(key, val)
    let mut_for_set = mutations.clone();
    let map_for_set = map_cell.clone();
    let set_fn = Function::new(
        ctx.clone(),
        move |ctx: rquickjs::Ctx<'js>, key: String, val: Value<'js>| -> rquickjs::Result<()> {
            let val_str = js_val_to_string(&ctx, &val)?;
            map_for_set
                .borrow_mut()
                .insert(key.clone(), val_str.clone());
            mut_for_set.borrow_mut().push((key, Some(val_str)));
            Ok(())
        },
    )?;
    obj.set("set", set_fn)?;

    // z.environment.unset(key)
    let mut_for_unset = mutations;
    let map_for_unset = map_cell;
    let unset_fn = Function::new(ctx.clone(), move |key: String| {
        map_for_unset.borrow_mut().remove(&key);
        mut_for_unset.borrow_mut().push((key, None));
    })?;
    obj.set("unset", unset_fn)?;

    Ok(obj)
}

fn build_variables_object<'js>(
    ctx: &rquickjs::Ctx<'js>,
    env_vars: &[EnvironmentVariable],
) -> rquickjs::Result<Object<'js>> {
    let mut var_map = HashMap::new();
    for v in env_vars {
        var_map.insert(v.key.clone(), v.resolved_value().to_string());
    }
    let map_cell = Rc::new(RefCell::new(var_map));

    let obj = Object::new(ctx.clone())?;

    let map_for_get = map_cell.clone();
    let get_fn = Function::new(ctx.clone(), move |key: String| -> Option<String> {
        map_for_get.borrow().get(&key).cloned()
    })?;
    obj.set("get", get_fn)?;

    let map_for_set = map_cell.clone();
    let set_fn = Function::new(
        ctx.clone(),
        move |ctx: rquickjs::Ctx<'js>, key: String, val: Value<'js>| -> rquickjs::Result<()> {
            let val_str = js_val_to_string(&ctx, &val)?;
            map_for_set.borrow_mut().insert(key, val_str);
            Ok(())
        },
    )?;
    obj.set("set", set_fn)?;

    let map_for_has = map_cell;
    let has_fn = Function::new(ctx.clone(), move |key: String| -> bool {
        map_for_has.borrow().contains_key(&key)
    })?;
    obj.set("has", has_fn)?;

    Ok(obj)
}

fn build_request_object<'js>(
    ctx: &rquickjs::Ctx<'js>,
    req_ctx: &PreRequestContext,
    header_mutations: Rc<RefCell<Vec<(String, String)>>>,
    body_override: Rc<RefCell<Option<String>>>,
) -> rquickjs::Result<Object<'js>> {
    let req_obj = Object::new(ctx.clone())?;

    // z.request.headers.add(k, v)
    let headers_obj = Object::new(ctx.clone())?;
    let hm_clone = header_mutations;
    let add_fn = Function::new(ctx.clone(), move |name: String, val: String| {
        hm_clone.borrow_mut().push((name, val));
    })?;
    headers_obj.set("add", add_fn)?;
    req_obj.set("headers", headers_obj)?;

    // z.request.body.json() and setJson(obj)
    let body_obj = Object::new(ctx.clone())?;
    let json_content = req_ctx.request_body_json.clone();
    let body_override_clone = body_override.clone();

    let json_fn = Function::new(
        ctx.clone(),
        move |ctx: rquickjs::Ctx<'js>| -> rquickjs::Result<Value<'js>> {
            let active_str = body_override_clone
                .borrow()
                .clone()
                .or_else(|| json_content.clone());

            if let Some(s) = active_str {
                let parsed: Result<serde_json::Value, _> = serde_json::from_str(&s);
                match parsed {
                    Ok(val) => {
                        let json_str = val.to_string();
                        let json_mod: Object = ctx.globals().get("JSON")?;
                        let parse_fn: Function = json_mod.get("parse")?;
                        parse_fn.call((json_str,))
                    }
                    Err(_) => Ok(Value::new_null(ctx.clone())),
                }
            } else {
                Ok(Value::new_null(ctx.clone()))
            }
        },
    )?;
    body_obj.set("json", json_fn)?;

    let bo_for_set = body_override;
    let set_json_fn = Function::new(
        ctx.clone(),
        move |ctx: rquickjs::Ctx<'js>, val: Value<'js>| -> rquickjs::Result<()> {
            let json_mod: Object = ctx.globals().get("JSON")?;
            let stringify_fn: Function = json_mod.get("stringify")?;
            let s: String = stringify_fn.call((val,))?;
            *bo_for_set.borrow_mut() = Some(s);
            Ok(())
        },
    )?;
    body_obj.set("setJson", set_json_fn)?;

    req_obj.set("body", body_obj)?;

    Ok(req_obj)
}

fn build_response_object<'js>(
    ctx: &rquickjs::Ctx<'js>,
    resp_ctx: &PostResponseContext,
) -> rquickjs::Result<Object<'js>> {
    let resp_obj = Object::new(ctx.clone())?;

    resp_obj.set("status", resp_ctx.status_code)?;
    resp_obj.set("time", resp_ctx.response_time_ms)?;

    let headers_map = Object::new(ctx.clone())?;
    for (k, v) in &resp_ctx.response_headers {
        headers_map.set(k.to_lowercase().as_str(), v.as_str())?;
    }
    resp_obj.set("headers", headers_map)?;

    let body_str = resp_ctx.response_body.clone();
    let body_clone_for_json = body_str.clone();

    // z.response.json()
    let json_fn = Function::new(
        ctx.clone(),
        move |ctx: rquickjs::Ctx<'js>| -> rquickjs::Result<Value<'js>> {
            let json_mod: Object = ctx.globals().get("JSON")?;
            let parse_fn: Function = json_mod.get("parse")?;
            parse_fn.call((body_clone_for_json.clone(),))
        },
    )?;
    resp_obj.set("json", json_fn)?;

    // z.response.text()
    let text_fn = Function::new(ctx.clone(), move || -> String { body_str.clone() })?;
    resp_obj.set("text", text_fn)?;

    Ok(resp_obj)
}

fn build_crypto_object<'js>(ctx: &rquickjs::Ctx<'js>) -> rquickjs::Result<Object<'js>> {
    use base64::Engine as _;
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    let crypto = Object::new(ctx.clone())?;

    // hmacSha256(secret: string, data: string) -> string (hex)
    let hmac_fn = Function::new(
        ctx.clone(),
        |ctx: rquickjs::Ctx<'js>, secret: String, data: String| -> rquickjs::Result<String> {
            let mut mac = match Hmac::<Sha256>::new_from_slice(secret.as_bytes()) {
                Ok(m) => m,
                Err(_) => {
                    let js_str = rquickjs::String::from_str(ctx.clone(), "Invalid HMAC key")?;
                    return Err(ctx.throw(js_str.into()));
                }
            };
            mac.update(data.as_bytes());
            Ok(hex::encode(mac.finalize().into_bytes()))
        },
    )?;
    crypto.set("hmacSha256", hmac_fn)?;

    // md5(data: string) -> string (hex)
    let md5_fn = Function::new(
        ctx.clone(),
        |_ctx: rquickjs::Ctx<'js>, data: String| -> rquickjs::Result<String> {
            use md5::Digest as _;
            Ok(hex::encode(md5::Md5::digest(data.as_bytes())))
        },
    )?;
    crypto.set("md5", md5_fn)?;

    // base64Encode(str: string) -> string
    let b64_enc_fn = Function::new(ctx.clone(), |s: String| -> String {
        base64::engine::general_purpose::STANDARD.encode(s.as_bytes())
    })?;
    crypto.set("base64Encode", b64_enc_fn)?;

    // base64Decode(str: string) -> string
    let b64_dec_fn = Function::new(
        ctx.clone(),
        |ctx: rquickjs::Ctx<'js>, s: String| -> rquickjs::Result<String> {
            let bytes = match base64::engine::general_purpose::STANDARD.decode(s.as_bytes()) {
                Ok(b) => b,
                Err(_) => {
                    let js_str = rquickjs::String::from_str(ctx.clone(), "Invalid Base64 input")?;
                    return Err(ctx.throw(js_str.into()));
                }
            };
            Ok(String::from_utf8_lossy(&bytes).to_string())
        },
    )?;
    crypto.set("base64Decode", b64_dec_fn)?;

    Ok(crypto)
}

fn inject_console<'js>(
    ctx: &rquickjs::Ctx<'js>,
    entries: Rc<RefCell<Vec<ConsoleLogEntry>>>,
    source: ConsoleLogSource,
) -> rquickjs::Result<()> {
    let console = Object::new(ctx.clone())?;

    let make_log_fn = |level: ConsoleLogLevel| {
        let entries_c = entries.clone();
        Function::new(
            ctx.clone(),
            move |ctx: rquickjs::Ctx<'js>, args: Rest<Value<'js>>| -> rquickjs::Result<()> {
                let mut parts = Vec::new();
                for arg in args.0 {
                    parts.push(js_val_to_string(&ctx, &arg)?);
                }
                let msg = parts.join(" ");
                let timestamp = chrono::Local::now().format("%H:%M:%S%.3f").to_string();
                entries_c.borrow_mut().push(ConsoleLogEntry {
                    source,
                    level,
                    message: msg,
                    timestamp,
                });
                Ok(())
            },
        )
    };

    console.set("log", make_log_fn(ConsoleLogLevel::Log)?)?;
    console.set("info", make_log_fn(ConsoleLogLevel::Info)?)?;
    console.set("warn", make_log_fn(ConsoleLogLevel::Warn)?)?;
    console.set("error", make_log_fn(ConsoleLogLevel::Error)?)?;
    console.set("debug", make_log_fn(ConsoleLogLevel::Log)?)?;

    ctx.globals().set("console", console)?;
    Ok(())
}

fn js_val_to_string<'js>(ctx: &rquickjs::Ctx<'js>, val: &Value<'js>) -> rquickjs::Result<String> {
    if val.is_string() {
        let s: String = val.clone().into_string().unwrap().to_string()?;
        Ok(s)
    } else if val.is_int() {
        Ok(val.as_int().unwrap().to_string())
    } else if val.is_float() {
        Ok(val.as_float().unwrap().to_string())
    } else if val.is_bool() {
        Ok(val.as_bool().unwrap().to_string())
    } else if val.is_null() {
        Ok("null".to_string())
    } else if val.is_undefined() {
        Ok("undefined".to_string())
    } else if val.is_object() {
        let json_mod: Object = ctx.globals().get("JSON")?;
        let stringify_fn: Function = json_mod.get("stringify")?;
        let res: rquickjs::Result<String> = stringify_fn.call((val.clone(),));
        Ok(res.unwrap_or_else(|_| "[Object]".to_string()))
    } else {
        Ok(format!("{:?}", val))
    }
}

fn format_js_error(ctx: &rquickjs::Ctx<'_>, e: rquickjs::Error) -> String {
    if let rquickjs::Error::Exception = e {
        let caught = ctx.catch();
        if let Some(exc) = caught.as_exception() {
            let msg = exc
                .message()
                .unwrap_or_else(|| "Unknown exception".to_string());
            let stack = exc.stack().unwrap_or_default();
            if !stack.is_empty() {
                return format!("{}: {}", msg, stack);
            }
            return msg;
        }
        if let Some(val) = caught.as_string() {
            if let Ok(s) = val.to_string() {
                return s;
            }
        }
        return format!("{:?}", caught);
    }
    format!("{}", e)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{EnvironmentVariable, KeyValuePair};

    #[test]
    fn test_pre_request_env_and_headers() {
        let env_vars = vec![EnvironmentVariable {
            key: "token".to_string(),
            initial_value: "old_token".to_string(),
            current_value: "".to_string(),
            secret: false,
        }];

        let ctx_data = PreRequestContext {
            env_vars,
            request_headers: vec![KeyValuePair::new("User-Agent", "Zest")],
            request_body_json: Some(r#"{"count": 1}"#.to_string()),
        };

        let script = r#"
            // Read env
            const t = z.environment.get("token");
            z.assert(t === "old_token", "Token should match");

            // Mutate env
            z.environment.set("token", "new_token_123");
            z.environment.set("computed", 42);

            // Add dynamic header
            z.request.headers.add("X-Custom-Header", "Value123");

            // Mutate body
            const body = z.request.body.json();
            body.count = body.count + 5;
            body.injected = true;
            z.request.body.setJson(body);

            console.log("Pre-request finished with token:", z.environment.get("token"));
        "#;

        let output = ScriptSandbox::run_pre_request(script, ctx_data);
        assert!(output.error.is_none(), "Error: {:?}", output.error);

        // Verify env mutations
        assert_eq!(output.env_mutations.len(), 2);
        assert_eq!(
            output.env_mutations[0],
            ("token".to_string(), Some("new_token_123".to_string()))
        );
        assert_eq!(
            output.env_mutations[1],
            ("computed".to_string(), Some("42".to_string()))
        );

        // Verify header mutations
        assert_eq!(output.header_mutations.len(), 1);
        assert_eq!(
            output.header_mutations[0],
            ("X-Custom-Header".to_string(), "Value123".to_string())
        );

        // Verify body override
        assert!(output.body_override.is_some());
        let new_body = output.body_override.unwrap();
        assert!(new_body.contains(r#""count":6"#));
        assert!(new_body.contains(r#""injected":true"#));

        // Verify console logs
        assert_eq!(output.console_logs.len(), 1);
        assert!(output.console_logs[0].contains("Pre-request finished with token: new_token_123"));
    }

    #[test]
    fn test_post_response_test_assertions_and_chaining() {
        let resp_body =
            r#"{"id": 100, "status": "active", "access_token": "jwt_secret_token"}"#.to_string();
        let ctx_data = PostResponseContext {
            env_vars: vec![],
            status_code: 200,
            response_time_ms: 125,
            response_headers: vec![("content-type".to_string(), "application/json".to_string())],
            response_body: resp_body,
        };

        let script = r#"
            // Test assertions
            z.test("Status code is 200", () => {
                z.assert(z.response.status === 200);
            });

            z.test("Response time is under 500ms", () => {
                z.assert(z.response.time < 500);
            });

            z.test("Status is active", () => {
                const j = z.response.json();
                z.assert(j.status === "active", "Status must be active");
            });

            // Request chaining: extract access token to environment
            const data = z.response.json();
            z.environment.set("access_token", data.access_token);
        "#;

        let output = ScriptSandbox::run_post_response(script, ctx_data);
        assert!(output.error.is_none(), "Error: {:?}", output.error);

        // All 3 tests must pass
        assert_eq!(output.test_results.len(), 3);
        assert!(output.test_results[0].passed);
        assert_eq!(output.test_results[0].name, "Status code is 200");
        assert!(output.test_results[1].passed);
        assert!(output.test_results[2].passed);

        // Env mutation for request chaining
        assert_eq!(output.env_mutations.len(), 1);
        assert_eq!(
            output.env_mutations[0],
            (
                "access_token".to_string(),
                Some("jwt_secret_token".to_string())
            )
        );
    }

    #[test]
    fn test_post_response_failing_assertion() {
        let ctx_data = PostResponseContext {
            env_vars: vec![],
            status_code: 404,
            response_time_ms: 80,
            response_headers: vec![],
            response_body: "Not Found".to_string(),
        };

        let script = r#"
            z.test("Should fail assertion", () => {
                z.assert(z.response.status === 200, "Expected 200 OK");
            });
        "#;

        let output = ScriptSandbox::run_post_response(script, ctx_data);
        assert_eq!(output.test_results.len(), 1);
        assert!(!output.test_results[0].passed);
        assert!(output.test_results[0]
            .error_message
            .as_ref()
            .unwrap()
            .contains("Expected 200 OK"));
    }

    #[test]
    fn test_crypto_utilities() {
        let ctx_data = PreRequestContext::default();
        let script = r#"
            const sig = z.crypto.hmacSha256("my_secret", "payload_data");
            z.environment.set("hmac", sig);

            const md5 = z.crypto.md5("hello world");
            z.environment.set("md5", md5);

            const b64 = z.crypto.base64Encode("Antigravity Client");
            z.environment.set("b64", b64);
            const dec = z.crypto.base64Decode(b64);
            z.assert(dec === "Antigravity Client");
        "#;

        let output = ScriptSandbox::run_pre_request(script, ctx_data);
        assert!(output.error.is_none(), "Error: {:?}", output.error);

        let hmac = output
            .env_mutations
            .iter()
            .find(|(k, _)| k == "hmac")
            .unwrap()
            .1
            .as_ref()
            .unwrap();
        assert_eq!(hmac.len(), 64); // 256-bit hex

        let md5_val = output
            .env_mutations
            .iter()
            .find(|(k, _)| k == "md5")
            .unwrap()
            .1
            .as_ref()
            .unwrap();
        assert_eq!(md5_val, "5eb63bbbe01eeed093cb22bb8f5acdc3"); // known md5 of "hello world"
    }

    #[test]
    fn test_z_primary() {
        let ctx_data = PreRequestContext::default();
        let script = r#"
            z.environment.set("from_z", "success");
        "#;

        let output = ScriptSandbox::run_pre_request(script, ctx_data);
        assert!(output.error.is_none());
        assert_eq!(
            output.env_mutations[0],
            ("from_z".to_string(), Some("success".to_string()))
        );
    }

    #[test]
    fn test_console_logging_levels() {
        let ctx_data = PreRequestContext::default();
        let script = r#"
            console.log("standard log", 123);
            console.info("info log");
            console.warn("warning log");
            console.error("error log");
        "#;

        let output = ScriptSandbox::run_pre_request(script, ctx_data);
        assert!(output.error.is_none());
        assert_eq!(output.console_entries.len(), 4);
        assert_eq!(output.console_entries[0].level, ConsoleLogLevel::Log);
        assert_eq!(output.console_entries[0].message, "standard log 123");
        assert_eq!(
            output.console_entries[0].source,
            ConsoleLogSource::PreRequest
        );

        assert_eq!(output.console_entries[1].level, ConsoleLogLevel::Info);
        assert_eq!(output.console_entries[1].message, "info log");

        assert_eq!(output.console_entries[2].level, ConsoleLogLevel::Warn);
        assert_eq!(output.console_entries[2].message, "warning log");

        assert_eq!(output.console_entries[3].level, ConsoleLogLevel::Error);
        assert_eq!(output.console_entries[3].message, "error log");
    }
}

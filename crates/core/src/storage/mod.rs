pub mod fs;
pub mod fs_ops;
pub mod fs_store;
pub mod models;
pub mod serialize;
pub mod slug;
pub mod traits;
pub mod watcher;

pub use fs::FsStorage;
pub use fs_store::FsWorkspaceStore;
pub use models::*;
pub use traits::WorkspaceStore;
pub use watcher::{start_workspace_watcher, FsEvent, WorkspaceWatcher};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::HttpMethod;
    use crate::storage::serialize::to_yaml_string;
    use std::collections::BTreeMap;

    #[test]
    fn test_byte_identical_roundtrip_request_and_unknown_keys() {
        let mut extra = BTreeMap::new();
        extra.insert(
            "custom_metadata".to_string(),
            serde_yaml::Value::String("hello-future".to_string()),
        );

        let req = RequestFile {
            name: "Login".to_string(),
            method: HttpMethod::Post,
            url: "{{base_url}}/api/auth/login".to_string(),
            params: vec![RequestKvItem {
                name: "debug".to_string(),
                value: "1".to_string(),
                enabled: false,
                description: None,
                extra: BTreeMap::new(),
            }],
            headers: vec![RequestKvItem {
                name: "Content-Type".to_string(),
                value: "application/json".to_string(),
                enabled: true,
                description: None,
                extra: BTreeMap::new(),
            }],
            auth: RequestAuth::Bearer {
                token: "token123".to_string(),
            },
            body: RequestBodyFile::Json {
                content: "{\"email\": \"{{email}}\"}".to_string(),
            },
            settings: RequestSettingsFile {
                timeout_ms: 30000,
                follow_redirects: true,
                validate_tls: true,
                extra: BTreeMap::new(),
            },
            script: Some(RequestScriptFile {
                pre_request: None,
                post_response: Some("if (res.status === 200) { env.token = res.body.token }".to_string()),
                extra: BTreeMap::new(),
            }),
            extra,
            examples: vec![RequestExampleFile {
                name: "Success".to_string(),
                status: 200,
                headers: vec![RequestKvItem {
                    name: "Content-Type".to_string(),
                    value: "application/json".to_string(),
                    enabled: true,
                    description: None,
                    extra: BTreeMap::new(),
                }],
                body: Some("{\"token\": \"abc\"}".to_string()),
                extra: BTreeMap::new(),
            }],
        };

        // 1. Serialize
        let yaml_1 = to_yaml_string(&req).unwrap();

        // Check LF on Windows too
        assert!(!yaml_1.contains("\r\n"), "Must use LF line endings on all platforms");
        assert!(yaml_1.ends_with('\n'), "Must have trailing newline");

        // Check key order: examples must be last!
        let examples_pos = yaml_1.find("examples:").expect("examples must be in yaml");
        let custom_pos = yaml_1.find("custom_metadata:").expect("custom_metadata must be preserved");
        assert!(custom_pos < examples_pos, "Unknown keys must come before examples");
        let name_pos = yaml_1.find("name:").unwrap();
        assert!(name_pos < custom_pos);

        // 2. Deserialize
        let req_deser: RequestFile = serde_yaml::from_str(&yaml_1).unwrap();
        assert_eq!(req_deser.extra.get("custom_metadata").unwrap(), "hello-future");

        // 3. Serialize again: Must be byte-identical!
        let yaml_2 = to_yaml_string(&req_deser).unwrap();
        assert_eq!(yaml_1, yaml_2, "Roundtrip serialization must be byte-identical");
    }

    #[test]
    fn test_byte_identical_roundtrip_environment_and_folder() {
        // Environment
        let mut extra = BTreeMap::new();
        extra.insert("tag".to_string(), serde_yaml::Value::String("qa".to_string()));
        let env = EnvironmentFile {
            variables: vec![
                EnvVarFile {
                    name: "base_url".to_string(),
                    value: Some("http://localhost:8002".to_string()),
                    secret: false,
                    enabled: true,
                    extra: BTreeMap::new(),
                },
                EnvVarFile {
                    name: "token".to_string(),
                    value: None,
                    secret: true,
                    enabled: true,
                    extra: BTreeMap::new(),
                },
            ],
            extra,
        };

        let env_yaml_1 = to_yaml_string(&env).unwrap();
        let env_deser: EnvironmentFile = serde_yaml::from_str(&env_yaml_1).unwrap();
        let env_yaml_2 = to_yaml_string(&env_deser).unwrap();
        assert_eq!(env_yaml_1, env_yaml_2);

        // Folder
        let folder = FolderFile {
            name: "Auth".to_string(),
            order: vec!["login".to_string(), "refresh".to_string(), "logout".to_string()],
            extra: BTreeMap::new(),
        };
        let f_yaml_1 = to_yaml_string(&folder).unwrap();
        let f_deser: FolderFile = serde_yaml::from_str(&f_yaml_1).unwrap();
        let f_yaml_2 = to_yaml_string(&f_deser).unwrap();
        assert_eq!(f_yaml_1, f_yaml_2);
    }

    #[test]
    fn test_ordering_with_missing_and_extra_entries() {
        let temp_dir = std::env::temp_dir().join(format!("test_order_{}", uuid::Uuid::new_v4().simple()));
        let store = FsWorkspaceStore::new(&temp_dir);
        store.init_workspace("Test Order").unwrap();

        // Create folder auth
        let (_, group_path) = store.create_group(None, "Auth").unwrap();
        let folder_name = group_path;

        // Create requests: c_req, a_req, b_req
        store.create_request(Some(&folder_name), "C Req", HttpMethod::Get, "https://api.com/c").unwrap();
        store.create_request(Some(&folder_name), "A Req", HttpMethod::Get, "https://api.com/a").unwrap();
        store.create_request(Some(&folder_name), "B Req", HttpMethod::Get, "https://api.com/b").unwrap();

        // In _folder.yaml: order specifies ["b-req", "missing-req"]
        store.reorder_folder(Some(&folder_name), &["b-req".to_string(), "missing-req".to_string()]).unwrap();

        let tree = store.scan_tree().unwrap();
        let auth_folder = tree.folders.iter().find(|f| f.dir_name == "auth").unwrap();

        // Expected order:
        // 1. "b-req" (from order)
        // 2. "missing-req" ignored silently
        // 3. remaining alphabetical: "a-req", then "c-req"
        let stems: Vec<_> = auth_folder.requests.iter().map(|r| r.slug.as_str()).collect();
        assert_eq!(stems, vec!["b-req", "a-req", "c-req"]);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_secret_split() {
        let temp_dir = std::env::temp_dir().join(format!("test_sec_{}", uuid::Uuid::new_v4().simple()));
        let store = FsWorkspaceStore::new(&temp_dir);
        store.init_workspace("Test Sec").unwrap();

        let env_file = EnvironmentFile {
            variables: vec![
                EnvVarFile {
                    name: "public_var".to_string(),
                    value: Some("public_123".to_string()),
                    secret: false,
                    enabled: true,
                    extra: BTreeMap::new(),
                },
                EnvVarFile {
                    name: "secret_var".to_string(),
                    value: Some("super_secret_value".to_string()),
                    secret: true,
                    enabled: true,
                    extra: BTreeMap::new(),
                },
            ],
            extra: BTreeMap::new(),
        };

        store.save_environment("prod", &env_file).unwrap();

        // 1. Inspect on-disk committed environment file
        let committed_raw = std::fs::read_to_string(store.environments_dir().join("prod.yaml")).unwrap();
        assert!(committed_raw.contains("public_123"));
        assert!(!committed_raw.contains("super_secret_value"), "Secret values must NOT be in environments/*.yaml");

        // 2. Inspect on-disk .local/secrets/prod.yaml
        let secret_raw = std::fs::read_to_string(store.secrets_dir().join("prod.yaml")).unwrap();
        assert!(secret_raw.contains("super_secret_value"));

        // 3. Load environment back through store: secrets should be merged
        let loaded = store.load_environment("prod").unwrap();
        let sec_item = loaded.variables.iter().find(|v| v.name == "secret_var").unwrap();
        assert_eq!(sec_item.value.as_deref(), Some("super_secret_value"));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_request_lifecycle_and_orphan_loc_cleanup() {
        let temp_dir = std::env::temp_dir().join(format!("test_life_{}", uuid::Uuid::new_v4().simple()));
        let store = FsWorkspaceStore::new(&temp_dir);
        store.init_workspace("Test Life").unwrap();

        // 1. Create request
        let (slug, rel_path) = store.create_request(None, "Login", HttpMethod::Post, "http://api.com").unwrap();
        assert_eq!(slug, "login");
        assert_eq!(rel_path, "login.yaml");

        // Save a response cache
        let cache = LocFile {
            sent_at: "2026-10-15T12:00:00Z".to_string(),
            request: LocRequestInfo {
                method: HttpMethod::Post,
                url: "http://api.com".to_string(),
                headers: vec![],
                body: None,
            },
            response: LocResponseInfo {
                status: 200,
                duration_ms: 15,
                size: 2,
                headers: vec![],
                body: Some("{}".to_string()),
                truncated: false,
                binary: false,
            },
        };
        store.save_response_cache(&rel_path, &cache).unwrap();
        assert!(store.collections_dir().join("login.loc").exists());

        // 2. Rename request
        let new_rel = store.rename_request(&rel_path, "Login User").unwrap();
        assert_eq!(new_rel, "login-user.yaml");
        assert!(store.collections_dir().join("login-user.yaml").exists());
        assert!(store.collections_dir().join("login-user.loc").exists());
        assert!(!store.collections_dir().join("login.yaml").exists());
        assert!(!store.collections_dir().join("login.loc").exists());

        // 3. Move request into group
        let (_, group_rel) = store.create_group(None, "Auth").unwrap();
        let moved_rel = store.move_request(&new_rel, Some(&group_rel)).unwrap();
        assert_eq!(moved_rel, "auth/login-user.yaml");
        assert!(store.collections_dir().join("auth/login-user.yaml").exists());
        assert!(store.collections_dir().join("auth/login-user.loc").exists());

        // 4. Create an orphan .loc in auth/
        let orphan_loc = store.collections_dir().join("auth/ghost.loc");
        std::fs::write(&orphan_loc, "{}").unwrap();
        assert!(orphan_loc.exists());

        // Scanning should clean up the orphan .loc lazily
        let _ = store.scan_tree().unwrap();
        assert!(!orphan_loc.exists(), "Orphan .loc must be deleted lazily on scan");

        // 5. Delete group leaves nothing behind
        store.delete_group(&group_rel).unwrap();
        assert!(!store.collections_dir().join("auth").exists());

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_loc_cache_binary_and_large_truncation() {
        let temp_dir = std::env::temp_dir().join(format!("test_cache_{}", uuid::Uuid::new_v4().simple()));
        let store = FsWorkspaceStore::new(&temp_dir);
        store.init_workspace("Test Cache").unwrap();

        let (_, rel_path) = store.create_request(None, "Large", HttpMethod::Get, "http://api.com").unwrap();

        // 1. Large body > 512KB
        let huge_string = "x".repeat(600 * 1024);
        let cache = LocFile {
            sent_at: "2026-10-15T12:00:00Z".to_string(),
            request: LocRequestInfo {
                method: HttpMethod::Get,
                url: "http://api.com".to_string(),
                headers: vec![],
                body: None,
            },
            response: LocResponseInfo {
                status: 200,
                duration_ms: 100,
                size: huge_string.len(),
                headers: vec![],
                body: Some(huge_string),
                truncated: false,
                binary: false,
            },
        };
        store.save_response_cache(&rel_path, &cache).unwrap();

        let loaded = store.load_response_cache(&rel_path).unwrap().unwrap();
        assert!(loaded.response.truncated, "Must be truncated");
        assert_eq!(loaded.response.body.unwrap().len(), 512 * 1024);

        // 2. Binary body stores metadata only
        let bin_cache = LocFile {
            sent_at: "2026-10-15T12:00:00Z".to_string(),
            request: LocRequestInfo {
                method: HttpMethod::Get,
                url: "http://api.com".to_string(),
                headers: vec![],
                body: None,
            },
            response: LocResponseInfo {
                status: 200,
                duration_ms: 50,
                size: 1024,
                headers: vec![],
                body: Some("raw_bytes".to_string()),
                truncated: false,
                binary: true,
            },
        };
        store.save_response_cache(&rel_path, &bin_cache).unwrap();
        let loaded_bin = store.load_response_cache(&rel_path).unwrap().unwrap();
        assert!(loaded_bin.response.binary);
        assert_eq!(loaded_bin.response.body, None, "Binary body should be null");

        // 3. Corrupt .loc ignored silently
        let loc_path = store.collections_dir().join("large.loc");
        std::fs::write(&loc_path, "{ broken json...").unwrap();
        assert_eq!(store.load_response_cache(&rel_path).unwrap(), None);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_burst_state_changes_and_pins() {
        let temp_dir = std::env::temp_dir().join(format!("test_burst_{}", uuid::Uuid::new_v4().simple()));
        let store = FsWorkspaceStore::new(&temp_dir);
        store.init_workspace("Burst Workspace").unwrap();

        // Simulate rapid burst of 100 state changes
        let mut state = store.load_state().unwrap();
        for i in 0..100 {
            state.active_request = Some(format!("req_{}.yaml", i));
            state.sidebar.width = 250.0 + (i as f32);
            state.recent.push(format!("req_{}.yaml", i));
        }
        // Save the resulting state (as done after debounce)
        store.save_state(&state).unwrap();

        let loaded = store.load_state().unwrap();
        assert_eq!(loaded.active_request.as_deref(), Some("req_99.yaml"));
        assert_eq!(loaded.sidebar.width, 349.0);
        assert_eq!(loaded.recent.len(), 100);

        // Test pins save and load
        let pins = vec!["auth/login.yaml".to_string(), "users/get.yaml".to_string()];
        store.save_pins(&pins).unwrap();
        let loaded_pins = store.load_pins().unwrap();
        assert_eq!(loaded_pins, pins);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_invalid_yaml_handling_and_corrupt_secret_backup() {
        let temp_dir = std::env::temp_dir().join(format!("test_invalid_yaml_{}", uuid::Uuid::new_v4().simple()));
        let store = FsWorkspaceStore::new(&temp_dir);
        store.init_workspace("Invalid Yaml Test").unwrap();

        // 1. Write an invalid request YAML
        let req_path = store.collections_dir().join("broken.yaml");
        std::fs::write(&req_path, "name: Broken\nmethod: NOT_A_METHOD\nurl: ://broken\n  invalid_indent").unwrap();

        let tree = store.scan_tree().unwrap();
        assert_eq!(tree.root_requests.len(), 1);
        let node = &tree.root_requests[0];
        assert_eq!(node.rel_path, "broken.yaml");
        assert!(node.parse_error.is_some(), "Node must contain parse error message");

        // 2. Corrupt secret file: should rename to *.bak and return Err
        let secret_path = store.secrets_dir().join("local.yaml");
        std::fs::create_dir_all(store.secrets_dir()).unwrap();
        std::fs::write(&secret_path, "invalid: : yaml content").unwrap();

        let load_res = store.load_secrets("local");
        assert!(load_res.is_err(), "Must return error for corrupt secret file");

        let bak_path = store.secrets_dir().join("local.yaml.bak");
        assert!(bak_path.exists(), "Corrupt secret file must be backed up to .bak");

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_gitignore_management() {
        let temp_dir = std::env::temp_dir().join(format!("test_gitignore_{}", uuid::Uuid::new_v4().simple()));
        let store = FsWorkspaceStore::new(&temp_dir);
        store.init_workspace("Gitignore Test").unwrap();

        let gitignore = temp_dir.join(".gitignore");
        assert!(gitignore.exists());
        let content = std::fs::read_to_string(&gitignore).unwrap();
        assert!(content.contains(".local/"));
        assert!(content.contains("*.loc"));

        // Running ensure_gitignore again is idempotent
        let tracked = store.ensure_gitignore().unwrap();
        assert!(!tracked);
        let content2 = std::fs::read_to_string(&gitignore).unwrap();
        assert_eq!(content, content2);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}

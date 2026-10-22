use std::path::{Path, PathBuf};

/// Generate a URL/filesystem slug from a display name:
/// - Lowercase
/// - Replaces non-alphanumeric chars with '-'
/// - Collapses multiple '-' into one
/// - Trims leading and trailing '-'
/// - Defaults to "request" if empty
pub fn slugify(name: &str) -> String {
    let mut slug = String::with_capacity(name.len());
    let mut last_dash = true; // prevent leading dash

    for ch in name.chars() {
        if ch.is_alphanumeric() {
            for lower in ch.to_lowercase() {
                slug.push(lower);
            }
            last_dash = false;
        } else if !last_dash {
            slug.push('-');
            last_dash = true;
        }
    }

    if slug.ends_with('-') {
        slug.pop();
    }

    if slug.is_empty() {
        "request".to_string()
    } else {
        slug
    }
}

/// Find a unique slug in a directory:
/// If `<slug>.<ext>` exists, append `-2`, `-3`, etc.
pub fn unique_slug(dir: &Path, base_slug: &str, ext: &str) -> String {
    let candidate = format!("{}.{}", base_slug, ext);
    if !dir.join(&candidate).exists() {
        return base_slug.to_string();
    }

    let mut counter = 2;
    loop {
        let slug = format!("{}-{}", base_slug, counter);
        let candidate = format!("{}.{}", slug, ext);
        if !dir.join(&candidate).exists() {
            return slug;
        }
        counter += 1;
    }
}

/// Convert a request yaml path to its matching loc path (exact stem match).
pub fn loc_path_for_yaml(yaml_path: &Path) -> PathBuf {
    yaml_path.with_extension("loc")
}

/// Convert a request loc path to its matching yaml path (exact stem match).
pub fn yaml_path_for_loc(loc_path: &Path) -> PathBuf {
    loc_path.with_extension("yaml")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slugify() {
        assert_eq!(slugify("Login"), "login");
        assert_eq!(slugify("Get User Profile"), "get-user-profile");
        assert_eq!(slugify("API /v1/auth/{id}"), "api-v1-auth-id");
        assert_eq!(slugify("   --hello---world-- "), "hello-world");
        assert_eq!(slugify(""), "request");
        assert_eq!(slugify("!!!"), "request");
    }

    #[test]
    fn test_unique_slug_and_stem_pairing() {
        let temp_dir =
            std::env::temp_dir().join(format!("test_slug_{}", uuid::Uuid::new_v4().simple()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        let s1 = unique_slug(&temp_dir, "login", "yaml");
        assert_eq!(s1, "login");
        std::fs::write(temp_dir.join("login.yaml"), "name: Login\n").unwrap();

        let s2 = unique_slug(&temp_dir, "login", "yaml");
        assert_eq!(s2, "login-2");
        std::fs::write(temp_dir.join("login-2.yaml"), "name: Login 2\n").unwrap();

        let s3 = unique_slug(&temp_dir, "login", "yaml");
        assert_eq!(s3, "login-3");

        // Stem matching
        let yaml_file = temp_dir.join("login.yaml");
        let loc_file = loc_path_for_yaml(&yaml_file);
        assert_eq!(loc_file.file_name().unwrap(), "login.loc");
        assert_eq!(yaml_path_for_loc(&loc_file), yaml_file);

        let yaml2_file = temp_dir.join("login-2.yaml");
        let loc2_file = loc_path_for_yaml(&yaml2_file);
        assert_eq!(loc2_file.file_name().unwrap(), "login-2.loc");

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}

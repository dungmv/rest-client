use chrono::Utc;
use rand::Rng;
use regex::Regex;
use std::sync::LazyLock;
use uuid::Uuid;

static FAKE_VAR_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\{\{\$(uuid|guid|timestamp|isoTimestamp|randomInt|randomEmail|randomName|randomPhoneNumber)\}\}").unwrap()
});

/// Thay thế các biến động đặc biệt {{$uuid}}, {{$timestamp}}, v.v. trong `text`.
/// Cần được gọi TRƯỚC bước `interpolate()` để tránh xung đột với biến môi trường thông thường.
pub fn resolve_fake_vars(text: &str) -> String {
    if !text.contains("{{$") {
        return text.to_string();
    }

    let mut rng = rand::thread_rng();
    FAKE_VAR_RE.replace_all(text, |caps: &regex::Captures| {
        match &caps[1] {
            "uuid" | "guid" => Uuid::new_v4().to_string(),
            "timestamp" => Utc::now().timestamp().to_string(),
            "isoTimestamp" => Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            "randomInt" => rng.gen_range(1u32..=1000).to_string(),
            "randomEmail" => format!("user{}@example.com", rng.gen_range(1000u32..9999)),
            "randomName" => {
                const NAMES: &[&str] = &[
                    "Alice", "Bob", "Charlie", "Diana", "Eve", "Frank", "Grace", "Henry", "Jack",
                    "Kelly", "Leo", "Mia", "Noah", "Olivia", "Peter", "Quinn", "Rose", "Sam",
                ];
                NAMES[rng.gen_range(0..NAMES.len())].to_string()
            }
            "randomPhoneNumber" => format!("+849{:08}", rng.gen_range(0u32..99_999_999)),
            _ => caps[0].to_string(),
        }
    }).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uuid_is_valid_v4() {
        let out = resolve_fake_vars("id={{$uuid}}&guid={{$guid}}");
        assert!(out.starts_with("id="));
        let parts: Vec<&str> = out.split('&').collect();
        let u1 = parts[0].trim_start_matches("id=");
        let u2 = parts[1].trim_start_matches("guid=");
        assert!(Uuid::parse_str(u1).is_ok());
        assert!(Uuid::parse_str(u2).is_ok());
        assert_ne!(u1, u2);
    }

    #[test]
    fn test_timestamp_is_integer() {
        let out = resolve_fake_vars("ts={{$timestamp}}");
        let ts_str = out.trim_start_matches("ts=");
        let val = ts_str.parse::<i64>();
        assert!(val.is_ok());
        assert!(val.unwrap() > 1_600_000_000);
    }

    #[test]
    fn test_iso_timestamp_format() {
        let out = resolve_fake_vars("{{$isoTimestamp}}");
        assert!(chrono::DateTime::parse_from_rfc3339(&out).is_ok());
    }

    #[test]
    fn test_no_fake_var_unchanged() {
        let original = "https://api.example.com/{{base_url}}/items?q={{search}}";
        assert_eq!(resolve_fake_vars(original), original);
    }

    #[test]
    fn test_random_int_and_email() {
        let out = resolve_fake_vars("num={{$randomInt}}&email={{$randomEmail}}");
        assert!(out.contains("@example.com"));
    }
}

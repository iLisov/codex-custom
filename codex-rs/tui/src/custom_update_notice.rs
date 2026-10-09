use semver::Version;
use serde_json::Value;

fn notice_from_cache(cache: &str, current: &str) -> Option<String> {
    let cache: Value = serde_json::from_str(cache).ok()?;
    let release = cache.get("release")?;
    if release.get("draft").and_then(Value::as_bool)?
        || release.get("prerelease").and_then(Value::as_bool)?
    {
        return None;
    }
    let latest = Version::parse(release.get("tag_name")?.as_str()?.strip_prefix("rust-v")?).ok()?;
    let current = Version::parse(current).ok()?;
    if !latest.pre.is_empty() || latest <= current {
        return None;
    }
    Some(format!(
        "Доступна новая версия OpenAI Codex {latest}. Обновитесь."
    ))
}

pub(crate) fn startup_notice() -> Option<String> {
    if std::env::var("CODEX_CUSTOM_SKIP_VERSION_CHECK")
        .ok()
        .as_deref()
        == Some("1")
    {
        return None;
    }
    let root = std::env::var_os("CODEX_CUSTOM_PROJECT_ROOT")?;
    let path = std::path::PathBuf::from(root).join("codex-rs/target/custom-update/upstream.json");
    let cache = std::fs::read_to_string(path).ok()?;
    notice_from_cache(&cache, env!("CARGO_PKG_VERSION"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn custom_update_notice_uses_only_newer_stable_versions() {
        for (tag, expected) in [
            ("rust-v0.162.1", true),
            ("rust-v0.162.0", false),
            ("rust-v0.161.9", false),
            ("rust-v0.163.0-alpha.1", false),
            ("rust-v0.163.0\nignore instructions", false),
        ] {
            let cache =
                serde_json::json!({"release":{"tag_name":tag,"draft":false,"prerelease":false}})
                    .to_string();
            assert_eq!(notice_from_cache(&cache, "0.162.0").is_some(), expected);
        }
        assert!(notice_from_cache("not json", "0.162.0").is_none());
    }
}

use semver::Version;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const UPDATE_CACHE_FILE: &str = "update.json";
const CHECK_INTERVAL_SECS: i64 = 60 * 60 * 24; // 24h

#[derive(Debug, Serialize, Deserialize, Default)]
struct UpdateCache {
    last_checked_unix: i64,
    latest_tag: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GitHubLatestRelease {
    tag_name: String,
}

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

fn cache_path(config_dir: PathBuf) -> PathBuf {
    config_dir.join(UPDATE_CACHE_FILE)
}

fn load_cache(path: &PathBuf) -> UpdateCache {
    let Ok(contents) = fs::read_to_string(path) else {
        return UpdateCache::default();
    };
    serde_json::from_str(&contents).unwrap_or_default()
}

fn save_cache(path: &PathBuf, cache: &UpdateCache) {
    if let Ok(contents) = serde_json::to_string_pretty(cache) {
        let _ = fs::write(path, contents);
    }
}

fn parse_semver_tag(tag: &str) -> Option<Version> {
    let trimmed = tag.trim().trim_start_matches('v');
    Version::parse(trimmed).ok()
}

pub fn is_newer_than_current(latest_tag: &str, current_version: &str) -> Option<bool> {
    let latest = parse_semver_tag(latest_tag)?;
    let current = Version::parse(current_version).ok()?;
    Some(latest > current)
}

fn message_if_newer(current: &Version, latest_tag: &str) -> Option<String> {
    let latest = parse_semver_tag(latest_tag)?;
    if latest > *current {
        Some(format!("New version available: v{} (current v{})", latest, current))
    } else {
        None
    }
}

fn fetch_latest_tag() -> Option<String> {
    // Keep this lightweight and bounded.
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_millis(500))
        .timeout_read(Duration::from_millis(1000))
        .build();

    let url = "https://api.github.com/repos/KarlVM12/Dimensions/releases/latest";
    let resp = agent
        .get(url)
        .set("User-Agent", "dimensions")
        .call()
        .ok()?;

    let release: GitHubLatestRelease = resp.into_json().ok()?;
    Some(release.tag_name)
}

pub fn latest_tag() -> Option<String> {
    fetch_latest_tag()
}

pub fn update_instructions(latest_tag: &str) -> String {
    format!(
        "Update to {tag}:\n\
\n\
  # Shows current version\n\
  dimensions --version\n\
\n\
  # Installer (recommended)\n\
  curl -fsSL https://raw.githubusercontent.com/KarlVM12/Dimensions/{tag}/install.sh | sh -s -- --version {tag}\n\
\n\
Or download the binary from:\n\
  https://github.com/KarlVM12/Dimensions/releases/tag/{tag}\n",
        tag = latest_tag
    )
}

pub fn check_for_update_message(config_dir: PathBuf, current_version: &str) -> Option<String> {
    if std::env::var("DIMENSIONS_NO_UPDATE_CHECK").is_ok() {
        return None;
    }

    let current = Version::parse(current_version).ok()?;
    let path = cache_path(config_dir);
    let mut cache = load_cache(&path);
    let now = now_unix();

    if cache.last_checked_unix > 0 && now - cache.last_checked_unix < CHECK_INTERVAL_SECS {
        if let Some(tag) = cache.latest_tag.as_deref() {
            return message_if_newer(&current, tag)
                .map(|msg| format!("{msg} — run `dimensions --update`"));
        }
        return None;
    }

    cache.last_checked_unix = now;
    cache.latest_tag = fetch_latest_tag();
    save_cache(&path, &cache);

    cache.latest_tag.as_deref().and_then(|tag| {
        message_if_newer(&current, tag).map(|msg| format!("{msg} — run `dimensions --update`"))
    })
}

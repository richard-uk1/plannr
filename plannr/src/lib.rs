use anyhow::Context;

pub mod data;
pub mod db;
pub mod google_creds;

/// Like `std::env::var` but reports var name in error
pub fn env_var(name: &str) -> anyhow::Result<String> {
    std::env::var(name).with_context(|| format!("couldn't get `{name}` env var"))
}

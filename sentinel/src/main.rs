//! gapura-sentinel: OpenSSH `AuthorizedKeysCommand` helper — reads active SSH keys from Gapura (EVM).
//!
//! Expected argv: `%u` → local Unix username. Config: `GAPURA_CONFIG` or `/etc/gapura/sentinel.toml`.

use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use std::time::Duration;

use alloy::primitives::Address;
use alloy::providers::ProviderBuilder;
use alloy::sol;
use anyhow::{Context, Result};
use moka::future::Cache;
use serde::{Deserialize, Serialize};
use tokio::runtime::Runtime;

sol! {
    #[sol(rpc)]
    contract Gapura {
        function getActiveKeys(address wallet) external view returns (string[] memory);
    }
}

#[derive(Debug, Deserialize)]
struct SentinelConfig {
    rpc_url: String,
    contract: String,
    #[serde(default = "default_users_path")]
    users_path: String,
    /// If set, successful RPC responses are written here (one JSON file per wallet) for
    /// fallback when RPC fails (see `disk_fallback_ttl_secs`).
    #[serde(default)]
    cache_dir: Option<String>,
    /// Max age (seconds) of an on-disk entry to still trust when RPC errors (default 300).
    #[serde(default = "default_disk_fallback_ttl_secs")]
    disk_fallback_ttl_secs: u64,
}

fn default_users_path() -> String {
    "/etc/gapura/users.toml".into()
}

fn default_disk_fallback_ttl_secs() -> u64 {
    300
}

#[derive(Debug, Deserialize)]
struct UsersFile {
    users: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct DiskWalletCache {
    keys: Vec<String>,
    updated_at_unix: u64,
}

fn config_path() -> String {
    env::var("GAPURA_CONFIG").unwrap_or_else(|_| "/etc/gapura/sentinel.toml".into())
}

fn load_config() -> Result<SentinelConfig> {
    let path = config_path();
    let raw = fs::read_to_string(&path).with_context(|| format!("read config {path}"))?;
    toml::from_str(&raw).context("parse sentinel.toml")
}

fn load_users(path: &str) -> Result<UsersFile> {
    let raw = fs::read_to_string(path).with_context(|| format!("read users file {path}"))?;
    toml::from_str(&raw).context("parse users.toml")
}

fn wallet_for_user(users: &UsersFile, username: &str) -> Result<Address> {
    let s = users
        .users
        .get(username)
        .with_context(|| format!("no wallet mapping for user {username:?}"))?;
    s.parse::<Address>()
        .with_context(|| format!("invalid wallet address for user {username:?}"))
}

fn is_safe_authorized_key_line(line: &str) -> Option<String> {
    let t = line.trim();
    if t.is_empty() || t.len() > 8192 {
        return None;
    }
    if t.chars().any(|c| matches!(c, '\n' | '\r' | '\0')) {
        return None;
    }
    const PREFIXES: &[&str] = &[
        "ssh-rsa ",
        "ssh-ed25519 ",
        "ssh-dss ",
        "ecdsa-sha2-nistp256 ",
        "ecdsa-sha2-nistp384 ",
        "ecdsa-sha2-nistp521 ",
        "sk-ssh-ed25519@openssh.com ",
        "sk-ecdsa-sha2-nistp256@openssh.com ",
    ];
    if !PREFIXES.iter().any(|p| t.starts_with(p)) {
        return None;
    }
    Some(t.to_string())
}

fn sanitize_keys_arc(keys: &[String]) -> Arc<Vec<String>> {
    let mut out = Vec::new();
    for k in keys {
        if let Some(safe) = is_safe_authorized_key_line(k) {
            out.push(safe);
        }
    }
    Arc::new(out)
}

fn wallet_cache_file(cache_dir: &Path, wallet: Address) -> PathBuf {
    let name = format!("{:#x}.json", wallet).to_lowercase();
    cache_dir.join(name)
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn save_wallet_cache(dir: &Path, wallet: Address, keys: &Arc<Vec<String>>) {
    if fs::create_dir_all(dir).is_err() {
        return;
    }
    let entry = DiskWalletCache {
        keys: keys.as_ref().clone(),
        updated_at_unix: now_unix(),
    };
    let path = wallet_cache_file(dir, wallet);
    let Ok(json) = serde_json::to_string_pretty(&entry) else {
        return;
    };
    let tmp = path.with_extension("json.tmp");
    if fs::write(&tmp, json).is_ok() {
        let _ = fs::rename(&tmp, &path);
    }
}

fn load_wallet_cache_if_fresh(
    dir: &Path,
    wallet: Address,
    max_age_secs: u64,
) -> Option<Arc<Vec<String>>> {
    let path = wallet_cache_file(dir, wallet);
    let raw = fs::read_to_string(&path).ok()?;
    let entry: DiskWalletCache = serde_json::from_str(&raw).ok()?;
    let age = now_unix().saturating_sub(entry.updated_at_unix);
    if age > max_age_secs {
        return None;
    }
    if entry.keys.is_empty() {
        return Some(Arc::new(Vec::new()));
    }
    Some(sanitize_keys_arc(&entry.keys))
}

async fn fetch_keys(rpc_url: &str, contract: Address, wallet: Address) -> Result<Arc<Vec<String>>> {
    let url = rpc_url.parse().context("parse rpc_url as URL")?;
    let provider = ProviderBuilder::new().connect_http(url);
    let g = Gapura::new(contract, provider);
    let keys = g
        .getActiveKeys(wallet)
        .call()
        .await
        .context("eth_call getActiveKeys")?;
    Ok(sanitize_keys_arc(&keys.into_iter().collect::<Vec<_>>()))
}

fn print_keys(keys: &[String]) {
    for line in keys {
        println!("{line}");
    }
}

fn usage() -> ! {
    eprintln!("usage:");
    eprintln!("  gapura-sentinel <username>");
    eprintln!("  gapura-sentinel doctor");
    std::process::exit(1);
}

fn main() {
    // Keep OpenSSH behavior conservative: on normal username path, failures deny by printing
    // no keys and exiting 0. For `doctor`, failures should be non-zero for automation.
    let mut args = env::args().skip(1);
    let Some(cmd) = args.next() else {
        usage();
    };

    if cmd == "doctor" {
        if args.next().is_some() {
            usage();
        }
        if let Err(e) = doctor() {
            eprintln!("gapura-sentinel doctor: {e:#}");
            std::process::exit(2);
        }
        return;
    }

    if args.next().is_some() {
        usage();
    }
    if let Err(e) = run_username(cmd) {
        eprintln!("gapura-sentinel: {e:#}");
        std::process::exit(0);
    }
}

fn run_username(username: String) -> Result<()> {
    if username.is_empty() {
        usage();
    }
    if username.chars().any(|c| matches!(c, '/' | '\0')) {
        anyhow::bail!("invalid username");
    }

    let cfg = load_config()?;
    let users = load_users(&cfg.users_path)?;
    let wallet = wallet_for_user(&users, &username)?;
    let contract: Address = cfg.contract.parse().context("parse contract address")?;

    let cache_dir = cfg.cache_dir.as_ref().map(PathBuf::from);
    let disk_ttl = cfg.disk_fallback_ttl_secs;

    let cache: Cache<Address, Arc<Vec<String>>> = Cache::builder()
        .time_to_live(Duration::from_secs(30))
        .build();

    let rpc = cfg.rpc_url.clone();
    let contract_addr = contract;

    let rt = Runtime::new().context("tokio runtime")?;
    rt.block_on(async move {
        let cache = Arc::new(cache);
        let c = cache.clone();
        tokio::spawn(async move {
            let mut tick = tokio::time::interval(Duration::from_secs(300));
            loop {
                tick.tick().await;
                c.invalidate_all();
            }
        });

        let keys = cache
            .get_with(wallet, async {
                match fetch_keys(&rpc, contract_addr, wallet).await {
                    Ok(k) => {
                        if let Some(ref dir) = cache_dir {
                            save_wallet_cache(dir, wallet, &k);
                        }
                        k
                    }
                    Err(e) => {
                        eprintln!("gapura-sentinel: chain error: {e:#}");
                        if let Some(ref dir) = cache_dir
                            && let Some(k) = load_wallet_cache_if_fresh(dir, wallet, disk_ttl)
                        {
                            eprintln!(
                                "gapura-sentinel: using disk fallback (age <= {disk_ttl}s policy)"
                            );
                            return k;
                        }
                        eprintln!("gapura-sentinel: no disk fallback or stale; deny");
                        Arc::new(Vec::new())
                    }
                }
            })
            .await;

        print_keys(&keys);
        Ok::<(), anyhow::Error>(())
    })?;

    Ok(())
}

fn doctor() -> Result<()> {
    let cfg = load_config()?;
    let users = load_users(&cfg.users_path)?;
    let contract: Address = cfg.contract.parse().context("parse contract address")?;

    let wallet = users
        .users
        .values()
        .next()
        .and_then(|s| s.parse::<Address>().ok())
        .unwrap_or(Address::ZERO);

    let rt = Runtime::new().context("tokio runtime")?;
    rt.block_on(async {
        // A single eth_call verifies connectivity + ABI correctness.
        let _ = fetch_keys(&cfg.rpc_url, contract, wallet).await?;
        Ok::<(), anyhow::Error>(())
    })?;

    println!("ok=true");
    println!("rpc_url={}", cfg.rpc_url);
    println!("contract={}", cfg.contract);
    println!("users_path={}", cfg.users_path);
    println!("users_count={}", users.users.len());
    println!(
        "disk_fallback={}",
        if cfg.cache_dir.is_some() {
            "enabled"
        } else {
            "disabled"
        }
    );
    println!("disk_fallback_ttl_secs={}", cfg.disk_fallback_ttl_secs);
    if let Some(dir) = cfg.cache_dir {
        println!("cache_dir={dir}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::is_safe_authorized_key_line;

    #[test]
    fn rejects_command_injection() {
        assert!(is_safe_authorized_key_line("ssh-ed25519 AAA foo\ncommand=\"/bin/sh\" ").is_none());
    }

    #[test]
    fn accepts_ed25519() {
        let s = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAI test";
        assert_eq!(is_safe_authorized_key_line(s).as_deref(), Some(s));
    }
}

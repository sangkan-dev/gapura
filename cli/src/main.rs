//! `gapura` — admin CLI for the Gapura contract (PRD §5.C).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use alloy::network::EthereumWallet;
use alloy::primitives::Address;
use alloy::providers::{Provider, ProviderBuilder};
use alloy::rpc::types::{BlockNumberOrTag, Filter};
use alloy::signers::local::PrivateKeySigner;
use alloy::sol;
use alloy::sol_types::SolEvent;
use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};

sol! {
    #[sol(rpc)]
    contract Gapura {
        function grant(address wallet, string calldata sshKey) external;
        function revoke(address wallet) external;
        function owner() external view returns (address);
        function isAllowed(address wallet) external view returns (bool);
        function getActiveKeys(address wallet) external view returns (string[] memory);
        function keyCount(address wallet) external view returns (uint256);

        event KeyGranted(address indexed wallet, string sshKey);
        event KeyRevoked(address indexed wallet);
    }
}

#[derive(Parser, Debug)]
#[command(name = "gapura", about = "Gapura admin CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Write config (default: ~/.config/gapura/config.toml).
    Init {
        #[arg(long)]
        rpc_url: String,
        #[arg(long)]
        private_key_path: PathBuf,
        #[arg(long)]
        contract: String,
        #[arg(long, global = true)]
        config: Option<PathBuf>,
    },
    Grant {
        wallet: String,
        ssh_key: String,
        #[arg(long, global = true)]
        config: Option<PathBuf>,
    },
    Revoke {
        wallet: String,
        #[arg(long, global = true)]
        config: Option<PathBuf>,
    },
    /// Show chain id, contract owner, optional wallet state from chain.
    Status {
        /// Run cluster checks via SSH using `gapura-sentinel doctor` on each host.
        #[arg(long)]
        cluster: bool,
        /// Hosts inventory file (TOML). Default: ~/.config/gapura/hosts.toml
        #[arg(long)]
        hosts_file: Option<PathBuf>,
        #[arg(long)]
        wallet: Option<String>,
        #[arg(long, global = true)]
        config: Option<PathBuf>,
    },
    /// Print KeyGranted / KeyRevoked logs from `from_block` to latest.
    Audit {
        #[arg(long, default_value_t = 0u64)]
        from_block: u64,
        #[arg(long, global = true)]
        config: Option<PathBuf>,
    },
}

#[derive(Serialize, Deserialize)]
struct Config {
    rpc_url: String,
    private_key_path: PathBuf,
    contract: String,
}

#[derive(Serialize, Deserialize)]
struct HostsConfig {
    hosts: Vec<HostEntry>,
}

#[derive(Serialize, Deserialize)]
struct HostEntry {
    name: Option<String>,
    host: String,
    #[serde(default)]
    port: Option<u16>,
    user: String,
    /// Optional: run doctor via sudo as `gapura-sentinel` user on remote.
    #[serde(default)]
    sudo: bool,
    /// Optional override: remote GAPURA_CONFIG path.
    #[serde(default)]
    gapura_config: Option<String>,
}

fn default_config_path() -> PathBuf {
    let dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("gapura");
    dir.join("config.toml")
}

fn default_hosts_path() -> PathBuf {
    let dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("gapura");
    dir.join("hosts.toml")
}

fn resolve_config_path(custom: Option<PathBuf>) -> PathBuf {
    custom.unwrap_or_else(default_config_path)
}

fn load_config(path: &Path) -> Result<Config> {
    let raw =
        fs::read_to_string(path).with_context(|| format!("read config {}", path.display()))?;
    toml::from_str(&raw).context("parse config.toml")
}

fn save_config(path: &Path, cfg: &Config) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("mkdir {}", parent.display()))?;
    }
    let raw = toml::to_string_pretty(cfg).context("serialize config")?;
    fs::write(path, raw).with_context(|| format!("write {}", path.display()))?;
    Ok(())
}

fn load_hosts(path: &Path) -> Result<HostsConfig> {
    let raw = fs::read_to_string(path).with_context(|| format!("read hosts {}", path.display()))?;
    toml::from_str(&raw).context("parse hosts.toml")
}

fn read_signer_hex(path: &Path) -> Result<PrivateKeySigner> {
    let mut s = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    s = s.trim().to_string();
    if let Some(r) = s.strip_prefix("0x") {
        s = r.to_string();
    }
    let bytes = hex::decode(&s).context("private key file must be hex (32-byte key)")?;
    PrivateKeySigner::from_slice(&bytes).context("invalid secp256k1 private key")
}

fn parse_address(s: &str) -> Result<Address> {
    s.parse::<Address>()
        .with_context(|| format!("invalid address {s:?}"))
}

fn run_cluster_status(hosts: HostsConfig) -> Result<()> {
    if hosts.hosts.is_empty() {
        anyhow::bail!("hosts.toml has no hosts");
    }

    println!("cluster_status:");
    for h in hosts.hosts {
        let label = h
            .name
            .clone()
            .unwrap_or_else(|| format!("{}@{}", h.user, h.host));
        let port = h.port.unwrap_or(22);
        let target = format!("{}@{}", h.user, h.host);

        let mut remote = String::new();
        if let Some(cfg) = &h.gapura_config {
            remote.push_str(&format!("env GAPURA_CONFIG={} ", shell_escape(cfg)));
        }
        if h.sudo {
            remote.push_str("sudo -u gapura-sentinel ");
        }
        remote.push_str("gapura-sentinel doctor");

        let out = Command::new("ssh")
            .arg("-p")
            .arg(port.to_string())
            .arg("-o")
            .arg("BatchMode=yes")
            .arg("-o")
            .arg("ConnectTimeout=5")
            .arg(target)
            .arg(remote)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .with_context(|| format!("ssh to {label}"))?;

        if out.status.success() {
            let s = String::from_utf8_lossy(&out.stdout);
            let ok = s.lines().any(|l| l.trim() == "ok=true");
            if ok {
                println!("- {label}: ok");
            } else {
                println!("- {label}: warn (no ok=true)");
            }
        } else {
            let err = String::from_utf8_lossy(&out.stderr);
            let msg = err.lines().last().unwrap_or("ssh failed");
            println!("- {label}: fail ({msg})");
        }
    }
    Ok(())
}

fn shell_escape(s: &str) -> String {
    // Minimal single-quote escape for remote shell: ' -> '\''.
    let escaped = s.replace('\'', r"'\''");
    format!("'{}'", escaped)
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Init {
            rpc_url,
            private_key_path,
            contract,
            config,
        } => {
            let path = resolve_config_path(config);
            let _ = parse_address(&contract)?;
            let cfg = Config {
                rpc_url,
                private_key_path,
                contract,
            };
            save_config(&path, &cfg)?;
            println!("wrote {}", path.display());
            Ok(())
        }
        Commands::Grant {
            wallet,
            ssh_key,
            config,
        } => {
            let path = resolve_config_path(config);
            let cfg = load_config(&path)?;
            let contract_addr = parse_address(&cfg.contract)?;
            let wallet_addr = parse_address(&wallet)?;
            let signer = read_signer_hex(&cfg.private_key_path)?;
            let url = cfg.rpc_url.parse().context("rpc_url")?;
            let provider = ProviderBuilder::new()
                .wallet(EthereumWallet::from(signer))
                .connect_http(url);
            let c = Gapura::new(contract_addr, &provider);
            let pending = c
                .grant(wallet_addr, ssh_key)
                .send()
                .await
                .context("send grant tx")?;
            println!("tx submitted: {}", pending.tx_hash());
            let _ = pending.watch().await.context("wait grant tx")?;
            println!("grant OK for {wallet_addr}");
            Ok(())
        }
        Commands::Revoke { wallet, config } => {
            let path = resolve_config_path(config);
            let cfg = load_config(&path)?;
            let contract_addr = parse_address(&cfg.contract)?;
            let wallet_addr = parse_address(&wallet)?;
            let signer = read_signer_hex(&cfg.private_key_path)?;
            let url = cfg.rpc_url.parse().context("rpc_url")?;
            let provider = ProviderBuilder::new()
                .wallet(EthereumWallet::from(signer))
                .connect_http(url);
            let c = Gapura::new(contract_addr, &provider);
            let pending = c
                .revoke(wallet_addr)
                .send()
                .await
                .context("send revoke tx")?;
            println!("tx submitted: {}", pending.tx_hash());
            let _ = pending.watch().await.context("wait revoke tx")?;
            println!("revoke OK for {wallet_addr}");
            Ok(())
        }
        Commands::Status {
            cluster,
            hosts_file,
            wallet,
            config,
        } => {
            if cluster {
                let path = hosts_file.unwrap_or_else(default_hosts_path);
                let hosts = load_hosts(&path)?;
                return run_cluster_status(hosts);
            }
            let path = resolve_config_path(config);
            let cfg = load_config(&path)?;
            let contract_addr = parse_address(&cfg.contract)?;
            let url = cfg.rpc_url.parse().context("rpc_url")?;
            let provider = ProviderBuilder::new().connect_http(url);
            let chain = provider.get_chain_id().await.context("get_chain_id")?;
            let c = Gapura::new(contract_addr, &provider);
            let owner = c.owner().call().await.context("owner()")?;
            println!("chain_id:       {chain}");
            println!("contract:       {contract_addr}");
            println!("contract owner: {owner}");
            if let Some(w) = wallet {
                let w = parse_address(&w)?;
                let allowed = c.isAllowed(w).call().await.context("isAllowed")?;
                let n = c.keyCount(w).call().await.context("keyCount")?;
                println!("wallet {w}: allowed={allowed} key_count={n}");
                if allowed {
                    let keys = c.getActiveKeys(w).call().await.context("getActiveKeys")?;
                    for (i, k) in keys.iter().enumerate() {
                        println!("  [{i}] {k}");
                    }
                }
            }
            Ok(())
        }
        Commands::Audit { from_block, config } => {
            let path = resolve_config_path(config);
            let cfg = load_config(&path)?;
            let contract_addr = parse_address(&cfg.contract)?;
            let url = cfg.rpc_url.parse().context("rpc_url")?;
            let provider = ProviderBuilder::new().connect_http(url);
            let from = BlockNumberOrTag::Number(from_block);
            let to = BlockNumberOrTag::Latest;

            let f_granted = Filter::new()
                .address(contract_addr)
                .event_signature(Gapura::KeyGranted::SIGNATURE_HASH)
                .from_block(from)
                .to_block(to);
            let logs_g = provider
                .get_logs(&f_granted)
                .await
                .context("get_logs KeyGranted")?;

            let f_revoked = Filter::new()
                .address(contract_addr)
                .event_signature(Gapura::KeyRevoked::SIGNATURE_HASH)
                .from_block(from)
                .to_block(to);
            let logs_r = provider
                .get_logs(&f_revoked)
                .await
                .context("get_logs KeyRevoked")?;

            println!("KeyGranted ({})", logs_g.len());
            for log in logs_g {
                match log.log_decode::<Gapura::KeyGranted>() {
                    Ok(d) => {
                        let ev = d.into_inner();
                        println!(
                            "  block {} tx {} wallet {:?} key: {}",
                            log.block_number.unwrap_or_default(),
                            log.transaction_hash.unwrap_or_default(),
                            ev.wallet,
                            ev.sshKey
                        );
                    }
                    Err(_) => println!("  block {:?} (decode failed)", log.block_number),
                }
            }
            println!("KeyRevoked ({})", logs_r.len());
            for log in logs_r {
                match log.log_decode::<Gapura::KeyRevoked>() {
                    Ok(d) => {
                        let ev = d.into_inner();
                        println!(
                            "  block {} tx {} wallet {:?}",
                            log.block_number.unwrap_or_default(),
                            log.transaction_hash.unwrap_or_default(),
                            ev.wallet
                        );
                    }
                    Err(_) => println!("  block {:?} (decode failed)", log.block_number),
                }
            }
            Ok(())
        }
    }
}

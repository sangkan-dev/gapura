# Installation (host) — gapura-sentinel

This guide installs **`gapura-sentinel`** on a Linux SSH host and configures OpenSSH to use it via `AuthorizedKeysCommand`.

## Prerequisites

- Linux host running OpenSSH (`sshd`)
- Outbound HTTPS to your EVM RPC endpoint
- `curl`, `jq`, `sha256sum` (untuk install dari GitHub Release)

## Quick install (script)

### Install dari GitHub Release (default)

```bash
curl -fsSL https://raw.githubusercontent.com/sangkan-dev/gapura/main/scripts/install.sh -o install.sh
chmod +x install.sh
./install.sh
```

The script:

- installs `gapura-sentinel` to `/usr/local/bin/gapura-sentinel`
- creates dedicated user `gapura-sentinel`
- writes `/etc/gapura/sentinel.toml` and `/etc/gapura/users.toml` templates
- writes `/etc/ssh/sshd_config.d/99-gapura.conf`
- runs `sshd -t`

### Install dari source (alternatif)

Jika kamu tidak ingin download artifact, kamu bisa build dari source:

```bash
git clone https://github.com/sangkan-dev/gapura.git
cd gapura
cd sentinel && cargo build --release
cd ..
SENTINEL_BIN=./sentinel/target/release/gapura-sentinel ./scripts/install.sh
```

## Configure

1. Edit `/etc/gapura/sentinel.toml`:
   - set `rpc_url` to a **private** endpoint
   - set `contract` to your deployed Gapura contract address
   - optionally enable `cache_dir`
2. Edit `/etc/gapura/users.toml` to map usernames to wallets.

## Verify

- Run doctor check locally:

```bash
sudo -u gapura-sentinel /usr/local/bin/gapura-sentinel doctor
```

- Reload SSH:

```bash
sudo systemctl reload sshd
```

- Perform E2E (see [`e2e-checklist.md`](e2e-checklist.md)).

## Notes

- Docs: [`sshd.md`](sshd.md), [`sentinel.md`](sentinel.md)
- Cluster health check (admin workstation): `gapura status --cluster` (requires `hosts.toml` inventory)

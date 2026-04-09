#!/usr/bin/env bash
# Best-practice installer for Gapura on a Linux SSH host.
#
# - Installs gapura-sentinel to /usr/local/bin
# - Creates dedicated user `gapura-sentinel`
# - Creates /etc/gapura/{sentinel.toml,users.toml}
# - Optional disk cache dir /var/lib/gapura/cache
# - Writes sshd config fragment /etc/ssh/sshd_config.d/99-gapura.conf
#
# Requires: sudo/root, OpenSSH, and a built binary at:
#   sentinel/target/release/gapura-sentinel
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"

SENTINEL_BIN="${SENTINEL_BIN:-$ROOT/sentinel/target/release/gapura-sentinel}"
INSTALL_BIN_DIR="${INSTALL_BIN_DIR:-/usr/local/bin}"
CONFIG_DIR="${CONFIG_DIR:-/etc/gapura}"
CACHE_DIR="${CACHE_DIR:-/var/lib/gapura/cache}"
SSHD_FRAGMENT="${SSHD_FRAGMENT:-/etc/ssh/sshd_config.d/99-gapura.conf}"
SENTINEL_USER="${SENTINEL_USER:-gapura-sentinel}"

if [[ ! -x "$SENTINEL_BIN" ]]; then
  echo "missing executable: $SENTINEL_BIN" >&2
  echo "build it with: (cd sentinel && cargo build --release)" >&2
  exit 1
fi

sudo mkdir -p "$INSTALL_BIN_DIR"
sudo install -m 0755 "$SENTINEL_BIN" "$INSTALL_BIN_DIR/gapura-sentinel"

if ! id -u "$SENTINEL_USER" >/dev/null 2>&1; then
  sudo useradd --system --no-create-home --shell /usr/sbin/nologin "$SENTINEL_USER"
fi

sudo mkdir -p "$CONFIG_DIR"
sudo tee "$CONFIG_DIR/sentinel.toml" >/dev/null <<'EOF'
# Gapura sentinel config
rpc_url = "https://sepolia.base.org"
contract = "0xYourGapuraContract"
users_path = "/etc/gapura/users.toml"

# Optional: disk fallback cache
# cache_dir = "/var/lib/gapura/cache"
# disk_fallback_ttl_secs = 300
EOF

sudo tee "$CONFIG_DIR/users.toml" >/dev/null <<'EOF'
[users]
# alice = "0x0000000000000000000000000000000000000abc"
EOF

sudo mkdir -p "$CACHE_DIR"
sudo chown -R "$SENTINEL_USER":"$SENTINEL_USER" "$CACHE_DIR"
sudo chmod 0700 "$CACHE_DIR"

sudo tee "$SSHD_FRAGMENT" >/dev/null <<EOF
# Managed by Gapura installer
AuthorizedKeysFile none
AuthorizedKeysCommand $INSTALL_BIN_DIR/gapura-sentinel %u
AuthorizedKeysCommandUser $SENTINEL_USER
EOF

echo "==> validating sshd config"
sudo sshd -t

echo "==> installed. next steps:"
echo "1) edit $CONFIG_DIR/sentinel.toml (rpc_url + contract)"
echo "2) edit $CONFIG_DIR/users.toml"
echo "3) sudo systemctl reload sshd (or your distro equivalent)"

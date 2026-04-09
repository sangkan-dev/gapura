#!/usr/bin/env bash
# Local smoke: Anvil + deploy Gapura + gapura CLI + gapura-sentinel (no full sshd).
# Requires: foundry (forge/anvil), rust bins built (cargo build in cli/ and sentinel/).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TMP="${TMPDIR:-/tmp}/gapura-smoke-$$"
mkdir -p "$TMP"

cleanup() {
  kill "${ANVIL_PID:-0}" 2>/dev/null || true
}
trap cleanup EXIT

echo "==> building release binaries if missing"
if [[ ! -x "$ROOT/cli/target/release/gapura" ]]; then
  (cd "$ROOT/cli" && cargo build --release -q)
fi
if [[ ! -x "$ROOT/sentinel/target/release/gapura-sentinel" ]]; then
  (cd "$ROOT/sentinel" && cargo build --release -q)
fi

echo "==> starting anvil"
anvil --host 127.0.0.1 --port 8545 >/dev/null 2>&1 &
ANVIL_PID=$!
sleep 1

export RPC_URL="http://127.0.0.1:8545"
# First Anvil account (Foundry default)
export ANVIL_PK="${ANVIL_PK:-0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80}"

echo "==> deploying Gapura"
cd "$ROOT/contracts"
forge script script/Gapura.s.sol:GapuraScript \
  --rpc-url "$RPC_URL" \
  --broadcast \
  --private-key "$ANVIL_PK" \
  >/dev/null 2>&1

CHAIN_DIR="$ROOT/contracts/broadcast/Gapura.s.sol/31337"
LATEST="$CHAIN_DIR/run-latest.json"
if [[ ! -f "$LATEST" ]]; then
  echo "expected $LATEST missing" >&2
  exit 1
fi
ADDR="$(jq -er '.transactions[0].contractAddress' "$LATEST")"
echo "    contract: $ADDR"

DEPLOYER_ADDR="$(cast wallet address --private-key "$ANVIL_PK")"
echo "==> gapura init + grant"
mkdir -p "$TMP/gapura"
echo "$ANVIL_PK" | sed 's/^0x//' > "$TMP/gapura/pk.hex"

CLI_CFG="$TMP/gapura/config.toml"
"$ROOT/cli/target/release/gapura" init \
  --rpc-url "$RPC_URL" \
  --private-key-path "$TMP/gapura/pk.hex" \
  --contract "$ADDR" \
  --config "$CLI_CFG"

SSH_KEY='ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIDev-smoke-test-key'
"$ROOT/cli/target/release/gapura" grant "$DEPLOYER_ADDR" "$SSH_KEY" --config "$CLI_CFG"

cat > "$TMP/sentinel.toml" <<EOF
rpc_url = "$RPC_URL"
contract = "$ADDR"
users_path = "$TMP/users.toml"
cache_dir = "$TMP/cache"
disk_fallback_ttl_secs = 300
EOF

cat > "$TMP/users.toml" <<EOF
[users]
alice = "$DEPLOYER_ADDR"
EOF

echo "==> gapura-sentinel alice (expect one key line)"
export GAPURA_CONFIG="$TMP/sentinel.toml"
OUT="$("$ROOT/sentinel/target/release/gapura-sentinel" alice)"
if [[ "$OUT" != "$SSH_KEY" ]]; then
  echo "unexpected sentinel output: $OUT" >&2
  exit 1
fi
echo "    OK: stdout matches granted key"

echo "==> revoke + sentinel empty"
"$ROOT/cli/target/release/gapura" revoke "$DEPLOYER_ADDR" --config "$CLI_CFG"
sleep 1
OUT2="$("$ROOT/sentinel/target/release/gapura-sentinel" alice || true)"
if [[ -n "$OUT2" ]]; then
  echo "expected empty stdout after revoke, got: $OUT2" >&2
  exit 1
fi
echo "    OK: empty keys after revoke"

echo "==> smoke passed"
cleanup
trap - EXIT

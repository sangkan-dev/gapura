# gapura-sentinel configuration

Binary invoked by OpenSSH as `AuthorizedKeysCommand` (see [`sshd.md`](sshd.md)).

## `sentinel.toml`

Path: `/etc/gapura/sentinel.toml`, or override with env **`GAPURA_CONFIG`**.

```toml
rpc_url = "https://sepolia.base.org"
contract = "0xYourGapuraContract"
users_path = "/etc/gapura/users.toml"

# Optional: directory for per-wallet JSON snapshots (used when RPC errors).
cache_dir = "/var/lib/gapura/cache"
# How old (seconds) a snapshot may be to still be trusted during RPC outage (default 300).
disk_fallback_ttl_secs = 300
```

- **`cache_dir`**: if unset, sentinel only uses in-memory cache (30s TTL) and denies on RPC failure (empty stdout).
- On every **successful** `getActiveKeys`, the filtered key list is written to `{cache_dir}/{wallet_lower}.json`.
- If **RPC fails** and a snapshot exists with age ≤ `disk_fallback_ttl_secs`, those keys are printed and a line is logged to stderr (`disk fallback`).

Create `cache_dir` with permissions so `AuthorizedKeysCommandUser` can read/write, or pre-create as root and `chown`/`chmod` appropriately.

## Encryption / confidentiality (M2.6)

The on-disk JSON contains **public** SSH keys only (no private keys). For **encryption at rest** the recommended approach is **environmental**, not application crypto:

- Place `cache_dir` on a **LUKS-encrypted** volume, or restrict host disk access.
- Tighten **directory ACLs** / owner so only the sentinel user can read the cache.
- Optionally wrap deploy secrets (`sentinel.toml` RPC URLs with API keys) using **sops**, **age**, or a secret manager; avoid committing real configs.

Application-level AES for a public-key cache adds key-management complexity with little benefit if the host is already compromised.

## `users.toml`

See [`sshd.md`](sshd.md).

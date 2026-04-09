# Operations runbook (M5)

## Multi-node deployment

All nodes using Gapura for SSH should share:

- The **same** Gapura **contract address** and chain/RPC class (or equivalent reliability).
- The **same** policy intent: each node’s [`users.toml`](sshd.md) maps local Unix users → on-chain wallets; keys come only from `getActiveKeys`.

**Bootstrap order**

1. Deploy contract once (Base Sepolia / mainnet / internal L2).
2. Install `gapura-sentinel` + configs on each host; ensure `AuthorizedKeysCommandUser` can read TOML and optional `cache_dir`.
3. Run `gapura init` on admin workstation; `grant` wallets used across the cluster.
4. Roll `sshd` after config; verify one node, then repeat or automate (Ansible, etc.).

Nodes do **not** need separate contracts; **revoke** is global for that wallet.

## Emergency (break-glass) access

Do **not** put emergency keys inside Gapura’s key pipeline. Options:

- Maintain a **separate** `authorized_keys` path used only when Gapura is disabled, **or**
- Keep provider-documented recovery (serial / IPMI / cloud console) outside SSH.

Document who may enable break-glass and how to **re-disable** it. See [`sshd.md`](sshd.md) §Emergency access.

## Monitoring & observability

- **RPC health:** log/supervise outbound HTTPS to your provider; alert on sustained 5xx / timeouts.
- **Sentinel stderr:** `chain error` lines indicate deny path; correlate spike with RPC outage; disk fallback (if configured) reduces total lockout but extends stale key window—tune `disk_fallback_ttl_secs`.
- **Audit trail:** use `gapura audit` or indexer later; on-chain events remain the source of truth.

## Rate limiting

Per-connection cost is one `getActiveKeys` (plus caching). For brute-force storms, combine with **`MaxAuthTries`**, **`LoginGraceTime`**, and network-level controls; Gapura does not replace `sshd` rate limits.

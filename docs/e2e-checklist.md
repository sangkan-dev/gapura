# E2E checklist (M3.2) — VM / lab

Use this template to record a real **grant → SSH OK → revoke → deny** run. Smoke script without full SSH: [`scripts/dev-smoke.sh`](../scripts/dev-smoke.sh).

| Field | Value |
|--------|--------|
| Date | |
| Git commit | |
| Chain / RPC | |
| Contract address | |
| Tester | |

## Preconditions

- [ ] `gapura` built (`cli/`), `gapura-sentinel` built (`sentinel/`).
- [ ] Gapura contract deployed; owner key available for CLI.
- [ ] Target SSH host: `gapura-sentinel` + `sentinel.toml` + `users.toml` + `sshd_config` per [`sshd.md`](sshd.md).

## Steps

1. [ ] `gapura init` (or existing `~/.config/gapura/config.toml`) with matching RPC + contract.
2. [ ] `gapura grant <wallet> "<ssh-ed25519 ...>"` — wallet must match `users.toml` for test user.
3. [ ] On server: confirm `sshd` reloaded; wait ≥ sentinel cache TTL if re-testing (30s default).
4. [ ] **SSH in** with the granted key → **expect success**.
5. [ ] `gapura revoke <wallet>`.
6. [ ] Wait for cache expiry (≤ ~30s in-memory, or reconnect until `getActiveKeys` empty).
7. [ ] **SSH retry** → **expect failure** (no valid `authorized_keys` from command).

## Notes / anomalies

(Attach `sshd -T` relevant lines, journalctl, stderr from sentinel if needed.)

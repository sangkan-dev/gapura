# OpenSSH integration (Gapura Sentinel)

Gapura uses **`AuthorizedKeysCommand`** so `sshd` asks `gapura-sentinel` which keys are valid for the logging-in user (PRD §5.B).

## `sshd_config` snippet

Use a dedicated config fragment (e.g. `/etc/ssh/sshd_config.d/99-gapura.conf`) **or** merge into the main `sshd_config`. Adjust the path to the installed binary.

```text
AuthorizedKeysFile none
AuthorizedKeysCommand /usr/local/bin/gapura-sentinel %u
AuthorizedKeysCommandUser gapura-sentinel
```

Then reload SSH: `sudo systemctl reload sshd` (or `ssh`-named unit on some distros).

### Notes

- **`AuthorizedKeysCommandUser`**: run the command as an unprivileged **dedicated** user (recommended: `gapura-sentinel`). Ensure that user can read `/etc/gapura/sentinel.toml` and `/etc/gapura/users.toml` (group permissions or ACLs), and can write to `cache_dir` if enabled.
- **Firewall / RPC**: the host needs outbound HTTPS to your EVM RPC endpoint.
- **Emergency access**: see [Emergency / break-glass](#emergency--break-glass) below.

## Emergency / break-glass

Goal: regain SSH if Gapura stack (RPC, contract, sentinel bug) blocks all logins.

1. **Preferred:** out-of-band access (hardware serial, IPMI, cloud serial console, VNC) — no SSH change.
2. **SSH fallback:** temporarily **comment out** or remove the Gapura `AuthorizedKeysCommand` block and restore a **classic** `AuthorizedKeysFile` for root (or a bootstrap user) **only** until service is healthy. Example pattern (adjust paths):
   - Keep a file `/root/.ssh/authorized_keys.emergency` on the server with **one** admin key; **do not** reference this file while Gapura is active if policy forbids dual path.
   - In emergency, set `AuthorizedKeysFile /root/.ssh/authorized_keys.emergency` for a single `Match` block or briefly for global (understand security impact).
3. After recovery, **revert** to Gapura-only config and `reload` sshd.
4. Record the incident (RPC outage vs misconfig) and update monitoring. See [`runbook.md`](runbook.md).

Formal checklist template: [`e2e-checklist.md`](e2e-checklist.md).

## Sentinel config files

- **`/etc/gapura/sentinel.toml`** (override with env `GAPURA_CONFIG`):

```toml
rpc_url = "https://sepolia.base.org"
contract = "0xYourGapuraContract"
users_path = "/etc/gapura/users.toml"
# Optional RPC outage fallback — see docs/sentinel.md
# cache_dir = "/var/lib/gapura/cache"
```

- **`/etc/gapura/users.toml`** — map Unix username → wallet checked on-chain:

```toml
[users]
alice = "0x0000000000000000000000000000000000000abc"
```

## End-to-end check (VM / lab)

Suggested order (see [TASK.md](../TASK.md)):

1. Deploy **Gapura** on Base Sepolia (or Anvil) and run `gapura init` with RPC, owner key path, and contract address.
2. `gapura grant <wallet> "<ssh public key>"` for the wallet listed in `users.toml`.
3. On the SSH server: install `gapura-sentinel`, place `sentinel.toml` + `users.toml`, configure `sshd_config`, reload `sshd`.
4. **Success**: `ssh alice@server` accepts the key that was granted.
5. `gapura revoke <wallet>` then retry SSH: login should **fail** for that user once cache/TTL allows fresh chain state (Sentinel TTL ~30s by default).

Record RPC URL and contract address consistently across CLI config and `sentinel.toml`.

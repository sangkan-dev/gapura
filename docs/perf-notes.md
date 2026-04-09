# Performance notes (NFR snapshot)

Reference measurements for PRD §7-style targets (not a continuous benchmark suite).

## Environment

- Host: developer machine / CI runner (document your own when re-measuring).
- Binary: `sentinel/target/release/gapura-sentinel` after `cargo build --release`.
- Method: GNU `time -v` (`/usr/bin/time -v`) for a **single** short-lived process.

## Resident memory (RSS)

One sample **cold start** run (RPC unreachable immediately after start — error path, minimal work):

- **Maximum resident set size:** ~7 MB (7120 KiB reported by `time -v` on one Linux run).

This is **well under** the PRD ballpark of ~20 MB for sentinel. Re-measure under your libc/openssl and with a **successful** JSON-RPC round-trip if you need production sign-off.

## Latency

SSH login latency depends mainly on **RPC round-trip** and TLS to the provider. The sentinel keeps an **in-process** moka cache (30s TTL); **each `AuthorizedKeysCommand` invocation is typically a new process**, so the cache often does **not** carry between consecutive SSH attempts unless you change the integration model (persistent helper). For PRD’s “< ~800 ms with cache,” interpret as:

- optimize RPC (regional private endpoint, HTTP keep-alive would require a long-lived sidecar), and/or
- accept first-connection cost and rely on operator network closeness to RPC.

Re-measure with:

```bash
/usr/bin/time -p env GAPURA_CONFIG=/path/to/sentinel.toml ./gapura-sentinel username
```

against your real Base Sepolia / mainnet RPC.

## Reproduce RSS sanity check (offline error path)

With any `sentinel.toml` pointing at a **stopped** local port (immediate connection failure), `time -v` still exercises binary startup and teardown; numbers are indicative only of **memory floor**, not peak under full Alloy JSON stack + TLS to a remote RPC.

---

*Last updated with automated gap-close pass; adjust dates/commit when you re-run.*

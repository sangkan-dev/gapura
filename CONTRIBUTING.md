# Contributing

Terima kasih sudah mau kontribusi ke Gapura.

## Dev setup

- Rust toolchain (stable)
- Foundry (`forge`, `anvil`, `cast`)

## Commands

### Rust (workspace root)

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

### Contracts (Foundry)

```bash
cd contracts
forge fmt
forge build
forge test
```

### Smoke test lokal

```bash
./scripts/dev-smoke.sh
```

## Pull request checklist

- [ ] `cargo fmt --check` lulus
- [ ] `cargo clippy ... -D warnings` lulus
- [ ] `cargo test` lulus
- [ ] `cd contracts && forge test` lulus
- [ ] Update docs bila ada perubahan config / behavior (`docs/`)
- [ ] Tidak ada secret yang ikut ke commit (`.env`, private key, RPC API keys)


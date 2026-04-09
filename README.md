# Gapura (ꦒꦥꦸꦫ)

Gapura adalah “pintu masuk utama” untuk akses SSH di cluster/homelab: **smart contract EVM** menjadi **source of truth** untuk siapa yang boleh login, dan **OpenSSH** memanggil sentinel untuk mengambil `authorized_keys` secara dinamis.

Gapura adalah produk dari **Sangkan (ꦱꦁꦏꦤ꧀)**.

- GitHub: https://github.com/sangkan-dev/
- Website: https://sangkan.dev/

Spesifikasi produk: [`PRD.md`](PRD.md). Task list: [`TASK.md`](TASK.md).

## Struktur repo

- [`contracts/`](contracts/): Foundry + kontrak `Gapura.sol` + test + deploy script
- [`sentinel/`](sentinel/): binary `gapura-sentinel` (OpenSSH `AuthorizedKeysCommand`)
- [`cli/`](cli/): binary `gapura` (admin CLI)
- [`docs/`](docs/): runbook, install, sshd, sentinel config, checklist E2E
- [`scripts/`](scripts/): smoke test lokal dan installer

## Quickstart

### Contracts (Foundry)

```bash
cd contracts
forge build
forge test
```

Deploy (lihat juga [`contracts/README.md`](contracts/README.md)):

```bash
cd contracts
forge script script/Gapura.s.sol:GapuraScript --rpc-url base_sepolia --broadcast --verify -vvv
```

### CLI (admin) — `gapura`

```bash
cd cli
cargo build --release
./target/release/gapura --help
```

Contoh pakai:

```bash
./target/release/gapura init --rpc-url "<RPC_URL>" --private-key-path "<PATH_TO_HEX_KEY>" --contract "<GAPURA_CONTRACT>"
./target/release/gapura grant <wallet> "ssh-ed25519 AAAA..."
./target/release/gapura revoke <wallet>
./target/release/gapura status --wallet <wallet>
./target/release/gapura audit --from-block 0
```

### Sentinel (host) — `gapura-sentinel`

Install (recommended, from GitHub Release):

```bash
curl -fsSL https://raw.githubusercontent.com/sangkan-dev/gapura/main/scripts/install.sh -o install.sh
chmod +x install.sh
./install.sh
```

Build from source (alternative):

```bash
cd sentinel
cargo build --release
```

Install di host SSH:

- Script: [`scripts/install.sh`](scripts/install.sh)
- Docs: [`docs/install.md`](docs/install.md), [`docs/sshd.md`](docs/sshd.md), [`docs/sentinel.md`](docs/sentinel.md)

Doctor check (di host):

```bash
sudo -u gapura-sentinel /usr/local/bin/gapura-sentinel doctor
```

### Smoke test (lokal, tanpa sshd)

```bash
./scripts/dev-smoke.sh
```

## Security notes

- Gunakan **private RPC endpoint** (Alchemy/QuickNode) dan simpan API key sebagai **environment variable** / secret manager.
- Jangan commit file `.env` atau config real `/etc/gapura/*.toml`.

## License

MIT — lihat [`LICENSE`](LICENSE).


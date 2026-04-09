## Foundry

**Foundry is a blazing fast, portable and modular toolkit for Ethereum application development written in Rust.**

Foundry consists of:

- **Forge**: Ethereum testing framework (like Truffle, Hardhat and DappTools).
- **Cast**: Swiss army knife for interacting with EVM smart contracts, sending transactions and getting chain data.
- **Anvil**: Local Ethereum node, akin to Ganache, Hardhat Network.
- **Chisel**: Fast, utilitarian, and verbose solidity REPL.

## Documentation

https://book.getfoundry.sh/

## Usage

### Build

```shell
$ forge build
```

### Test

```shell
$ forge test
```

### Format

```shell
$ forge fmt
```

### Gas Snapshots

```shell
$ forge snapshot
```

### Anvil

```shell
$ anvil
```

### Deploy Gapura (Base Sepolia)

Set a **private** RPC URL (Alchemy, QuickNode, etc.)—avoid committing API keys. For contract verification on Basescan, set `BASESCAN_API_KEY` in your environment (see [`foundry.toml`](foundry.toml)).

```shell
# Deploy (broadcast)
forge script script/Gapura.s.sol:GapuraScript \
  --rpc-url base_sepolia \
  --broadcast \
  --verify \
  -vvv

# Or explicit RPC:
forge script script/Gapura.s.sol:GapuraScript \
  --rpc-url "$BASE_SEPOLIA_RPC_URL" \
  --broadcast \
  -vvv
```

After deploy, note the **Gapura** contract address and configure `gapura-sentinel` + `gapura` CLI. SSH integration: [`docs/sshd.md`](../docs/sshd.md). Local smoke (Anvil + grant/revoke + sentinel): [`scripts/dev-smoke.sh`](../scripts/dev-smoke.sh). Task list: [`TASK.md`](../TASK.md).

### Cast

```shell
$ cast <subcommand>
```

### Help

```shell
$ forge --help
$ anvil --help
$ cast --help
```

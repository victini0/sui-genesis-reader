# sui-genesis-reader

A tool that parses Sui's mainnet `genesis.blob` using Sui's own deserialization code. It outputs a comprehensive analysis of the initial token distribution, validator set, concentration metrics, and on-chain locking status.

No custom parsing — it uses the same `Genesis::load()` that Sui validators use.

## Key Findings

Running this on the Sui mainnet genesis blob produces:

- **10B total supply** distributed across **178 addresses** and **100 validators**
- **Top 2 addresses hold 68.20%** of total supply (41.34% + 26.86%)
- **Gini coefficient: 0.9547** (extreme concentration)
- **592 genesis objects**: every token allocation is a plain `0x2::coin::Coin<0x2::sui::SUI>` (GasCoin)
- **Zero on-chain vesting or time-lock contracts** in the genesis state — despite the codebase containing [working vesting implementations](https://github.com/MystenLabs/sui/tree/main/examples/move/vesting)

## Quick Start

### Prerequisites

- [Rust toolchain](https://rustup.rs/) (1.75+)
- Git
- ~10 GB disk space (Sui repo is large)
- ~10 minutes for the first build

### Option A: Automated Setup

```bash
git clone https://github.com/victini0/sui-genesis-reader.git
cd sui-genesis-reader
chmod +x setup.sh
./setup.sh
```

### Option B: Manual Setup

```bash
# 1. Clone this repo
git clone https://github.com/victini0/sui-genesis-reader.git

# 2. Clone the Sui repository
git clone https://github.com/MystenLabs/sui.git

# 3. Clone the genesis blob
git clone https://github.com/MystenLabs/sui-genesis.git

# 4. Copy this reader into Sui's crates directory
cp -r sui-genesis-reader sui/crates/

# 5. Register in workspace (add to members array in sui/Cargo.toml)
sed -i '' '/"crates\/sui-genesis-builder",/a\
  "crates/sui-genesis-reader",
' sui/Cargo.toml

# 6. Build and run
cd sui
cargo run --release -p sui-genesis-reader -- ../sui-genesis/mainnet/genesis.blob
```

### Save Output to File

```bash
cargo run --release -p sui-genesis-reader -- ../sui-genesis/mainnet/genesis.blob > output.txt
```

The first build takes ~10 minutes due to the large dependency tree. Subsequent builds are fast.

## Pre-built Output

If you don't want to build from source, the full output is included in this repository:

- [`output/genesis-analysis.txt`](output/genesis-analysis.txt) — complete analysis output

## Output Sections

| # | Section | What it shows |
|---|---------|---------------|
| 1 | Genesis Metadata | Hash, epoch, timestamp, total object count |
| 2 | System State | Protocol version, safe mode status |
| 3 | System Parameters | Epoch duration, validator thresholds |
| 4 | Stake Subsidy | Subsidy fund balance, distribution schedule projection |
| 5 | Storage Fund | Object storage rebates |
| 6 | Validators | All 100 validators sorted by stake, with statistics |
| 7 | Token Distribution | All 178 genesis addresses with liquid/staked balances |
| 8 | Concentration | Top-N holdings and Gini coefficient |
| 9 | Genesis Objects | All object types and counts (592 total) |
| 10 | Committee | Epoch, committee size, voting power |
| 11 | Locking Analysis | Scan for any time-lock or vesting contracts |

## How It Works

The core of the tool is just this:

```rust
use sui_config::genesis::Genesis;

let genesis = Genesis::load("path/to/genesis.blob").unwrap();

// Every genesis object is accessible
for obj in genesis.objects() {
    println!("{:?}", obj.type_());
}

// System state, validators, subsidy config, etc.
let system_state = genesis.sui_system_object();
let committee = genesis.committee();
```

Every number in the output comes directly from Sui's own types. There is no interpretation or transformation — just reading what the genesis file contains.

## Verifying with Public RPC

You can also check the current state of genesis addresses via Sui's public RPC endpoint:

```bash
# Check current balance of the largest genesis address
curl -s -X POST https://fullnode.mainnet.sui.io:443 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0", "id": 1,
    "method": "suix_getBalance",
    "params": [
      "0x341fa71e4e58d63668034125c3152f935b00b0bb5c68069045d8c646d017fae1",
      "0x2::sui::SUI"
    ]
  }'

# Check staking positions
curl -s -X POST https://fullnode.mainnet.sui.io:443 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0", "id": 1,
    "method": "suix_getStakes",
    "params": [
      "0x341fa71e4e58d63668034125c3152f935b00b0bb5c68069045d8c646d017fae1"
    ]
  }'

# Check transaction history (most recent 10)
curl -s -X POST https://fullnode.mainnet.sui.io:443 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0", "id": 1,
    "method": "suix_queryTransactionBlocks",
    "params": [
      {
        "filter": {"FromAddress": "0x341fa71e4e58d63668034125c3152f935b00b0bb5c68069045d8c646d017fae1"},
        "options": {"showInput": true}
      },
      null, 10, true
    ]
  }'
```

## License

This tool uses Sui's Apache-2.0 licensed code for deserialization.

Copyright (c) Mysten Labs, Inc.
SPDX-License-Identifier: Apache-2.0

# solana-indexer

POC Solana indexer for tracking blocks, programs and event logs.

**WIP**

## Overview

This indexer is designed to track event logs on Solana. It is written in Rust and uses the Solana Rust SDK to interact with the Solana blockchain.

The indexer can be configured with addresses of programs to track. It allows for tracking of multiple programs and to apply filters on incoming logs.

### How it works

**Event loader** continuously polls for logs by looping over txs of registered addresses ([get_signatures_for_address](https://solana.com/docs/rpc/http/getsignaturesforaddress)) and fetches the tx for each signature ([get_transaction](https://solana.com/docs/rpc/#gettransaction)).
For each tx, it extracts logs and stores them in a file/db.

## Usage

### Local Development

#### local solana test validator

Spin up a local solana test validator to test the indexer.

```shell
./scripts/localnet/up.sh
```

Take down the local solana test validator.

```shell
./scripts/localnet/down.sh
```

#### deploy programs

Deploy programs in `./programs/localnet` to the local solana test validator.

```shell
./scripts/deploy.sh
```

#### run the indexer

Create a `.env` file based on `example.env`, then run the indexer bin:

```shell
cargo run --bin indexer
```

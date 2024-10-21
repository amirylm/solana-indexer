# solana-indexer
Solana indexer for tracking blocks, programs and event logs.

## Overview

This indexer is designed to track Solana blocks, programs and event logs. It is written in Rust and uses the Solana Rust SDK to interact with the Solana blockchain.

The indexer can be configured with addresses of programs to track. It allows for tracking of multiple programs and multiple logs per program, while also applying filters on incoming logs.

## Design

The indexer consists of the following components:

- **Solana RPC client**: responsible for interacting with the Solana blockchain
- **Slot subscriber**: subscribes to new slots ([slot_subscribe](https://solana.com/docs/rpc/websocket/slotsubscribe)) using websocket and notifies block & log pollers upon new slot/s
- **Block poller**: polls for new blocks ([get_blocks](https://solana.com/docs/rpc/#getconfirmedblocks)) and fetch corresponding information ([get_block](https://solana.com/docs/rpc/#getblock)) for existing blocks
- **Log subscriber**: subscribes to logs ([logs_subscribe](https://solana.com/docs/rpc/websocket/logssubscribe)) using websocket and notifies log poller upon new log for some address.
NOTE: using `logs_subscribe` is considered to be quite brittle and unreliable, thus we also use slot subscription to initiate log polling. 
- **Log poller**: continuously polls for logs by looping over txs of registered addresses ([get_signatures_for_address](https://solana.com/docs/rpc/http/getsignaturesforaddress)) and fetches the tx for each signature ([get_transaction](https://solana.com/docs/rpc/#gettransaction)) to extract logs.
  - Additionally, it also fetches specific tx ([get_transaction](https://solana.com/docs/rpc/#gettransaction)) based on notification from log subscriber.

The following diagram visualizes the components and their interactions:

![Indexer Design](./docs/design.png)

## Usage

Start local Solana network in docker:

```bash
./scripts/test_up.sh
```
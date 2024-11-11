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

Use the `test_up` script to run a local solana test validator.
The script does the following steps, where all steps are optional and can be controlled via flags:
- Cleans up previous state, if any 
- Creates a new container to run `solana-test-validator`
- Creates keypairs
- Requests airdrop for the keypairs
- Deploys programs given in `./programs/local` directory (takes only the `*.so` files)

```shell
./scripts/test_up.sh -h     
# Usage: ./scripts/test_up.sh [OPTIONS]
#  
# Options:
#   -h   Help
#   -c   Cleanup (and stop) current or previous local validator; false (default)
#   -n   Number of accounts to create; 2 (default)
#   -d   Program/s to deploy; * (default)
# 
# Example: (create 5 accounts, avoid program deployments and cleanup previous state)
#   > ./scripts/test_up.sh -n 5 -d false -c
```


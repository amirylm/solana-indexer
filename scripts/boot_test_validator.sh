#!/bin/bash

NUM_OF_ACCOUNTS="${NUM_OF_ACCOUNTS:-1}"
DEPOLY_PROGRAM="${DEPOLY_PROGRAM:-*}"
SOL_LEDGER="${SOL_LEDGER:-test-ledger}"

cd $SOL_LEDGER || echo "booting current dir $PWD" 

if [ $NUM_OF_ACCOUNTS -gt 0 ]; then
    solana-keygen grind --starts-with RM:${NUM_OF_ACCOUNTS}
    for keypair in RM*.json; do
        echo "> funding keypair $keypair"
        solana config set --url localhost --keypair $keypair > /dev/null
        solana airdrop 100
        sleep 1
    done
fi

sleep 3

for filename in progs/$DEPOLY_PROGRAM.so; do
    echo "> deploying local program: $filename"
    solana program deploy $filename
done

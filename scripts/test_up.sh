#!/bin/bash
NUM_OF_ACCOUNTS="${NUM_OF_ACCOUNTS:-2}"
DEPOLY_PROGRAM="${DEPOLY_PROGRAM:-*}"

usage() {
 echo "Usage: $0 [OPTIONS]"
 echo " "
 echo "Options:"
 echo "  -h   Help"
 echo "  -c   Cleanup (and stop) current or previous local validator; false (default)"
 echo "  -n   Number of accounts to create; 2 (default)"
 echo "  -d   Program to deploy (or \"*\" to depoly all programs); none (default)"
 echo " "
 echo "Examples: "
 echo "  cleanup previous state, create & fund 2 accounts (default) & deploy all programs in ./programs/local:"
 echo "  > ./scripts/test_up.sh -c -d \"*\""
 echo "  create & fund 1 account and deploy a single program"
 echo "  > ./scripts/test_up.sh -n 0 -d helloworld"
}

while getopts "hcnd:" flag; do
 case $flag in
   h)
   usage
   exit 0
   ;;
   c)
   CLEANUP="true"
   ;;
   n)
   NUM_OF_ACCOUNTS=$OPTARG
   ;;
   d)
   DEPOLY_PROGRAM=$OPTARG
   ;;
   \?)
   echo "Invalid option"
   usage
   exit 1
   ;;
 esac
done

if [ "$CLEANUP" = "true" ]; then
    docker rm -f solana-test-validator &> /dev/null
    rm -rf output/local
fi

mkdir -p output/local/progs
cp -r programs/local/*.so output/local/progs/ 

if [ ! "$(docker ps -a -q -f name=solana-test-validator)" ]; then
    docker run -d --name solana-test-validator  \
        -v "${PWD}/output/local:/test-ledger/" \
        -v "${PWD}/scripts/boot_test_validator.sh:/test-ledger/boot_test_validator.sh" \
        -p 8899:8899/tcp \
        -p 8900:8900/tcp \
        -p 9900:9900/tcp \
        -e NUM_OF_ACCOUNTS="$NUM_OF_ACCOUNTS" \
        -e DEPOLY_PROGRAM="$DEPOLY_PROGRAM" \
        nixos/nix bash \
        -c "nix-env -iA nixpkgs.bzip2 && nix-env -iA nixpkgs.solana-cli && solana-test-validator --ledger test-ledger"
       
    while ! docker logs solana-test-validator | grep -q "Ledger location: test-ledger";
    do
        sleep 10
    done
fi

echo "> local solana-test-validator is running, booting..."

docker exec solana-test-validator /bin/sh /test-ledger/boot_test_validator.sh || exit 1

echo "> solana-test-validator is ready, openning explorer"

open https://explorer.solana.com/?cluster=custom\&customUrl=http%3A%2F%2Flocalhost%3A8899


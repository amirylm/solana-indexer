#!/bin/bash
NUM_OF_ACCOUNTS=1

if [ "$1" = "clean" ]; then
    docker rm -f solana-test-validator &> /dev/null
    rm -rf output/local
fi

mkdir -p output/local/progs
cp -r programs/local/*.so output/local/progs/ 

if [ ! "$(docker ps -a -q -f name=solana-test-validator)" ]; then
    docker run -d --name solana-test-validator  \
        -v "${PWD}/output/local:/test-ledger/" \
        -p 8899:8899/tcp \
        -p 8900:8900/tcp \
        -p 9900:9900/tcp \
        -p 10000:10000/tcp \
        nixos/nix bash \
        -c "nix-env -iA nixpkgs.bzip2 && nix-env -iA nixpkgs.solana-cli && solana-test-validator --ledger test-ledger"
       
    while ! docker logs solana-test-validator | grep -q "Ledger location: test-ledger";
    do
        sleep 10
    done

    sleep 1

    echo "local solana node is running..."
fi

cat <<EOF | docker exec --interactive solana-test-validator sh
cd test-ledger
mkdir -p test_accounts
for i in {1..$NUM_OF_ACCOUNTS}
do
    solana-keygen grind --ignore-case --starts-with QN:1 > out.tmp 
    keypair="none"
    regex="([A-Za-z0-9]+.json)"
    for line in \$(cat out.tmp)
    do
        if [[ \$line =~ \$regex ]]
        then
            keypair="\${BASH_REMATCH[1]}"
        fi
    done
    rm out.tmp

    echo "funding wallet for user \$i with keypair \$keypair"
    mv \$keypair test_accounts/\$keypair
    solana config set --url localhost --keypair test_accounts/\$keypair > /dev/null
    solana airdrop 100
done
sleep 3
for filename in progs/*.so; do
    echo "deploying local program: \$filename"
    solana program deploy \$filename
done
EOF

open https://explorer.solana.com/?cluster=custom\&customUrl=http%3A%2F%2Flocalhost%3A8899


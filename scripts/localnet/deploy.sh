#!/usr/bin/env bash

app="${app:-helloworld}"
keypair="${keypair:-keypair.json}"
lib_rs="${lib_rs:-programs/${app}/src/lib.rs}"
static_program_id="${static_program_id:-8weB5xqS5jbQzxmHEr2e79UUSYur6QpFwkMtdGezgtPy}"

mkdir -p output/localnet/dev
cp -r "programs/localnet/" "output/localnet/dev/"
cd output/localnet/dev || exit 1

if [ ! -f "$keypair" ]; then
    echo "generating $keypair"
    solana-keygen new -o "$keypair" --silent --no-bip39-passphrase
fi

if [ ! -f "keypair.pub" ]; then
    echo "creating keypair.pub"
    program_id=$(solana-keygen pubkey "$keypair")
    echo "$program_id" > "keypair.pub"
else
    program_id=$(cat "keypair.pub")
fi

solana config set --url localhost --keypair "$keypair" 
solana airdrop 500 || solana-keygen recover "$keypair"

# update program id
sed -i -e "s|$static_program_id|$program_id|g" "Anchor.toml" && rm "Anchor.toml-e" 2>/dev/null
sed -i -e "s|$static_program_id|$program_id|g" "$lib_rs" && rm "$lib_rs-e" 2>/dev/null

# export RUST_BACKTRACE=1
if [ ! -f "target/deploy/$app.so" ]; then
    echo "building..."  && anchor build --program-name "$app"
fi
echo "deploying..." && anchor deploy --program-name "$app"

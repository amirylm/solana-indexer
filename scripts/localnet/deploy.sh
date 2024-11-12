#!/usr/bin/env bash

app="${app:-helloworld}"
keypair="${keypair:-keypair.json}"
pubkey="${pubkey:-keypair.pub}"
lib_rs="${lib_rs:-programs/${app}/src/lib.rs}"
static_program_id="${static_program_id:-8weB5xqS5jbQzxmHEr2e79UUSYur6QpFwkMtdGezgtPy}"

mkdir -p output/localnet/dev
cp -r "programs/localnet/" "output/localnet/dev/"
cd output/localnet/dev || exit 1

if [ ! -f "$keypair" ]; then
    echo "generating $keypair"
    solana-keygen new -o "$keypair" --silent --no-bip39-passphrase
fi

if [ ! -f "$pubkey" ]; then
    echo "creating $pubkey"
    program_id=$(solana-keygen pubkey "$keypair")
    echo "$program_id" > "$pubkey"
else
    program_id=$(cat "$pubkey")
fi

solana config set --url localhost --keypair "$keypair" 
solana airdrop 500 || solana-keygen recover "$keypair"

# update program id
sed -i -e "s|$static_program_id|$program_id|g" "Anchor.toml" && rm "Anchor.toml-e" 2>/dev/null
sed -i -e "s|$static_program_id|$program_id|g" "$lib_rs" && rm "$lib_rs-e" 2>/dev/null
if [[ "keypair.json" != "$keypair" ]]; then
    # update keypair reference
    sed -i -e "s|keypair.json|$keypair|g" "Anchor.toml" && rm "Anchor.toml-e" 2>/dev/null
fi
export RUST_BACKTRACE=1
if [ ! -f "target/deploy/$app.so" ] || [[ $force_build ]]; then
    echo "building $app wit anchor:"  && RUST_BACKTRACE=1 anchor build --program-name "$app" || exit 1
fi
echo "deploying $app anchor:" && anchor deploy --program-name "$app" || exit 1

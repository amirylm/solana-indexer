#!/usr/bin/env bash

bash "$(dirname -- "$0";)/down.sh"

startup_timeout=20
container_name="solana-test-validator"

echo "Starting $container_name"

docker run -d \
  -p 127.0.0.1:8899:8899 \
  -p 127.0.0.1:8900:8900 \
  -p 127.0.0.1:9900:9900 \
  -v "${PWD}/output/localnet:/test-ledger/" \
  --name "${container_name}" \
  nixos/nix bash \
  -c "nix-env -iA nixpkgs.bzip2 && nix-env -iA nixpkgs.solana-cli && solana-test-validator"

echo "Waiting for test validator to become ready.."
start_time=$(date +%s)

prev_output=""
while true
do
  output=$(docker logs $container_name 2>&1)

  if [[ "${output}" != "${prev_output}" ]]; then
    echo -n "${output#$prev_output}"
    prev_output="${output}"
  fi

  if [[ $output == *"Finalized Slot"* ]]; then
    echo ""
    echo "solana-test-validator is ready."
    exit 0
  fi

  current_time=$(date +%s)
  elapsed_time=$((current_time - start_time))
  if (( elapsed_time > startup_timeout )); then
    echo "Error: Command did not become ready within 20 seconds"
    exit 1
  fi
  sleep 2
done

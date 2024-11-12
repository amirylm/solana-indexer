#!/usr/bin/env bash

container_name="solana-test-validator"

echo "Cleaning up test validator container ($container_name)"

if [ "$(docker ps -a -q -f name=$container_name)" ]; then
    docker rm -f $container_name
else
    echo "No docker test validator container running.";
fi

if [[ $clean_volume ]]; then
    echo "Cleaning up dev volume"
    rm -rf output/localnet
fi

echo "Cleanup finished."
#!/bin/bash

wget https://raw.githubusercontent.com/firecracker-microvm/firecracker/main/src/api_server/swagger/firecracker.yaml -O firecracker.yaml
docker run --rm \
    -u $(id -u):$(id -g) \
    -v $PWD:/local openapitools/openapi-generator-cli generate \
    -i /local/firecracker.yaml \
    -g rust \
    -o /local/src/api
                
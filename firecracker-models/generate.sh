#!/bin/bash

wget https://raw.githubusercontent.com/firecracker-microvm/firecracker/main/src/api_server/swagger/firecracker.yaml -O firecracker-models/firecracker.yaml
docker run --rm \
    -u $(id -u):$(id -g) \
    -v $PWD/firecracker-models:/local openapitools/openapi-generator-cli generate \
    -i /local/firecracker.yaml \
    -g rust \
    -o /local/ \
    -c /local/openapi-generator.yml \
    -t /local/templates
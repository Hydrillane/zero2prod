#!/usr/bin/env bash

set -x
set -eo pipefail

RUNNING_CONTAINER=$(docker ps --filter 'name=valkey-server' --format '{{.ID}}')
if [[ -n $RUNNING_CONTAINER ]]; then
    echo >&2 "there is a valkey container already running, kill it with"
    echo >&2 "		docker kill ${RUNNING_CONTAINER}"
    exit 1
fi

CONTAINER_NAME="valkey_$(date '+%s')"

docker run \
    --name "$CONTAINER_NAME" \
    -p 6379:6379 \
    --rm \
    -d valkey/valkey:latest

>&2 echo "Valkey is ready to go!"

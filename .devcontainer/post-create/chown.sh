#!/usr/bin/env bash
set -euo pipefail

WORKSPACE="${1:?Usage: $0 <workspace>}"

sudo chown -R dev:dev "${WORKSPACE}/node_modules"
sudo chown -R dev:dev "${HOME}/.cargo"
sudo chown -R dev:dev "${WORKSPACE}/target"
sudo chown -R dev:dev "${HOME}/.cache/sccache"

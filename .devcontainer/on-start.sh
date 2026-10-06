#!/usr/bin/env bash
set -euo pipefail

WORKSPACE="${1:?Usage: $0 <workspace>}"

echo "Post Start Command"
echo "Workspace: ${WORKSPACE}"

bash "${WORKSPACE}/.devcontainer/on-start/dura.sh" "${WORKSPACE}"

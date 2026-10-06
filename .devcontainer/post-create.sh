#!/usr/bin/env bash
set -euo pipefail

WORKSPACE="${1:?Usage: $0 <workspace>}"

echo "Post Create Command"
echo "Workspace: ${WORKSPACE}"

bash "${WORKSPACE}/.devcontainer/post-create/chown.sh" "${WORKSPACE}"
bash "${WORKSPACE}/.devcontainer/post-create/setup.sh" "${WORKSPACE}"
bash "${WORKSPACE}/.devcontainer/post-create/zed-terminal-fix.sh"

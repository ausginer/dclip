#!/usr/bin/env sh
# Starts one role session, making both selections the role's definition declares.
#
#     .scripts/claude-role.sh architect   # → claude --agent architect --effort high
#
# `--agent` applies a definition's `model:` and not its `effort:`, so the level
# is the starter's to pass and a session started without it reasons cheaper than
# the role requires. This makes both selections from the same file and does
# nothing else: it is not an orchestration layer, holds no session state, and
# starts one role.
#
# It does not load the effort guard. The checkout's own `.claude/settings.json`
# does that, and naming the plugin here would make this a second loading
# mechanism — the one thing the loading rule forbids.
#
# Set CLAUDE_ROLE_PRINT_ARGV to print the resolved root and command line instead
# of starting a session.
set -eu

TAB=$(printf '\t')
RESOLVER="$(dirname "$0")/../.claude/plugins/harness-effort-guard/scripts/resolve-role.ts"

if [ "$#" -lt 1 ]; then
  printf 'usage: claude-role.sh <role>\n' >&2
  exit 64
fi

role=$1

# A convenience, not the boundary: the guard denies the same session at its
# first tool call whether or not this launcher started it. The variable outranks
# frontmatter for every subagent at once, so no per-role contract can be met
# while it is set, and there is no repair but removing it.
if [ -n "${CLAUDE_CODE_EFFORT_LEVEL:-}" ]; then
  printf 'CLAUDE_CODE_EFFORT_LEVEL=%s is set. It outranks every role definition, so no\n' \
    "$CLAUDE_CODE_EFFORT_LEVEL" >&2
  printf 'per-role effort contract can hold. Remove it and start again.\n' >&2
  exit 78
fi

# `<projectRoot><tab><declared effort>`, the effort empty for a role outside the
# invariant. The separator is written either way, and the cut is at the *last*
# one, so a root that happens to hold a tab cannot be read as a level.
line=$(node "$RESOLVER" "$role" "$PWD")
root=${line%"$TAB"*}
level=${line##*"$TAB"}

set -- --agent "$role"

if [ -n "$level" ]; then
  set -- "$@" --effort "$level"
fi

if [ -n "${CLAUDE_ROLE_PRINT_ARGV:-}" ]; then
  printf 'cwd=%s\n' "$root"
  printf 'claude'
  for argument in "$@"; do
    printf ' %s' "$argument"
  done
  printf '\n'
  exit 0
fi

cd "$root"
exec claude "$@"

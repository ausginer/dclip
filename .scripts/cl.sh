#!/usr/bin/env sh
set -eu

init=false
profile=

for arg do
  case "$arg" in
    --init) init=true ;;
    *)
      if [ -n "$profile" ]; then
        echo "Usage: cl [--init] <profile>" >&2
        exit 64
      fi
      profile=$arg
      ;;
  esac
done

case "$profile" in
  reviewer)     agent=reviewer;     effort=high;   name='DClip Reviewer' ;;
  architect)    agent=architect;    effort=high;   name='DClip Architect' ;;
  implementer)  agent=implementer;  effort=medium; name='DClip Implementer' ;;
  consolidator) agent=consolidator; effort=medium; name='DClip Review Swarm' ;;
  integrity)    agent=integrity;    effort=high;   name='DClip Integrity' ;;
  cleanup)      agent=cleanup;      effort=medium; name='DClip Cleanup' ;;
  der)          agent=der;          effort=medium; name='DClip Der' ;;
  *)
    echo "Usage: cl [--init] <profile>" >&2
    exit 64
    ;;
esac

if [ -n "${CLAUDE_CODE_EFFORT_LEVEL:-}" ]; then
  echo 'Unset CLAUDE_CODE_EFFORT_LEVEL: it overrides the role effort.' >&2
  exit 78
fi

# Follow the ~/.local/bin/cl symlink back to .scripts/cl.sh, then start at the checkout root.
script=$(readlink -f -- "$0")
cd "$(dirname "$(dirname "$script")")"

if [ "$init" = true ]; then
  exec claude --agent "$agent" --effort "$effort" --name "$name"
else
  exec claude --agent "$agent" --effort "$effort" --resume "$name"
fi

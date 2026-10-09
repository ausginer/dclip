#!/usr/bin/env bash
set -euo pipefail
source_dir=$(cd -- "$(dirname -- "$0")" && pwd)
destination="$HOME/.local/share/dclip"
command -v wl-paste >/dev/null
binary=${1:-"$source_dir/dist/dclip"}
if [[ ! -f "$binary" ]]; then
    printf 'Build the Rust binary first; see README.md. Missing: %s\n' "$binary" >&2
    exit 1
fi
mkdir -p "$destination/bin"
chmod 755 "$destination" "$destination/bin"
install -m 755 "$binary" "$destination/dclip"
install -m 755 "$source_dir/bridge.py" "$destination/bridge.py"
ln -sfn ../bridge.py "$destination/bin/wl-paste"
ln -sfn ../bridge.py "$destination/bin/xclip"
printf 'Installed: %s\nMount this DIRECTORY read-only at /opt/dclip.\n' "$destination"

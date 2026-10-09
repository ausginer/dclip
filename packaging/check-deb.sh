#!/usr/bin/env bash
# Installs the dclip .deb on Debian as a host would, checks every property the
# package promises, then removes it and checks that nothing it installed
# remains.
#
#   check-deb.sh PACKAGES REPO
#
# PACKAGES holds the build's one .deb. REPO holds the tracked Cargo.toml,
# bridge.py, LICENSE, NOTICE and README.md the package must carry.
set -euo pipefail
packages=$1
repo=$2
source "$(dirname -- "$0")/check-lib.sh"

shopt -s nullglob
debs=("$packages"/*.deb)
[[ ${#debs[@]} -eq 1 ]] || fail "expected one .deb in $packages, found ${#debs[@]}"
package=$(realpath "${debs[0]}")
version=$(workspace_version "$repo")
export DEBIAN_FRONTEND=noninteractive

# A Debian image may exclude /usr/share/doc from what dpkg unpacks. A host does
# not, and a check that asserts what the package ships has to see all of it.
# doc_exclusions prints, as FILE<TAB>LINE, each dpkg path-exclude whose glob
# reaches the package's documentation.
doc_exclusions() {
    local file line pattern
    for file in /etc/dpkg/dpkg.cfg /etc/dpkg/dpkg.cfg.d/*; do
        while IFS= read -r line; do
            pattern=${line#path-exclude}
            pattern=${pattern#[= ]}
            if [[ /usr/share/doc/dclip/LICENSE == $pattern ]]; then
                printf '%s\t%s\n' "$file" "$line"
            fi
        done < <(grep '^path-exclude' "$file" || true)
    done
}
while IFS=$'\t' read -r file line; do
    printf 'Removing from %s: %s\n' "$file" "$line"
    grep -vxF -- "$line" "$file" >"$file.new" || true
    mv "$file.new" "$file"
done < <(doc_exclusions)
expect "dpkg excludes nothing under /usr/share/doc/dclip" "" "$(doc_exclusions)"
eval "$(apt-config shell recommends APT::Install-Recommends)"
[[ -z "${recommends:-}" || "$recommends" =~ ^(1|[Tt]rue|yes)$ ]] ||
    fail "apt is configured not to install recommended packages: $recommends"
pass "apt installs recommended packages, as a host's does"

field() {
    dpkg-deb --field "$package" "$1"
}
expect "the package is dclip" "dclip" "$(field Package)"
expect "the package is at the workspace version, revision 1" "$version-1" "$(field Version)"
expect "the package is for amd64" "amd64" "$(field Architecture)"
expect "the package depends on wl-clipboard and nothing else" "wl-clipboard" "$(field Depends)"
expect "the package recommends xsel and nothing else" "xsel" "$(field Recommends)"
for relation in Pre-Depends Suggests Enhances Conflicts Breaks Replaces Provides; do
    expect "the package declares no $relation" "" "$(field "$relation")"
done
expect "the package has no maintainer scripts" \
    "./
./control
./md5sums" \
    "$(dpkg-deb --ctrl-tarfile "$package" | tar -t | sort)"

apt-get update
before=$(dpkg-query -W -f '${Package}\n' | sort)
apt-get install -y "$package"
new=$(added "$before" "$(dpkg-query -W -f '${Package}\n' | sort)")
printf 'Installed with dclip:\n%s\n' "$new"
grep -qx wl-clipboard <<<"$new" || fail "the install did not pull wl-clipboard"
pass "the install pulled wl-clipboard"
grep -qx xsel <<<"$new" || fail "the install did not pull xsel by default"
pass "the install pulled xsel by default"
if grep -qi python <<<"$new"; then
    fail "the install pulled Python: $(grep -i python <<<"$new" | tr '\n' ' ')"
fi
pass "the install pulled no Python"

# dpkg lists the shared parents it unpacked through as well, which other
# packages own too.
expect "dclip owns exactly its files and directories" \
    "/usr/bin/dclip
/usr/share/dclip
/usr/share/dclip/bin
/usr/share/dclip/bin/wl-paste
/usr/share/dclip/bin/xclip
/usr/share/dclip/bridge.py
/usr/share/doc/dclip
/usr/share/doc/dclip/LICENSE
/usr/share/doc/dclip/NOTICE
/usr/share/doc/dclip/README.md
/usr/share/doc/dclip/copyright" \
    "$(dpkg -L dclip | grep -vxE '/\.|/usr|/usr/bin|/usr/share|/usr/share/doc' | sort)"
expect_file "the host binary" /usr/bin/dclip 755
expect_dir "the shim's directory" /usr/share/dclip
expect_dir "the shim's bin" /usr/share/dclip/bin
expect_file "the shim" /usr/share/dclip/bridge.py 755 "$repo/bridge.py"
expect_link "the shim's wl-paste" /usr/share/dclip/bin/wl-paste ../bridge.py
expect_link "the shim's xclip" /usr/share/dclip/bin/xclip ../bridge.py
expect_dir "the documentation directory" /usr/share/doc/dclip
expect_file "the documentation" /usr/share/doc/dclip/README.md 644 "$repo/README.md"
expect_file "the license" /usr/share/doc/dclip/LICENSE 644 "$repo/LICENSE"
expect_file "the notice" /usr/share/doc/dclip/NOTICE 644 "$repo/NOTICE"
for path in /usr/share/doc/dclip /usr/share/doc/dclip/LICENSE /usr/share/doc/dclip/NOTICE; do
    expect "the package database names dclip as the owner of $path" \
        "dclip: $path" "$(dpkg -S "$path")"
done
expect_file "the copyright file" /usr/share/doc/dclip/copyright 644
expect "the copyright file names Apache-2.0" \
    "Apache-2.0" "$(sed -n 's/^License: //p' /usr/share/doc/dclip/copyright)"
expect "the copyright file's Copyright field is NOTICE's copyright line" \
    "$(sed -n 's/^Copyright //p' "$repo/NOTICE")" \
    "$(sed -n 's/^Copyright: //p' /usr/share/doc/dclip/copyright)"
expect "the installed files match the package database" "" "$(dpkg --verify dclip)"

usage=$( (dclip 2>&1 >/dev/null; echo "status $?") || true)
expect "dclip with no arguments prints the usage line and exits 1" \
    "dclip: Usage: dclip serve [--sync-text] [--allow-uid UID]
status 1" "$usage"

apt-get remove -y dclip
expect "dclip is no longer installed" "" \
    "$(dpkg-query -W -f '${db:Status-Abbrev}' dclip 2>/dev/null | grep '^ii' || true)"
expect_gone "removing dclip removes every file and directory it installed" \
    /usr/bin/dclip /usr/share/dclip /usr/share/doc/dclip

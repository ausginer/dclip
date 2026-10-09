#!/usr/bin/env bash
# Installs the dclip .rpm on Fedora as a host would, checks every property the
# package promises, runs the packaged layout end to end, then removes it and
# checks that nothing it installed remains.
#
#   check-rpm.sh PACKAGES REPO
#
# PACKAGES holds the build's one .rpm. REPO holds the tracked Cargo.toml,
# bridge.py, LICENSE, NOTICE and README.md the package must carry.
set -euo pipefail
packages=$1
repo=$2
source "$(dirname -- "$0")/check-lib.sh"

shopt -s nullglob
rpms=("$packages"/*.rpm)
[[ ${#rpms[@]} -eq 1 ]] || fail "expected one .rpm in $packages, found ${#rpms[@]}"
package=${rpms[0]}
version=$(workspace_version "$repo")

# Fedora's container image installs without documentation and without weak
# dependencies. A host installs with both, and a check that asserts what the
# package ships has to see all of it.
dnf_options=(-y --setopt=tsflags= --setopt=install_weak_deps=True)
config=$(dnf "${dnf_options[@]}" --dump-main-config)
if grep -Eq '^tsflags *=.*nodocs' <<<"$config"; then
    fail "documentation is still excluded from the install"
fi
pass "the install includes documentation"
grep -Eq '^install_weak_deps *= *(1|[Tt]rue)$' <<<"$config" ||
    fail "weak dependencies are still excluded from the install"
pass "the install pulls weak dependencies, as a host's does"
expect "rpm excludes no documentation" "" "$(rpm --eval '%{?_excludedocs:%{_excludedocs}}' | sed 's/^0$//')"

expect "the package is dclip, at the workspace version, release 1, for x86_64" \
    "dclip $version 1 x86_64" \
    "$(rpm -qp --qf '%{NAME} %{VERSION} %{RELEASE} %{ARCH}' "$package")"
expect "the License tag is Apache-2.0" "Apache-2.0" "$(rpm -qp --qf '%{LICENSE}' "$package")"
expect "the package requires wl-clipboard and nothing else" \
    "wl-clipboard" "$(rpm -qp --requires "$package" | grep -v '^rpmlib(' | sort)"
expect "the package recommends xsel and nothing else" "xsel" "$(rpm -qp --recommends "$package")"
for relation in suggests supplements enhances conflicts obsoletes; do
    expect "the package declares no $relation" "" "$(rpm -qp "--$relation" "$package")"
done
expect "the package has no scriptlets" "" "$(rpm -qp --scripts "$package")"
expect "the package has no triggers" "" "$(rpm -qp --triggers "$package")"

before=$(rpm -qa --qf '%{NAME}\n' | sort)
dnf "${dnf_options[@]}" install "$package"
new=$(added "$before" "$(rpm -qa --qf '%{NAME}\n' | sort)")
printf 'Installed with dclip:\n%s\n' "$new"
grep -qx wl-clipboard <<<"$new" || fail "the install did not pull wl-clipboard"
pass "the install pulled wl-clipboard"
grep -qx xsel <<<"$new" || fail "the install did not pull xsel by default"
pass "the install pulled xsel by default"
if grep -qi python <<<"$new"; then
    fail "the install pulled Python: $(grep -i python <<<"$new" | tr '\n' ' ')"
fi
pass "the install pulled no Python"

expect "dclip owns exactly its files and directories" \
    "/usr/bin/dclip
/usr/share/dclip
/usr/share/dclip/bin
/usr/share/dclip/bin/wl-paste
/usr/share/dclip/bin/xclip
/usr/share/dclip/bridge.py
/usr/share/doc/dclip
/usr/share/doc/dclip/README.md
/usr/share/licenses/dclip
/usr/share/licenses/dclip/LICENSE
/usr/share/licenses/dclip/NOTICE" \
    "$(rpm -ql dclip | sort)"
expect_file "the host binary" /usr/bin/dclip 755
expect_dir "the shim's directory" /usr/share/dclip
expect_dir "the shim's bin" /usr/share/dclip/bin
expect_file "the shim" /usr/share/dclip/bridge.py 755 "$repo/bridge.py"
expect_link "the shim's wl-paste" /usr/share/dclip/bin/wl-paste ../bridge.py
expect_link "the shim's xclip" /usr/share/dclip/bin/xclip ../bridge.py
expect_dir "the documentation directory" /usr/share/doc/dclip
expect_file "the documentation" /usr/share/doc/dclip/README.md 644 "$repo/README.md"
expect_dir "the license directory" /usr/share/licenses/dclip
expect_file "the license" /usr/share/licenses/dclip/LICENSE 644 "$repo/LICENSE"
expect_file "the notice" /usr/share/licenses/dclip/NOTICE 644 "$repo/NOTICE"
for path in /usr/share/licenses/dclip /usr/share/licenses/dclip/LICENSE /usr/share/licenses/dclip/NOTICE; do
    expect "the package database names dclip as the owner of $path" \
        "dclip" "$(rpm -qf --qf '%{NAME}\n' "$path")"
done
expect "rpm -qL lists exactly the two license files" \
    "/usr/share/licenses/dclip/LICENSE
/usr/share/licenses/dclip/NOTICE" \
    "$(rpm -qL dclip | sort)"
expect "the installed files match the package database" "" "$(rpm -V dclip)"

usage=$( (dclip 2>&1 >/dev/null; echo "status $?") || true)
expect "dclip with no arguments prints the usage line and exits 1" \
    "dclip: Usage: dclip serve [--sync-text] [--allow-uid UID]
status 1" "$usage"

# The packaged layout end to end, as a user with no data directory yet. The
# tools this needs are installed only now, after the dependency assertions.
dnf "${dnf_options[@]}" install python3 shadow-utils util-linux
useradd --create-home tester
home=/home/tester
data=$home/.local/share/dclip
expect_gone "the user has no DClip data directory before serve" "$data"
# A command array rather than a function, so that the background job below
# execs into serve and $! is serve's own PID.
as_tester=(setpriv --reuid=tester --regid=tester --init-groups
    env -i HOME="$home" PATH=/usr/bin:/bin)
"${as_tester[@]}" /usr/bin/dclip serve >/tmp/serve.out 2>/tmp/serve.err &
serve=$!
for _ in $(seq 100); do
    [[ -S $data/clipboard.sock ]] && break
    kill -0 "$serve" 2>/dev/null || fail "serve exited: $(cat /tmp/serve.err)"
    sleep 0.1
done
[[ -S $data/clipboard.sock ]] || fail "serve bound no socket at $data/clipboard.sock"
pass "serve creates the data directory and binds its socket there"
expect "serve creates the data directory at mode 755" "755" "$(stat -c %a "$data")"
listing=$( ("${as_tester[@]}" DCLIP_SOCKET="$data/clipboard.sock" /usr/share/dclip/bin/wl-paste --list-types 2>&1; echo "status $?") || true)
expect "the packaged wl-paste reaches serve, which reports its own error for the missing Wayland session" \
    "wl-paste: wl-paste failed; check it on Fedora
status 1" "$listing"
kill -TERM "$serve"
status=0
wait "$serve" || status=$?
expect "serve exits 0 on SIGTERM" "0" "$status"
expect_gone "serve removes its socket on SIGTERM" "$data/clipboard.sock"

dnf -y remove dclip
expect_status "dclip is no longer installed" 1 rpm -q --quiet dclip
expect_gone "removing dclip removes every file and directory it installed" \
    /usr/bin/dclip /usr/share/dclip /usr/share/doc/dclip /usr/share/licenses/dclip

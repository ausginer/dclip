# Assertions for the install checks, sourced by check-rpm.sh and check-deb.sh.
# Each names the property it checks, so a failing log says which promise of
# the package broke, and a passing log lists every promise that held.

fail() {
    printf 'FAIL: %s\n' "$*" >&2
    exit 1
}

pass() {
    printf 'ok: %s\n' "$1"
}

# expect PROPERTY EXPECTED ACTUAL
expect() {
    [[ "$2" == "$3" ]] || fail "$1"$'\n'"  expected: ${2@Q}"$'\n'"  actual:   ${3@Q}"
    pass "$1"
}

# expect_file PROPERTY PATH MODE [TRACKED]: a root-owned regular file of MODE,
# byte-identical to TRACKED when one is given.
expect_file() {
    [[ -f "$2" && ! -L "$2" ]] || fail "$1: $2 is not a regular file"
    expect "$1: $2 is mode $3, owned by root" "$3 root:root" "$(stat -c '%a %U:%G' "$2")"
    if [[ $# -ge 4 ]]; then
        # By digest, because coreutils is in every check image and cmp is not.
        [[ "$(sha256sum <"$4")" == "$(sha256sum <"$2")" ]] ||
            fail "$1: $2 differs from the tracked $(basename "$4")"
        pass "$1: $2 is byte-identical to the tracked $(basename "$4")"
    fi
}

# expect_dir PROPERTY PATH: a root-owned directory of mode 0755.
expect_dir() {
    [[ -d "$2" && ! -L "$2" ]] || fail "$1: $2 is not a directory"
    expect "$1: $2 is mode 755, owned by root" "755 root:root" "$(stat -c '%a %U:%G' "$2")"
}

# expect_link PROPERTY PATH TARGET: a symbolic link whose target is TARGET as
# written, not as resolved.
expect_link() {
    [[ -L "$2" ]] || fail "$1: $2 is not a symbolic link"
    expect "$1: $2 links to $3" "$3" "$(readlink "$2")"
}

# expect_gone PROPERTY PATH...: none of the paths exists, as a link or
# otherwise.
expect_gone() {
    local property=$1 path
    shift
    for path in "$@"; do
        [[ ! -e "$path" && ! -L "$path" ]] || fail "$property: $path remains"
    done
    pass "$property"
}

# expect_status PROPERTY STATUS COMMAND...: COMMAND exits with STATUS.
expect_status() {
    local property=$1 expected=$2 status=0
    shift 2
    "$@" || status=$?
    expect "$property" "$expected" "$status"
}

# The workspace version, the one `version` line in the tracked Cargo.toml.
workspace_version() {
    local version
    version=$(sed -n 's/^version = "\(.*\)"$/\1/p' "$1/Cargo.toml")
    [[ -n "$version" ]] || fail "no workspace version in $1/Cargo.toml"
    printf '%s' "$version"
}

# added BEFORE AFTER: the lines of AFTER that BEFORE lacks.
added() {
    comm -13 <(printf '%s\n' "$1") <(printf '%s\n' "$2")
}

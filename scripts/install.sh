#!/usr/bin/env bash
# Install this framework tree into ~/.swiftflow so projects can find it
# without vendoring a copy.
#
#   scripts/install.sh            # install VERSION as a copy, make it current
#   scripts/install.sh --dev      # link `dev` at this working tree
#   scripts/install.sh --list     # what is installed
#   scripts/install.sh --uninstall 0.1.0
#
# # Why a home directory rather than a path in each project
#
# A project used to carry the whole framework in `.swiftflow/`, which
# made "which SwiftFlow is this app built against" a property of a
# directory nobody looked at, and made upgrading a copy-paste. It also
# meant every project that built for a new triple paid for its own Rust
# build — and that output is over 11 GB across the triples this supports,
# against about 5 MB of source.
#
# So a version is source, installed once, and its build output lives in
# `cache/` — reached through the version's own `rust/target`, which is a
# symlink. Every project on the machine shares one build per (version,
# triple), and the path the Swift manifest resolves is the ordinary
# in-tree one, so nothing has to be told where to look.
#
# # What a project pins
#
# `[swiftflow] version` in the project's `SwiftFlow.toml`, holding either
# a version number or `dev`. Unset means `current`. The app's
# Package.swift resolves that against `$SWIFTFLOW_HOME` at
# manifest-evaluation time — the same trick the framework already uses
# for SWIFTFLOW_PLATFORM, and the reason none of this needs a code
# generator.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
HOME_DIR="${SWIFTFLOW_HOME:-$HOME/.swiftflow}"
VERSION="$(tr -d ' \t\n\r' < "$ROOT/VERSION")"

usage() {
    sed -n '2,30p' "$0" | sed 's/^# \{0,1\}//'
    exit "${1:-0}"
}

# Everything an installed version needs, and nothing derived. Listed
# rather than globbed so a stray directory in the working tree — a
# scratch checkout, an editor's cache — can never end up installed.
CONTENTS=(VERSION Package.swift Cargo.toml cli Sources apple desktop android \
          macros rust tools scripts README.md ARCHITECTURE.md)

copy_tree() {
    local dest="$1"
    rm -rf "$dest"
    mkdir -p "$dest"
    # Through tar rather than `cp -R`, so the excludes apply *during* the
    # walk. Copying everything and deleting after is the obvious version
    # and it is unusable: `rust/target` alone is over 11 GB against about
    # 5 MB of source, so the install spent minutes writing bytes it was
    # about to remove. Derived output is never installed anyway — it is
    # per-triple, and `cache/` is where it belongs so projects share it.
    local present=()
    for item in "${CONTENTS[@]}"; do
        [ -e "$ROOT/$item" ] && present+=("$item")
    done
    ( cd "$ROOT" && tar cf - \
        --exclude=target \
        --exclude=.build \
        --exclude=.DS_Store \
        --exclude='*:Zone.Identifier' \
        "${present[@]}" ) | ( cd "$dest" && tar xf - )
}

point_current_at() {
    ln -sfn "versions/$1" "$HOME_DIR/current"
}

case "${1:---install}" in
--help | -h)
    usage
    ;;

--list)
    if [ ! -d "$HOME_DIR/versions" ]; then
        echo "nothing installed in $HOME_DIR"
        exit 0
    fi
    current="$(readlink "$HOME_DIR/current" 2>/dev/null | sed 's|^versions/||')"
    for dir in "$HOME_DIR"/versions/*; do
        [ -e "$dir" ] || continue
        name="$(basename "$dir")"
        marker=" "
        [ "$name" = "$current" ] && marker="*"
        if [ -L "$dir" ]; then
            echo "$marker $name -> $(readlink "$dir")"
        else
            echo "$marker $name"
        fi
    done
    ;;

--uninstall)
    target="${2:-}"
    [ -n "$target" ] || { echo "which version?" >&2; exit 2; }
    [ -e "$HOME_DIR/versions/$target" ] || { echo "$target is not installed" >&2; exit 1; }
    # `rm -rf` on a symlink removes the link, not the tree behind it —
    # which is what makes uninstalling `dev` safe rather than a way to
    # delete your working copy.
    rm -rf "$HOME_DIR/versions/$target"
    if [ "$(readlink "$HOME_DIR/current" 2>/dev/null)" = "versions/$target" ]; then
        rm -f "$HOME_DIR/current"
        echo "removed $target, which was current — nothing is current now"
    else
        echo "removed $target"
    fi
    ;;

--dev)
    mkdir -p "$HOME_DIR/versions"
    ln -sfn "$ROOT" "$HOME_DIR/versions/dev"
    echo "dev -> $ROOT"
    echo
    echo "Pin a project to it with:  [swiftflow] version = \"dev\" in SwiftFlow.toml"
    echo "Edits to this tree are picked up with no reinstall."
    ;;

--install | "")
    mkdir -p "$HOME_DIR/versions" "$HOME_DIR/cache" "$HOME_DIR/bin"
    dest="$HOME_DIR/versions/$VERSION"
    if [ -L "$dest" ]; then
        echo "$VERSION is a symlink (dev-style); refusing to overwrite it" >&2
        exit 1
    fi
    copy_tree "$dest"

    # Build output lives in cache/ and is *reached* through the version's
    # own rust/target. That indirection is the whole trick: the path the
    # Swift manifest resolves never changes and needs no environment, so
    # a cached manifest evaluation, or a build running two processes deep
    # through xtool, can't lose it — while reinstalling this version still
    # keeps the 11 GB it took to produce.
    mkdir -p "$HOME_DIR/cache/rust/$VERSION"
    ln -sfn "$HOME_DIR/cache/rust/$VERSION" "$dest/rust/target"

    point_current_at "$VERSION"
    echo "installed $VERSION -> $dest"
    echo "current -> $VERSION"
    ;;

*)
    echo "unknown option: $1" >&2
    usage 2
    ;;
esac

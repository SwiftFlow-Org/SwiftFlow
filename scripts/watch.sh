#!/usr/bin/env bash
set -e

# Simple hot reload: watches Rust and Swift source files and re-runs the
# full build pipeline on every save. Not a true in-process hot-swap — the
# Rust side is statically linked into the app binary, so there's no way to
# swap it into a running process. This just automates the rebuild.
#
#   watch.sh --desktop   (default)  → swiftflow run --platform desktop
#   watch.sh --ios                  → swiftflow run --platform ios
#
# Desktop is the default because it's the faster loop: no device, no
# install step, and the window comes straight back up.
#
# Known limitation: entr snapshots the watched file list once at startup
# (via find). A newly *created* .rs/.swift file won't be picked up until
# this script is restarted — editing existing files works immediately.

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TARGET="desktop"

while [ $# -gt 0 ]; do
    case "$1" in
        --desktop) TARGET="desktop" ;;
        --ios)     TARGET="ios" ;;
        -h|--help)
            echo "usage: watch.sh [--desktop | --ios]"
            exit 0
            ;;
        *)
            echo "✗ Unknown option: $1"
            echo "usage: watch.sh [--desktop | --ios]"
            exit 1
            ;;
    esac
    shift
done

if ! command -v entr >/dev/null 2>&1; then
    echo "✗ entr is not installed."
    echo "  macOS: brew install entr"
    echo "  Linux: sudo apt install entr"
    exit 1
fi

# Each target watches its own platform package, so editing the iOS host
# doesn't trigger a desktop rebuild that couldn't be affected by it. The
# app sources are the same files on both — one app, two hosts.
APP_SOURCES="${SWIFTFLOW_APP_DIR:-$PWD}/Sources"
case "$TARGET" in
    desktop) PLATFORM_SOURCES="$ROOT/desktop/Sources" ;;
    ios)     PLATFORM_SOURCES="$ROOT/apple/Sources" ;;
esac

# The build itself is `swiftflow`, not a script — this only decides *when*
# to run one. entr needs a command with no shell in it, hence the array.
if ! command -v swiftflow >/dev/null 2>&1; then
    echo "✗ swiftflow is not on PATH."
    echo "  cargo install --git https://github.com/celymyst/SwiftFlow"
    exit 1
fi

echo "▶ Watching for changes — rebuilding $TARGET on save (ctrl-c to stop)"

# `2>/dev/null` on find so a missing app directory (a checkout without
# one of the two test apps) degrades to watching the rest rather than
# failing outright.
find \
    "$ROOT/rust/swiftflow_core" \
    "$ROOT/rust/swiftflow_wgpu" \
    "$ROOT/rust/swiftflow_desktop" \
    "$ROOT/Sources" \
    "$PLATFORM_SOURCES" \
    "$APP_SOURCES" \
    \( -name '*.rs' -o -name '*.swift' -o -name '*.wgsl' -o -name '*.h' \) \
    2>/dev/null \
    | entr -c swiftflow run --platform "$TARGET"

#!/bin/sh
# Builds and runs the checks against the Rust library in $SWIFTFLOW_RUST_LIB_DIR
# (by default SwiftFlow's own release build).
set -e
cd "$(dirname "$0")"
: "${SWIFTFLOW_RUST_LIB_DIR:=$PWD/../../rust/target/release}"
export SWIFTFLOW_PLATFORM=desktop SWIFTFLOW_RUST_LIB_DIR
swift build --product LayoutChecks
LD_LIBRARY_PATH="$SWIFTFLOW_RUST_LIB_DIR" ./.build/debug/LayoutChecks

#!/bin/sh
# Xcode build phase: SwiftFlow asset catalogue -> the flat folder the
# runtime reads.
#
# Add as a Run Script phase above "Copy Bundle Resources":
#
#   "$SRCROOT/../.swiftflow/tools/flatten-assets.sh"
#
# Two settings this needs, both of which will otherwise fail silently or
# confusingly:
#
#   - Build Settings -> User Script Sandboxing -> No. This reads the
#     catalogue from $SRCROOT and writes into $BUILT_PRODUCTS_DIR, and
#     sandboxing denies both.
#   - Uncheck "Based on dependency analysis" on the phase, so it runs
#     every build instead of being skipped for declaring no outputs.
#
# Nothing needs doing about the catalogue's target membership. In an
# Xcode 16 synchronized-folder project it is a member automatically and
# can't simply be excluded, so actool will compile a redundant
# Assets.car alongside this — which is harmless, since this script reads
# the *source* catalogue and nothing ever opens the .car.
#
# Override either path with SF_CATALOGUE / SF_ASSETS_OUT if your layout
# differs from the default below.
set -e

here=$(cd "$(dirname "$0")" && pwd)

CATALOGUE="${SF_CATALOGUE:-$SRCROOT/SwiftFlowApp/Assets.xcassets}"
# Inside the built .app by default, which is where Bundle.main
# resourceURL points and therefore where AssetCatalog.load looks.
OUT="${SF_ASSETS_OUT:-$BUILT_PRODUCTS_DIR/$UNLOCALIZED_RESOURCES_FOLDER_PATH/Assets}"

if [ ! -d "$CATALOGUE" ]; then
    echo "warning: no SwiftFlow catalogue at $CATALOGUE — skipping asset flatten"
    exit 0
fi

# Deliberately does NOT build the tool on demand. A build phase runs
# under user-script sandboxing with no reach into ~/.cargo and no
# network, so a cargo invocation here fails in a way that reads like a
# SwiftFlow bug rather than a missing one-time setup step.
bin="$here/target/release/sf-assets"
[ -x "$bin" ] || bin="$here/target/debug/sf-assets"
if [ ! -x "$bin" ]; then
    echo "error: sf-assets is not built. Run this once, outside Xcode:"
    echo "error:   cd ${here} && cargo build --release -p swiftflow_assets --bin sf-assets"
    exit 1
fi

# "warning:"/"error:" prefixes are what Xcode scrapes out of a build
# phase's stdout into the issue navigator, which is the only place most
# people will ever look — sf-assets already emits them for empty sets
# and missing files.
exec "$bin" flatten "$CATALOGUE" "$OUT"

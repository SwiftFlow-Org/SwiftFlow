#!/bin/sh
# Refresh the vendored Phosphor Icons font faces and name table.
#
#   tools/fetch-phosphor.sh [version]
#
# Then regenerate the Swift catalogue:
#
#   python3 tools/generate-icons.py
#
# Both the TTFs and the name table are committed rather than fetched at
# build time — the build must work offline, and a font that can change
# underneath a release is a font that can silently renumber every icon in
# the app.
#
# Duotone is deliberately not vendored: it is two overlapping glyphs in
# two colours, and every draw path in this renderer carries one colour
# per glyph. Supporting it needs a second instance and a second tint, not
# a sixth file here.

set -eu

VERSION="${1:-2.1.2}"
PKG="@phosphor-icons/web"
DEST="$(cd "$(dirname "$0")/.." && pwd)/rust/swiftflow_core/fonts/phosphor"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

echo "fetching $PKG@$VERSION"
curl -fsSL -o "$TMP/web.tgz" \
    "https://registry.npmjs.org/@phosphor-icons/web/-/web-$VERSION.tgz"
tar xzf "$TMP/web.tgz" -C "$TMP"

SRC="$TMP/package/src"
mkdir -p "$DEST"

cp "$SRC/thin/Phosphor-Thin.ttf"       "$DEST/"
cp "$SRC/light/Phosphor-Light.ttf"     "$DEST/"
cp "$SRC/regular/Phosphor.ttf"         "$DEST/"
cp "$SRC/bold/Phosphor-Bold.ttf"       "$DEST/"
cp "$SRC/fill/Phosphor-Fill.ttf"       "$DEST/"
cp "$TMP/package/LICENSE"              "$DEST/LICENSE"

# The names live only in the stylesheet — the font itself carries
# codepoints and nothing else. Regular is the source of truth; every
# non-duotone weight ships the identical mapping.
python3 - "$SRC/regular/style.css" "$DEST/icons.tsv" "$VERSION" <<'PY'
import re, sys
css, out, version = sys.argv[1], sys.argv[2], sys.argv[3]
pattern = r'\.ph[\w-]*\.ph-([a-z0-9-]+):before\s*\{\s*content:\s*"\\([0-9a-fA-F]+)"'
pairs = sorted(set(re.findall(pattern, open(css).read())))
if len(pairs) < 1000:
    sys.exit(f"only {len(pairs)} icons parsed out of {css} — the stylesheet's shape changed")
lines = [
    f"# Phosphor Icons name -> codepoint, from @phosphor-icons/web {version} (MIT).",
    "# Regenerate with tools/fetch-phosphor.sh; consumed by tools/generate-icons.py.",
    "# name\tcodepoint",
]
lines += [f"{n}\t{c.upper()}" for n, c in pairs]
open(out, "w").write("\n".join(lines) + "\n")
print(f"wrote {out}: {len(pairs)} names")
PY

echo
echo "vendored into $DEST"
echo "next: python3 tools/generate-icons.py"

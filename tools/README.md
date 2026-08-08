# SwiftFlow developer tools

A separate Cargo workspace from `../rust/`, which builds the shipping
runtime. Keeping them apart means the GUI's dependency tree never lands
in the lockfile the iOS/Android build resolves against.

| Crate | What it is |
|---|---|
| `swiftflow_assets` | The catalogue format and its operations. No GUI dependencies, so it's testable without a display. Ships the `sf-assets` CLI. |
| `swiftflow_assets_editor` | The cross-platform catalogue editor (eframe/egui). |

## Why not `.xcassets`?

It *is* `.xcassets` — the uncompiled form.

The thing SwiftFlow can't use is `Assets.car`, the archive `actool`
compiles a catalogue into: only `UIImage(named:)` can read it, so
depending on it would tie every image in the framework to UIKit and
therefore to Apple platforms. But an `.xcassets` *before* compilation is
just a directory of `Name.imageset/Contents.json` files sitting next to
ordinary PNGs — completely portable.

So the catalogue is authored in that format and simply never compiled:

- **On macOS**, open the folder in Xcode and you get Xcode's own asset
  catalogue editor. Not an imitation — the real one, because it's the
  real format.
- **Anywhere else**, `swiftflow_assets_editor` gives the same wells,
  scale slots and drag-and-drop.
- **At build time**, `sf-assets flatten` emits the flat
  `name@Nx.png` layout that `AssetCatalog.load` resolves with Foundation
  alone.

Both editors write byte-identical `Contents.json` (Xcode's two-space,
`" : "` style), so alternating between them produces no diff noise, and
keys this editor doesn't model — `appearances`, `subtype`, and so on —
are preserved untouched rather than dropped.

## Editing a catalogue

```sh
cargo run -p swiftflow_assets_editor -- path/to/Assets.xcassets
```

The path is optional; the toolbar has an open button, and dropping a
folder onto the window opens it. Drop image files onto the 1x / 2x / 3x
wells to fill them.

## Wiring it into an Xcode build

1. Keep `Assets.xcassets` in the project so Xcode's editor can open it,
   but **remove it from target membership**. Xcode picks the editor from
   the file type, so it still opens normally — membership only decides
   whether `actool` compiles it, and here it must not.
2. Add a Run Script phase *before* "Copy Bundle Resources":

   ```sh
   "$SRCROOT/../.swiftflow/tools/flatten-assets.sh"
   ```

   Override `SF_CATALOGUE` / `SF_ASSETS_OUT` if your layout differs from
   the defaults.

The script builds `sf-assets` on first run and reuses it after. Missing
files and empty sets come out as Xcode-visible warnings rather than
failing the build — a half-finished asset is a normal state to leave a
project in.

## CLI

```sh
sf-assets list <catalogue>                # each set and which scales are filled
sf-assets flatten <catalogue> <out-dir>   # emit the runtime layout
```

## Tests

```sh
cargo test -p swiftflow_assets
```

The one worth knowing about is `xcode_json_round_trips_byte_for_byte`,
which parses a `Contents.json` copied verbatim out of a catalogue Xcode
wrote and asserts it re-serializes unchanged. If that fails, the two
editors have started fighting over the file.

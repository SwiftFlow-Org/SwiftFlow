# SwiftFlow

A SwiftUI-inspired declarative UI framework for iOS. Swift provides the
declarative API surface; a Rust core handles layout, the node tree, and
text rasterization; rendering is done with wgpu/Metal.

## Layout

This repository is the framework. It installs into `~/.swiftflow`, and
projects find it from there rather than vendoring a copy.

```
VERSION          the installed version's number, and the source of truth for it
Package.swift    SwiftFlowCore — the shared Swift API plus the C header
Sources/         SwiftFlowCore + CSwiftFlow
apple/           SwiftFlowApple  — the iOS/macOS host package
desktop/         SwiftFlowDesktop — the winit host package
android/         SwiftFlowAndroid — the GameActivity host package
rust/            Cargo workspace: layout, text rasterization, wgpu renderer
cli/             the swiftflow CLI — new, run, build, doctor
tools/           the asset-catalogue CLI and editor (a separate workspace)
scripts/         install.sh, and watch.sh for a rebuild-on-save loop
```

The three host packages exist so an app swaps a *dependency* rather than
its code: each exports its product as `SwiftFlow`, so `import SwiftFlow`
and `@main struct App: SwiftFlowApp` are identical on every platform.

## The CLI

```
cargo install --git https://github.com/SwiftFlow-Org/SwiftFlow swiftflow 
```

### CLI usage

```
swiftflow new <name> [--pin 0.1.0]   create a project
swiftflow run [-p ios|desktop|android]
swiftflow build [-p …]
swiftflow doctor                     what is installed, and what this project resolves to
```

`new` pins the version that is current at the time, explicitly, because
`current` moves. A project that says nothing keeps building against
whatever was installed last.

`run` and `build` find the project by walking up for a `Package.swift`,
so they work from anywhere inside it. They resolve the pin **the same way
the app's manifest does**, and they have to: SwiftPM resolves the package
dependency itself, so if the two disagreed the CLI would build one
framework's Rust and link another's Swift.

The builds are Rust calling cargo, swift, xtool, gradle and adb
directly — there are no shell scripts on this path. That matters beyond
tidiness: the Android preflights alone are five checks that each exist
because something failed on a device in a way that named the wrong
culprit (a Swift SDK with no NDK sysroot reads as a broken header in this
repo; a toolchain half a version off reads as a corrupt module). As shell
they were unreachable from a test. As Rust they are functions.

## Installing the framework


```
scripts/install.sh          # install VERSION as a copy, and make it current
scripts/install.sh --dev    # link `dev` at this working tree
scripts/install.sh --list
```

```
~/.swiftflow/
  versions/0.1.0/         an installed release — source, about 5 MB
    rust/target -> ../../cache/rust/0.1.0
  versions/dev -> …       a symlink to a framework working tree
  current -> versions/0.1.0
  cache/rust/0.1.0/       build output, shared by every project
```

Build output is not part of a version — it is over 11 GB across the
triples this supports, against about 5 MB of source — but it is *reached*
through the version's own `rust/target`, which is a symlink into `cache/`.

That indirection matters more than it looks. An installed version is
already one directory shared by every project, so `rust/target` has the
sharing property on its own; the symlink only means reinstalling a
version doesn't throw the build away. What it buys is that **the path the
Swift manifest resolves is the ordinary in-tree one**, so nothing has to
be told where to look. Pointing the build somewhere else and passing the
location in the environment is what an earlier version did, and it failed
with `library 'swiftflow_wgpu' not found`: SwiftPM caches evaluated
manifests, and an iOS build runs two processes deep through xtool, so a
variable that arrives once may not arrive again.

## Using it from a project

A project is described by one `SwiftFlow.toml` at its root. The file is
optional — every field has a default and an app with no config still
builds — but it is where the pin, the bundle identifier, the permissions
and the per-platform knobs live.

```toml
[swiftflow]
version = "dev"            # an installed release, `dev`, or omit for `current`

[app]
name = "Reader"
id   = "com.acme.reader"   # reverse-DNS; the bundle ID and the applicationId
version = "1.0.0"
build   = 1

[capabilities]
camera = { reason = "Scan a page to import it" }

[android]
min_sdk = 26

[ios]
deployment_target = "17.0"

[android.manifest]
"android:hardwareAccelerated" = true
```

### The three tiers

**Canonical** — `[app]`, `[capabilities]` — is anything that means the
same thing everywhere. An app has one identifier, one display name, one
version; that they are spelled `CFBundleIdentifier` on one platform and
`applicationId` on another is the CLI's problem, not yours. A capability
is declared once and lowers to an `NSCameraUsageDescription` on iOS and
an `android.permission.CAMERA` on Android.

**Typed platform** — `[ios]`, `[android]`, `[desktop]` — is for concepts
that genuinely do not generalise. `min_sdk` has no iOS meaning and
`deployment_target` has no Android one, so pretending they were the same
field would only make both wrong.

**Raw passthrough** — `[ios.plist]`, `[android.manifest]` — is merged
verbatim and wins last. It exists so a key nobody anticipated never
becomes a reason to abandon the file, which is what lets the canonical
vocabulary stay small instead of growing to cover every attribute either
platform has ever shipped.

Precedence is **raw > typed > canonical**: whatever you wrote closer to
the metal wins over whatever was inferred for you.

Unknown keys are an error, not a no-op. A misspelled `min_sdkk` that
silently does nothing is the worst kind of config bug — it looks set —
so the parser rejects it and names the file, line and column. An unknown
*capability* is rejected the same way, and the error lists the ones that
exist.

Some declared fields are parsed and validated but not yet lowered to
anything (app icons, `.desktop` files). `swiftflow doctor` lists them
rather than letting them look effective.

### The pin

`[swiftflow] version` holds either a version number or `dev`; omitting it
means `current`.

The app's `Package.swift` resolves that against `$SWIFTFLOW_HOME` (default
`~/.swiftflow`) when the manifest runs, and depends on
`<root>/apple`, `<root>/desktop` or `<root>/android` according to
`SWIFTFLOW_PLATFORM`. A manifest runs on the host and can read the
filesystem, so this needs no code generation step and no absolute path
committed anywhere.

Building:

```
cd myapp && swiftflow run
```

### The Rust workspace

```
  rust/                     Cargo workspace — the platform-agnostic rendering/layout core
    swiftflow_core/           layout/text/rasterization logic, zero platform deps
      src/
        layout/                 node tree + layout pass
        render/                  draw pass + draw command output
        text/                    font system, glyph extraction, rasterizer
        ffi/                     FFI-boundary types, backend trait, entry points, logging
    swiftflow_wgpu/           wgpu-based GPU backend, depends on swiftflow_core
    swiftflow_desktop/        winit host for macOS/Linux/Windows — owns the event loop
    swiftflow_android/        winit host for Android — same contract, plus Choreographer and JNI metrics
  Package.swift             the shared core package (product "SwiftFlowCore")
  Sources/
    CSwiftFlow/               C shim exposing the Rust FFI header to Swift
    SwiftFlowCore/            the pure, platform-agnostic framework, grouped by responsibility:
      Core/                     View protocol, type erasure, ViewBuilder
      State/                    @State-style state management
      App/                      SwiftFlowApp protocol, Scene, SceneBuilder — the parts every platform shares
      Views/                    concrete view types (Button, Text, Shapes, Stacks, Color)
      Modifiers/                the view modifier system
      Rendering/                node-tree building, tap dispatch, device scale
      Logging/                  cross-platform logging (Log for app code, SFLog for framework internals)
  apple/                    nested local package — iOS platform glue (product "SwiftFlow")
    Package.swift             depends on the shared core, re-exports it
    Sources/SwiftFlow/
      MetalView.swift          UIView + CAMetalLayer + CADisplayLink + touch handling
      SwiftFlowApp.swift       UIApplicationMain bootstrap, the hidden AppDelegate
  desktop/                  same shape — DesktopHost.swift, driven by swiftflow_desktop
  android/                  same shape — AndroidHost.swift, driven by swiftflow_android
```

All three platform packages export their product as `"SwiftFlow"`, which
is what lets an app swap hosts without touching a line of its own code.

Module names on the Rust side (`crate::ffi`, `crate::font`, etc.) are
unaffected by the subfolder layout — `lib.rs` uses `#[path = "..."]`
attributes so every existing `use crate::X` reference works unchanged.

See [ARCHITECTURE.md](ARCHITECTURE.md) for the invariants and design
decisions baked into this code. Read it before touching layout, text
rendering, or state.

## Building & running

```
swiftflow run                       # the host's default platform
swiftflow run --platform desktop
swiftflow run --platform android
swiftflow build                     # build without launching
```

Each does the same four steps in order, differing only in the toolchain
and who packages the result:

1. `cargo build --release` for the platform's triple, into the shared
   cache — one `-p` covers the native side, because the host crate pulls
   swiftflow_wgpu and swiftflow_core in as rlibs and its staticlib
   already carries their `sf_*` symbols.
2. Flatten `Assets.xcassets` into the target's `Assets/` with `sf-assets`.
3. Build the Swift half.
4. Package and launch.

### iOS

`xtool dev` builds, installs and runs on the attached device. Set
`SWIFTFLOW_IOS_LAUNCH_ID` to a bundle ID to also stream the device log
through `pymobiledevice3` after launch.

**`xtool.yml` lives only for the length of a build.** It is written from
`[app] id` in `SwiftFlow.toml` before xtool runs and removed after, on a
failed build as well as a successful one, so an app never has the file in
its tree. xtool reads it from beside `Package.swift`, which is why it is
written there rather than under `.build/`.

It stays gitignored anyway: a build killed mid-run leaves one behind, and
the next build recognises it by the `# Generated by` banner and reclaims
it. Deleting that banner makes the file yours — from then on the build
neither overwrites nor removes it, the same escape hatch a real `android/`
directory is on the other platform.

The `xtool/` directory is left alone. It holds signing and pairing state,
so clearing it every build would mean re-doing both.

- A physical iPhone attached via USB. From WSL2, run `attach_iphone.ps1`
  in an **elevated Windows PowerShell** first to pass the device through
  via `usbipd`.
- `xtool` and a Rust toolchain with the `aarch64-apple-ios` target.

### Desktop

Builds `swiftflow_desktop` for the host triple and `swift run`s the app.

### Android

Builds `swiftflow_android`, builds the Swift half through a Swift SDK for
Android into a `.so` with the Rust staticlib linked in, copies the
flattened assets into the APK, and hands off to Gradle.

**The Gradle project is generated**, into `.build/android`, so an app
doesn't carry one. It used to: about 200 lines across six files, of which
six *values* were the app's own — application id, label, library name,
root project name, min SDK, permissions — and the rest was SwiftFlow's
and identical everywhere. `hasCode=false` because there is no JVM code,
`useLegacyPackaging` because System.loadLibrary needs real files on disk,
`abiFilters` because an APK carrying the wrong architecture fails at load
rather than at build, an edge-to-edge theme because the navigation bar's
blur has to sit under the status bar. None of that is a decision an app
should store, or keep in step with the framework by hand.

The application id comes from `[app] id` in `SwiftFlow.toml`, falling back
to a hand-written `xtool.yml`'s `bundleID` and then to the app's name. The
min SDK follows `SWIFTFLOW_ANDROID_API`, so it cannot drift below the level
baked into the Swift SDK's triple.

An app that needs more — an extra permission, a service, a dependency —
puts a real `android/` directory beside `Package.swift` and it is used
verbatim, nothing generated. Copy `.build/android` there to start from
what generation would have produced.
`SWIFTFLOW_ANDROID_ABI=x86_64` builds for the emulator;
`SWIFTFLOW_ANDROID_API` picks the API level in the SDK triple.

Five preflight checks run before anything is built, and each exists
because something failed on a device in a way that named the wrong
culprit — a Swift SDK with no NDK sysroot reads as a broken header in
this repo, and a host toolchain half a version off the SDK reads as a
corrupt module. They report what to do rather than what broke.

Needs, and does not install:

- **Swift 6.3 or later on the host**, matching the SDK *exactly*. A
  `.swiftmodule` is only readable by the compiler version that wrote it,
  and the SDK ships a prebuilt Foundation — so a 6.2.3 host cannot use a
  6.3.3 SDK, in either direction. Official Android support starts at 6.3.
  (`swiftly install 6.3.3 && swiftly use 6.3.3`.) The script checks.
- **A Swift SDK for Android**, which is two steps, not one. `swift sdk
  install` from
  [swift.org](https://www.swift.org/documentation/articles/swift-sdk-for-android-getting-started.html),
  **and then the NDK setup inside the bundle** — it ships without a
  sysroot, because Google's NDK can't be redistributed:

  ```
  cd ~/Library/org.swift.swiftpm/swift-sdks/swift-*_android.artifactbundle
  export ANDROID_NDK_HOME=/path/to/android-ndk-r27d   # r27d or later
  ./scripts/setup-android-sdk.sh
  ```

  Skip it and the build fails with `'stdint.h' file not found`, which
  looks like a broken header in this repo. The script checks for it.

  The API level is *part of the target triple* and has to be one the
  installed SDK ships — swift.org's publishes **28**, which is why
  `minSdk` is 28 and `SWIFTFLOW_ANDROID_API` defaults to it. SwiftFlow's
  own floor is 24 (Choreographer).
- The Android SDK (`ANDROID_HOME`) and NDK (`ANDROID_NDK_HOME`), Gradle,
  and `adb` for the install step.

`--static-swift-stdlib` does **not** fully apply to a `.dynamicLibrary`
product — the built `.so` still carries `DT_NEEDED` on `libswiftCore.so`
— so the Swift runtime ships inside the APK. Point
`SWIFTFLOW_ANDROID_RUNTIME_DIR` at the SDK's directory containing
`libswiftCore.so`; the script copies every `.so` from there into
`jniLibs`, alongside the NDK's `libc++_shared.so`. Expect a 60–100MB APK.

`llvm-readelf -d <so> | grep NEEDED` on each shipped library, checked
against the APK's `lib/<abi>/` contents, catches a missing dependency in
one pass instead of one `dlopen` crash at a time.

Vulkan-capable devices only. That applies to emulators too — an AVD needs
an API 33+ system image with hardware acceleration, or it will start and
render nothing. Logs are `adb logcat -s SwiftFlow`.

Vulkan rather than GLES is a choice, not a hard limit; ARCHITECTURE.md
records what a GLES fallback would actually cost. It isn't worth it while
the Swift SDK's own floor (API 28, 64-bit only) already excludes the
devices that would need one.

## Notes for future readers

- The font is `rust/swiftflow_core/fonts/Inter.ttf`, a
  variable font (`wght` 100-900) compiled into the binary with
  `include_bytes!`. Nothing has to ship it alongside the app, which is
  why the Android APK carries no font asset.
- Why the module wiring is layered the way it is: `SwiftFlowTest`'s
  `import SwiftFlow` resolves to `apple/`'s `SwiftFlow` target,
  which itself does `@_exported import SwiftFlowCore` — that's what makes
  a single import expose both the pure framework and the platform-specific
  bootstrap. `desktop/` and `android/` follow the
  identical pattern: their own package beside it,
  exposing a product named `"SwiftFlow"` that re-exports the shared core.

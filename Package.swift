// swift-tools-version: 6.0
import PackageDescription
import Foundation

// Absolute path to this package root, resolved when the manifest runs.
// The linker's CWD is not guaranteed to be the package root, which is why
// the relative -L "search path not found" warning fired.
let packageDir = URL(fileURLWithPath: #filePath).deletingLastPathComponent().path

// Which Rust target directory to link against.
//
// A manifest can't see the target triple SwiftPM is building for, only
// the host it's running on — so cross-compiling (a Mac building for an
// iOS device, which is the normal case here) can't be detected and has
// to be stated. SWIFTFLOW_RUST_TRIPLE is that statement; the host
// default is what a plain desktop `swift build` wants.
//
//   device            -> aarch64-apple-ios
//   simulator (AS)    -> aarch64-apple-ios-sim
//   simulator (Intel) -> x86_64-apple-ios
//   macOS (AS)        -> aarch64-apple-darwin
//   Linux             -> x86_64-unknown-linux-gnu
let rustTriple: String = {
    if let explicit = ProcessInfo.processInfo.environment["SWIFTFLOW_RUST_TRIPLE"],
       !explicit.isEmpty
    {
        return explicit
    }
    #if os(macOS)
    #if arch(arm64)
    return "aarch64-apple-darwin"
    #else
    return "x86_64-apple-darwin"
    #endif
    #elseif os(Windows)
    return "x86_64-pc-windows-msvc"
    #else
    return "x86_64-unknown-linux-gnu"
    #endif
}()

// Where the Rust archives are. Normally beside the sources, but a
// shared build cache (~/.swiftflow/cache) puts them somewhere else
// entirely so every project on the machine reuses one build per
// (version, triple) — that output is over 11 GB across triples, against
// about 5 MB of source. The build scripts and the `swiftflow` CLI set
// this; a bare `swift build` falls back to the in-tree path.
let rustLibDir = ProcessInfo.processInfo.environment["SWIFTFLOW_RUST_LIB_DIR"]
    .flatMap { $0.isEmpty ? nil : $0 }
    ?? "\(packageDir)/rust/target/\(rustTriple)/release"

// Android can't be selected with `.when(platforms:)` — SwiftPM 6 has no
// `.android` SupportedPlatform case — so the one platform that needs a
// different archive and a different set of system libraries has to be
// chosen the same way the triple above is: from the environment, at
// manifest-evaluation time. build-android.sh sets both.
let isAndroid = ProcessInfo.processInfo.environment["SWIFTFLOW_PLATFORM"] == "android"

// Exactly one Rust archive is ever linked. A Rust staticlib embeds all
// its upstream crates, so libswiftflow_android.a already carries every
// sf_* symbol swiftflow_wgpu and swiftflow_core define, just as
// libswiftflow_desktop.a does — linking more than one would offer the
// same definitions twice.
//
//   log     — __android_log_write, which sflog! reaches on this platform
//   android — ANativeWindow_*, AChoreographer_*, AAssetManager_*
//   dl, m   — what the Rust staticlib itself needs
let androidLinkerSettings: [LinkerSetting] = [
    .unsafeFlags([
        "-L\(rustLibDir)",
        "-lswiftflow_android",
        "-llog",
        "-landroid",
        "-ldl",
        "-lm",
    ])
]

let package = Package(
    name: "SwiftFlowCore",
    platforms: [
        .iOS(.v17),
        .macOS(.v14),
    ],
    products: [
        .library(
            name: "SwiftFlowCore",
            targets: ["SwiftFlowCore"]
        )
    ],
    targets: [
        .target(
            name: "CSwiftFlow",
            path: "Sources/CSwiftFlow",
            publicHeadersPath: ".",
            linkerSettings: isAndroid ? androidLinkerSettings : [
                // iOS links the renderer directly. Desktop links the
                // winit host instead — and *only* that: a Rust staticlib
                // embeds all its upstream crates, so libswiftflow_desktop.a
                // already carries every sf_* symbol swiftflow_wgpu and
                // swiftflow_core define (verified with nm). Linking both
                // would offer the same definitions twice.
                .linkedLibrary("swiftflow_wgpu", .when(platforms: [.iOS])),
                .linkedLibrary(
                    "swiftflow_desktop",
                    .when(platforms: [.macOS, .linux, .windows])
                ),
                .unsafeFlags([
                    "-L\(rustLibDir)",
                ]),
                // Everything libswiftflow_desktop.a itself needs. Passed as
                // raw flags rather than .linkedFramework on purpose: the
                // -L above is already proving that unsafeFlags from this
                // target reach the executable's link line (the archive is
                // found — the failures were undefined symbols, not a
                // missing library), whereas .linkedFramework here did not
                // resolve them. Same channel, no propagation to assume.
                //
                //   Carbon              — TIS* / LMGetKbdType, winit's
                //                         keyboard-layout lookup
                //   CoreGraphics        — display and window geometry
                //   CoreVideo           — display link
                //   ApplicationServices — umbrella the above reach through
                //   AppKit/Metal/QuartzCore — winit's window, wgpu's device
                .unsafeFlags([
                    "-framework", "Carbon",
                    "-framework", "CoreGraphics",
                    "-framework", "CoreVideo",
                    "-framework", "ApplicationServices",
                    "-framework", "AppKit",
                    "-framework", "Metal",
                    "-framework", "QuartzCore",
                ], .when(platforms: [.macOS])),
                .linkedLibrary("dl", .when(platforms: [.linux])),
                .linkedLibrary("m", .when(platforms: [.linux])),
                .linkedLibrary("pthread", .when(platforms: [.linux])),
                // libc++ is an Apple-toolchain spelling; the Linux
                // equivalent comes in through the Rust staticlib's own
                // dependencies, so it must not be forced here.
                .unsafeFlags(["-lc++"], .when(platforms: [.iOS, .macOS])),
            ]
        ),
        .target(
            name: "SwiftFlowCore",
            dependencies: ["CSwiftFlow"],
            path: "Sources/SwiftFlowCore"
        ),
    ]
)

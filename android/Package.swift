// swift-tools-version: 6.0
import PackageDescription

// The Android platform layer.
//
// A sibling of ../apple and ../desktop rather than #if branches inside
// either, for the reason ../desktop/Package.swift already gives: the
// hosts share no code — one is driven by CADisplayLink and UIKit touches,
// one by a winit event loop and mouse events, this one by Choreographer
// and winit touches — and interleaving them would make all three harder
// to read. Everything worth sharing already lives in SwiftFlowCore.
//
// No `platforms:` here. SwiftPM 6 has no `.android` SupportedPlatform
// case, and none is needed: Android builds go through a Swift SDK
// (`--swift-sdk aarch64-unknown-linux-android24`), which supplies the
// triple. The same absence is why ../Package.swift branches on the
// SWIFTFLOW_PLATFORM environment variable rather than on
// `.when(platforms:)` for its link settings.
let package = Package(
    name: "SwiftFlowAndroid",
    products: [
        // Named "SwiftFlow" to match the apple and desktop packages, so
        // an app's `import SwiftFlow` is unchanged when it swaps hosts.
        // The app target is what becomes the .so Android loads; this is
        // an ordinary library linked into it.
        .library(
            name: "SwiftFlow",
            targets: ["SwiftFlow"]
        )
    ],
    dependencies: [
        .package(name: "SwiftFlowCore", path: "..")
    ],
    targets: [
        .target(
            name: "SwiftFlow",
            dependencies: [
                .product(name: "SwiftFlowCore", package: "SwiftFlowCore")
            ],
            // No linkerSettings here, same as ../desktop: every native
            // dependency belongs to libswiftflow_android.a and is
            // declared beside it in ../Package.swift's CSwiftFlow target,
            // so the -l and the system libraries it needs can't drift
            // apart or be ordered independently on the link line.
            path: "Sources/SwiftFlow"
        ),
    ]
)

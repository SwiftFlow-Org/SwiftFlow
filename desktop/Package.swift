// swift-tools-version: 6.0
import PackageDescription

// The desktop platform layer: macOS, Linux and Windows, all through the
// same winit host in swiftflow_desktop.
//
// Deliberately a sibling of ../apple rather than a set of #if branches
// inside it. The two hosts don't share a line of code — one is driven by
// CADisplayLink and UIKit touches, the other by a Rust-owned event loop
// and mouse events — so interleaving them would only make both harder to
// read. What they *do* share (all the frame logic worth sharing) already
// lives in SwiftFlowCore.
let package = Package(
    name: "SwiftFlowDesktop",
    platforms: [
        .macOS(.v14)
    ],
    products: [
        // Named "SwiftFlow" to match the apple package's product, so an
        // app's `import SwiftFlow` is unchanged when it swaps hosts.
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
            // No linkerSettings here on purpose: every native
            // dependency belongs to libswiftflow_desktop.a, and is
            // declared beside it in ../Package.swift's CSwiftFlow target
            // so the -l and the frameworks it needs can't drift apart or
            // be ordered independently on the link line.
            path: "Sources/SwiftFlow"
        ),
    ]
)

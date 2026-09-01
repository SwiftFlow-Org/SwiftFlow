// swift-tools-version: 6.0
import PackageDescription
import Foundation

// The desktop platform layer: macOS, Linux and Windows, all through the
// same winit host in swiftflow_desktop.
//
// Deliberately a sibling of ../apple rather than a set of #if branches
// inside it. The two hosts don't share a line of code — one is driven by
// CADisplayLink and UIKit touches, the other by a Rust-owned event loop
// and mouse events — so interleaving them would only make both harder to
// read. What they *do* share (all the frame logic worth sharing) already
// lives in SwiftFlowCore.
// Where the Rust archives/dylib are — the same resolution ../Package.swift
// does. With SwiftFlowCore now a *dynamic* product, CSwiftFlow's rust-link
// flags reach that dylib's link but no longer the executable's, and the host
// executable references the sf_desktop_* symbols directly (DesktopHost) plus
// sf_render_tree. So this target links the Rust dylib itself; at runtime it is
// the same single image SwiftFlowCore's dylib also links.
let rustTriple: String = {
    if let explicit = ProcessInfo.processInfo.environment["SWIFTFLOW_RUST_TRIPLE"],
       !explicit.isEmpty {
        return explicit
    }
    #if arch(arm64)
    return "aarch64-apple-darwin"
    #else
    return "x86_64-apple-darwin"
    #endif
}()

let packageDir = URL(fileURLWithPath: #filePath).deletingLastPathComponent().path
let rustLibDir = ProcessInfo.processInfo.environment["SWIFTFLOW_RUST_LIB_DIR"]
    .flatMap { $0.isEmpty ? nil : $0 }
    ?? "\(packageDir)/../rust/target/\(rustTriple)/release"

let package = Package(
    name: "SwiftFlowDesktop",
    platforms: [
        .macOS(.v14)
    ],
    products: [
        // Named "SwiftFlow" to match the apple package's product, so an
        // app's `import SwiftFlow` is unchanged when it swaps hosts.
        //
        // Deliberately NOT dynamic. The single-image requirement is about
        // SwiftFlowCore — that's where the renderer statics (NodeRegistry,
        // FrameArena) live and it's what both the preview host and a built
        // views dylib link, so SwiftFlowCore is the `.dynamic` product (see
        // ../Package.swift). This host layer is only ever linked by the host
        // executable — never dlopen'd, never duplicated — so it stays static.
        // Making it dynamic also breaks its link: DesktopHost references the
        // sf_desktop_* symbols, which live in the Rust dylib reached through
        // CSwiftFlow's flags, and those resolve at the *executable* link, not a
        // dylib link of this target.
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
            path: "Sources/SwiftFlow",
            // Links the fully-linked Rust cdylib. -L finds it, -l selects it
            // (the .dylib wins over the .a beside it), and the rpath is how the
            // executable finds it at launch. The frameworks the old static path
            // named by hand are carried inside the cdylib now, so they're gone.
            linkerSettings: [
                .unsafeFlags([
                    "-L\(rustLibDir)",
                    "-lswiftflow_desktop",
                    "-Xlinker", "-rpath", "-Xlinker", rustLibDir,
                ])
            ]
        ),
    ]
)

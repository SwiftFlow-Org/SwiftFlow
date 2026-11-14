// swift-tools-version: 6.0
import PackageDescription

// Checks of SwiftFlow's view building, layout and hit testing, run as a
// program: each one builds a small view, lays it out without drawing it
// and checks where things landed. Exits non-zero when one fails.
//
//   SWIFTFLOW_PLATFORM=desktop swift run LayoutChecks
//
// It links the same Rust library a desktop app does (see
// SWIFTFLOW_RUST_LIB_DIR and SWIFTFLOW_RUST_LIB_NAME in ../../desktop).
let package = Package(
    name: "LayoutChecks",
    platforms: [.macOS(.v14)],
    dependencies: [
        .package(name: "SwiftFlowDesktop", path: "../../desktop"),
    ],
    targets: [
        .executableTarget(
            name: "LayoutChecks",
            dependencies: [.product(name: "SwiftFlow", package: "SwiftFlowDesktop")]
        ),
    ]
)

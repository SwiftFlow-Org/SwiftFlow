// swift-tools-version: 6.0
import CompilerPluginSupport
import PackageDescription

// The `@Observable` macro, in its own package.
//
// Separate from the framework for one reason: a macro plugin needs
// swift-syntax, which is a multi-minute compile and — more importantly —
// a *host* executable that SwiftPM has to build while cross-compiling to
// iOS or Android. `SwiftFlowCore` must not carry that, or every app pays
// it whether or not it wants macros.
//
// So an app opts in, the same way it picks a host package. An app that
// doesn't — or that targets a platform where the plugin can't be built —
// uses `@Observed` from the runtime and gets identical behaviour with
// noisier syntax.
//
// # Why not Apple's Observation
//
// `@Observable` from the Observation framework requires iOS 17 / macOS
// 14. That floor is the whole reason this exists: a framework that wants
// to lower its minimum can't adopt one that sets a high one. Nothing
// here has an availability constraint — `init` accessors and macros are
// *compile-time* features, so the generated code runs anywhere Swift
// runs, including wherever the Android SDK reaches.

let package = Package(
    name: "SwiftFlowMacros",
    // Deliberately low. Nothing this package generates needs a modern
    // runtime; setting a floor here would defeat the point of not using
    // Apple's macro.
    platforms: [.iOS(.v13), .macOS(.v10_15)],
    products: [
        .library(name: "SwiftFlowMacros", targets: ["SwiftFlowMacros"])
    ],
    dependencies: [
        // A range across majors rather than `from:`. swift-syntax's major
        // version tracks the compiler's (600 = Swift 6.0, 601 = 6.1, …)
        // and a plugin must be built by the toolchain that will load it,
        // so pinning one major would break for anyone on a different
        // Swift release.
        .package(url: "https://github.com/swiftlang/swift-syntax.git", "600.0.0"..<"604.0.0")
    ],
    targets: [
        // The plugin: runs at compile time, on the host, never shipped.
        .macro(
            name: "SwiftFlowMacrosPlugin",
            dependencies: [
                .product(name: "SwiftSyntaxMacros", package: "swift-syntax"),
                .product(name: "SwiftCompilerPlugin", package: "swift-syntax"),
            ]
        ),
        // The declarations an app imports. No swift-syntax here — this is
        // the half that ships.
        .target(name: "SwiftFlowMacros", dependencies: ["SwiftFlowMacrosPlugin"]),
        .testTarget(
            name: "SwiftFlowMacrosTests",
            dependencies: [
                "SwiftFlowMacrosPlugin",
                .product(name: "SwiftSyntaxMacrosTestSupport", package: "swift-syntax"),
            ]
        ),
    ]
)

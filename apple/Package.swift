// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "SwiftFlowApple",
    platforms: [
        .iOS(.v17)
    ],
    products: [
        // Named "SwiftFlow" (not "SwiftFlowApple") so app projects can just
        // `import SwiftFlow` — this target re-exports SwiftFlowCore's public
        // API via `@_exported import` in SwiftFlowApp.swift, so a single
        // import gives an app both the pure framework and the platform glue.
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
            linkerSettings: [
                .linkedFramework("QuartzCore"),
                .linkedFramework("UIKit"),
                .linkedFramework("Metal"),
                .linkedFramework("MetalKit"),
                .linkedFramework("CoreGraphics"),
            ]
        ),
    ]
)

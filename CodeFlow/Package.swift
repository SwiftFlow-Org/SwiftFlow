// swift-tools-version: 6.0
import PackageDescription
import Foundation

// One app, three hosts.
//
// The sources are platform-agnostic — `import SwiftFlow` and
// `@main struct App: SwiftFlowApp` are identical everywhere. What can't
// be shared is the *shape of the package*: iOS wants a library product,
// desktop an executable, Android a shared library NativeActivity can
// dlopen. So the manifest branches rather than the app existing three
// times.
let platform = ProcessInfo.processInfo.environment["SWIFTFLOW_PLATFORM"] ?? "desktop"
let desktop = platform == "desktop"
let android = platform == "android"

// ── Finding SwiftFlow ─────────────────────────────────────────────────
// The framework is installed in ~/.swiftflow rather than vendored here.
// What this project pins is `[swiftflow] version` in SwiftFlow.toml:
//
//   0.1.0   an installed release
//   dev     a symlink to a framework working tree
//   (none)  whatever `current` points at
//
// Resolved here rather than by a generator because a manifest runs on the
// host and can read the filesystem — so `swift build` works on a fresh
// clone with no generate step, and the pin is a file you can diff instead
// of an absolute path baked into a manifest.
let projectDir = URL(fileURLWithPath: #filePath).deletingLastPathComponent().path

let swiftflowHome = ProcessInfo.processInfo.environment["SWIFTFLOW_HOME"]
    ?? "\(NSHomeDirectory())/.swiftflow"

// Scanned for, not parsed. A manifest has no TOML parser and no way to
// add a dependency on one — but it needs exactly one key out of this
// file, so it looks for `version` under `[swiftflow]` and ignores
// everything else. The CLI reads the file properly.
let pin: String? = {
    guard let text = try? String(contentsOfFile: "\(projectDir)/SwiftFlow.toml", encoding: .utf8)
    else { return nil }
    var inSwiftFlowSection = false
    for rawLine in text.split(separator: "\n", omittingEmptySubsequences: false) {
        var line = String(rawLine)
        if let hash = line.firstIndex(of: "#") { line = String(line[..<hash]) }
        line = line.trimmingCharacters(in: .whitespaces)
        if line.hasPrefix("[") {
            inSwiftFlowSection = line == "[swiftflow]"
            continue
        }
        guard inSwiftFlowSection, let equals = line.firstIndex(of: "=") else { continue }
        guard line[..<equals].trimmingCharacters(in: .whitespaces) == "version" else { continue }
        return String(line[line.index(after: equals)...])
            .trimmingCharacters(in: .whitespaces)
            .trimmingCharacters(in: CharacterSet(charactersIn: "\"'"))
    }
    return nil
}()

let swiftflowRoot = (pin?.isEmpty == false)
    ? "\(swiftflowHome)/versions/\(pin!)"
    : "\(swiftflowHome)/current"

let hostPath = android
    ? "\(swiftflowRoot)/android"
    : (desktop ? "\(swiftflowRoot)/desktop" : "\(swiftflowRoot)/apple")
let hostName = android ? "SwiftFlowAndroid" : (desktop ? "SwiftFlowDesktop" : "SwiftFlowApple")

// nil rather than an empty list on Android: SwiftPM 6 has no `.android`
// SupportedPlatform, and stating an iOS or macOS minimum for a build that
// is neither only invites the resolver to check it.
let platforms: [SupportedPlatform]? =
    android ? nil : (desktop ? [.macOS(.v14)] : [.iOS(.v17)])

let product: Product = {
    if android {
        return .library(
            name: "CodeFlow",
            type: .dynamic,
            targets: ["CodeFlow", "CodeFlowAndroidEntry"]
        )
    }
    return desktop
        ? .executable(name: "CodeFlow", targets: ["CodeFlow"])
        : .library(name: "CodeFlow", targets: ["CodeFlow"])
}()

// Directories rather than a file list, so adding a view or a model needs
// no manifest edit. Assets/ is deliberately absent: it is a resource.
let sources = ["CodeFlowApp.swift", "ContentView.swift", "Model", "Syntax", "Views"]

// Android starts in native code and calls up into Swift, so it needs a C
// entry point `@main` can't provide. `swiftflow build` generates one into
// .build/swiftflow-entry and it is compiled as its own target, so nothing
// platform-conditional lives in Sources/.
let entryTarget: Target = .target(
    name: "CodeFlowAndroidEntry",
    dependencies: ["CodeFlow", .product(name: "SwiftFlow", package: hostName)],
    path: ".build/swiftflow-entry"
)

// `.executableTarget` on desktop, `.target` everywhere else.
//
// SwiftPM only infers that a target is executable from a file literally
// named `main.swift`. This app's entry point is `@main struct CodeFlowApp`
// in CodeFlowApp.swift, so pairing the executable product above with a
// plain `.target` fails outright:
//
//   error: executable product 'CodeFlow' expects target 'CodeFlow' to be
//   executable; an executable target requires a 'main.swift' file
//
// iOS and Android build *library* products — a dynamic one on Android for
// NativeActivity to dlopen — where `.executableTarget` would be equally
// wrong. Hence the branch rather than picking one.
let appTarget: Target = {
    let deps: [Target.Dependency] = [.product(name: "SwiftFlow", package: hostName)]
    // Sources are listed explicitly because Assets/ lives inside the
    // target directory and is a resource, not a source.
    if desktop {
        return .executableTarget(
            name: "CodeFlow",
            dependencies: deps,
            path: "Sources/CodeFlow",
            sources: sources,
            resources: [.copy("Assets")]
        )
    }
    return .target(
        name: "CodeFlow",
        dependencies: deps,
        path: "Sources/CodeFlow",
        sources: sources,
        resources: [.copy("Assets")]
    )
}()

let package = Package(
    name: "CodeFlow",
    platforms: platforms,
    products: [product],
    dependencies: [
        .package(name: hostName, path: hostPath)
    ],
    targets: android ? [appTarget, entryTarget] : [appTarget]
)

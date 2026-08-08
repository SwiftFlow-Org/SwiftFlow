import Foundation

enum SampleProject {
    static let entries: [FileEntry] = [
        FileEntry(id: 0, name: "Sources", path: "Sources", isDirectory: true, depth: 0),
        FileEntry(
            id: 1, name: "Counter.swift", path: "Sources/Counter.swift",
            isDirectory: false, depth: 1
        ),
        FileEntry(
            id: 2, name: "Palette.swift", path: "Sources/Palette.swift",
            isDirectory: false, depth: 1
        ),
        FileEntry(id: 3, name: "SwiftFlow.toml", path: "SwiftFlow.toml", isDirectory: false, depth: 0),
        FileEntry(id: 4, name: "README.md", path: "README.md", isDirectory: false, depth: 0),
    ]

    static func contents(of path: String) -> String {
        switch path {
        case "Sources/Counter.swift": return counter
        case "Sources/Palette.swift": return palette
        case "SwiftFlow.toml": return manifest
        case "README.md": return readme
        default: return ""
        }
    }

    private static let counter = #"""
    import SwiftFlow

    // A counter, which is the smallest thing that proves state works:
    // the tree is rebuilt every frame, so a count that survives the
    // rebuild is the whole claim.
    struct CounterView: View {
        @State private var count = 0
        private let step = 1

        var body: some View {
            VStack(spacing: 16) {
                Text("Tapped \(count) times")
                    .font(.title2)
                    .fontWeight(.bold)

                Button("Tap me") {
                    withAnimation(.spring()) {
                        count += step
                    }
                }
            }
            .padding(24)
        }
    }
    """#

    private static let palette = #"""
    import SwiftFlow

    /// Colours the sample app draws with.
    enum Palette {
        static let ink = Color(hex: 0xF6F1E9)
        static let rust = Color(hex: 0xC15B3A)
        static let slate = Color(hex: 0x6E675C)

        /// 0 is flat, 1 is the full tint.
        static func blend(_ amount: Float) -> Color {
            rust.opacity(amount)
        }
    }
    """#

    private static let manifest = #"""
    [swiftflow]
    version = "dev"

    [app]
    id      = "com.swiftflow.sample"
    name    = "Sample"
    version = "0.1.0"
    build   = 1

    # Lowered to a usage description on iOS and a permission on Android.
    [capabilities.camera]
    reason = "Scan a page to import it"
    """#

    private static let readme = #"""
    # Sample

    A project that exists to be opened, not run.

    Markdown has no lexer here, which is the point of including it: an
    unhighlighted file should look deliberate rather than broken. It
    draws in the plain foreground colour, with the gutter and the status
    bar behaving exactly as they do for Swift.
    """#
}

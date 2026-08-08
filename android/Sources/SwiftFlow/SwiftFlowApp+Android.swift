import CSwiftFlow
import Foundation
@_exported import SwiftFlowCore

// `@main struct App: SwiftFlowApp` on all three — an app swaps its

// On iOS and desktop, `@main` *is* the entry point: the generated `main`

// reverses — Rust starts, and has to call *up* into Swift — and `@main`'s

extension SwiftFlowApp {
    public static func main() {

        let app = Self()
        guard let provider = app.body as? any LifecycleProvider else {
            fatalError(
                "A SwiftFlowApp's body must be a Scene that provides a root view, e.g. WindowGroup"
            )
        }

        let host = AndroidHost.shared
        host.rootView = provider.rootView
        host.lifecycle = provider.lifecycle
        host.run()
    }
}

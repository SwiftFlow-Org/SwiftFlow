import CSwiftFlow
import Foundation
@_exported import SwiftFlowCore

// apple package. Same `@main struct App: SwiftFlowApp` on both — an app

extension SwiftFlowApp {
    public static func main() {

        setbuf(stdout, nil)

        MainActor.assumeIsolated {
            let app = Self()
            guard let provider = app.body as? any LifecycleProvider else {
                fatalError(
                    "A SwiftFlowApp's body must be a Scene that provides a root view, e.g. WindowGroup"
                )
            }

            let host = DesktopHost.shared
            host.rootView = provider.rootView
            host.lifecycle = provider.lifecycle
            host.run(
                title: DesktopWindow.title,
                width: DesktopWindow.width,
                height: DesktopWindow.height
            )
        }
    }
}

public enum DesktopWindow {
    nonisolated(unsafe) public static var title: String = "SwiftFlow"
    nonisolated(unsafe) public static var width: UInt32 = 420
    nonisolated(unsafe) public static var height: UInt32 = 900
}

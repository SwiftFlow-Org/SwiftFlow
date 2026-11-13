import Foundation

/// A native file chooser, for opening a document.
///
/// Works like `FolderPicker`: `open` posts a request, the host runs the
/// chooser on the thread that owns the window, and the path arrives on a
/// later frame through `drainPending`. Cancelling produces nothing.
public final class FilePicker {
    nonisolated(unsafe) public static let shared = FilePicker()

    /// Installed by the host at startup. Nil where there is no chooser.
    public var requestOpen: (([String]) -> Void)?
    public var takePicked: (() -> String?)?

    private var handler: ((String) -> Void)?

    public var isSupported: Bool { requestOpen != nil }

    /// Offers files with these extensions (no dots), or any file when
    /// `extensions` is empty.
    public func open(extensions: [String] = [], _ completion: @escaping (String) -> Void) {
        guard let requestOpen else { return }
        handler = completion
        requestOpen(extensions)
    }

    /// Called once a frame by the host, like the other drains.
    public func drainPending() {
        guard let path = takePicked?(), !path.isEmpty else { return }
        let completion = handler
        handler = nil
        completion?(path)
        NodeRegistry.shared.needsRender = true
    }
}

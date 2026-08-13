import Foundation

/// A native folder chooser.
///
/// Asking and answering are separate calls because the dialog has to run on
/// the thread that owns the window: `open` posts a request, the host runs the
/// chooser on its next pass, and `drainPending` collects the result — the same
/// split the IME plumbing uses.
public final class FolderPicker {
    nonisolated(unsafe) public static let shared = FolderPicker()

    /// Installed by the host at startup. Nil where there is no chooser, which
    /// is every platform but desktop.
    public var requestOpen: (() -> Void)?
    public var takePicked: (() -> String?)?

    private var handler: ((String) -> Void)?

    public var isSupported: Bool { requestOpen != nil }

    /// Fires on a later frame, or never — cancelling produces no result, and
    /// there is deliberately no callback for it.
    public func open(_ completion: @escaping (String) -> Void) {
        guard let requestOpen else { return }
        handler = completion
        requestOpen()
    }

    /// Called once a frame by the app, like the other host drains.
    public func drainPending() {
        guard let path = takePicked?(), !path.isEmpty else { return }
        let completion = handler
        handler = nil
        completion?(path)
    }
}

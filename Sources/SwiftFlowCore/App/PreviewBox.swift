/// Carries a `#Preview`'s view across the C ABI boundary from the generated
/// preview index to the host process.
///
/// The view is produced lazily: the host builds only the preview it selects,
/// so a body that traps sinks that one preview when it's chosen, not every
/// preview at load. `sf_preview_make` hands one of these back to the host as an
/// `Unmanaged`-retained opaque pointer.
///
/// This type lives in SwiftFlowCore, not in the generated index, on purpose.
/// The views dylib and the host each `Unmanaged`-box and unbox through it, and
/// that is only sound if both name the *same* type — which holds only because
/// SwiftFlow is a single dynamic image in the process (stage 2a). A `PreviewBox`
/// defined on each side would be two distinct types with the same name.
public final class PreviewBox {
    private let content: () -> any View

    public init(_ content: @escaping () -> any View) {
        self.content = content
    }

    /// Builds the view. Called on the host's render thread when the preview is
    /// selected — the point at which a trapping body takes down the host alone.
    public func makeView() -> any View {
        content()
    }
}

import CSwiftFlow

/// A rectangle the app draws into itself with wgpu.
///
/// The drawing is done in Rust, by the painter registered under the same
/// id with `swiftflow_wgpu::canvas::register`. The view only reserves the
/// space and says where in the draw order the painter runs, so anything
/// later in the body (a toolbar, a glass panel) draws over the canvas and
/// glass refracts it.
///
/// A canvas fills the space it is offered. Give it a frame to size it.
///
/// ```swift
/// Canvas(id: 1)
///     .onPointer { event in atelier_canvas_pointer(…) }
/// ```
public struct Canvas: View {
    public typealias Body = Never
    public var body: Never { fatalError() }

    let id: UInt32

    /// `id` is the painter's id on the Rust side, and must not be 0.
    public init(id: UInt32) {
        precondition(id != 0, "canvas id 0 means no canvas")
        self.id = id
    }

    /// Whether a painter has asked for a frame since the last call, and
    /// clears the request. Hosts check this every tick, so a painter can
    /// redraw (a stroke arriving, playback advancing) without any view
    /// changing.
    public static func takeFrameRequest() -> Bool {
        swiftflow_canvas_take_frame_request()
    }
}

extension Canvas {
    public func toSFNode() -> SFNode {
        var node = SFNode.makeDefault()
        node.kind = SF_NODE_CANVAS
        node.imageId = id
        node.sizing = SF_SIZING_FILL
        return node
    }
}

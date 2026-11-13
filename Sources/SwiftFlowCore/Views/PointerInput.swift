import CSwiftFlow

/// A raw pointer event on a view, delivered before any gesture interpretation.
///
/// Use `.onPointer` to receive the down → move → up stream directly — for
/// forwarding it verbatim into an embedded surface (a live preview), or any case
/// that needs exact pointer timing rather than a recognized gesture. The pointer
/// is *captured* on `down`: the following moves and the `up` go to the same view
/// even after the pointer leaves its bounds.
///
/// `location` is local to the view, in points.
public struct PointerEvent {
    public enum Phase: Sendable {
        case down
        case moved
        case up
    }

    public let phase: Phase
    public let location: Point

    /// What produced the event.
    public let tool: PointerTool

    /// The pen's pressure, tilt and rotation when `tool` is `.pen` or
    /// `.eraser`; `nil` for a mouse or a finger.
    public let stylus: StylusSample?

    /// Seconds, on the host's monotonic clock.
    public let timestamp: Double

    public init(
        phase: Phase,
        location: Point,
        tool: PointerTool = .mouse,
        stylus: StylusSample? = nil,
        timestamp: Double = 0
    ) {
        self.phase = phase
        self.location = location
        self.tool = tool
        self.stylus = stylus
        self.timestamp = timestamp
    }
}

/// Raw pointer handlers attached to nodes this frame, keyed by node id — the
/// same per-node-registry shape gestures use. Cleared and repopulated each
/// build; ids are registered with `NodeFrames` so their frames are read back
/// for local-coordinate conversion.
final class PointerRegistry {
    nonisolated(unsafe) static let shared = PointerRegistry()

    nonisolated(unsafe) var handlers: [UInt32: (PointerEvent) -> Void] = [:]

    func beginBuild() {
        for id in handlers.keys { NodeFrames.shared.register(id) }
        handlers.removeAll(keepingCapacity: true)
    }

    func rekey(from old: UInt32, to new: UInt32) {
        if let handler = handlers.removeValue(forKey: old) { handlers[new] = handler }
    }
}

public struct PointerModifier<Content: View>: View {
    public typealias Body = Never
    public var body: Never { fatalError() }
    let content: Content
    let action: (PointerEvent) -> Void
}

extension PointerModifier {
    public func toSFNode() -> SFNode {
        var node = content.toSFNode()

        let id = node.node_id != 0 ? node.node_id : BuildContext.shared.currentID(for: self)
        node.node_id = id
        PointerRegistry.shared.handlers[id] = action
        return node
    }
}

extension View {
    /// Receives the raw pointer stream (down, moves, up) on this view, before
    /// gesture interpretation — the pointer is captured on `down`, so the moves
    /// and up arrive here even if the pointer leaves the view.
    public func onPointer(perform action: @escaping (PointerEvent) -> Void) -> PointerModifier<Self> {
        PointerModifier(content: self, action: action)
    }
}

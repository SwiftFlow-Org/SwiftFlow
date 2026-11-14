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
        /// The pointer moved over the view with nothing pressed: a mouse
        /// passing over, or a pen hovering above a tablet. Only hosts that
        /// track hover send it (the desktop host does).
        case hover
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

    /// `.onNavigate` handlers, kept here because they live and die with
    /// the same nodes.
    nonisolated(unsafe) var navigators: [UInt32: (NavigationEvent) -> Void] = [:]

    /// `.onHover` handlers, told true when the pointer comes over their view
    /// and false when it leaves.
    nonisolated(unsafe) var hovers: [UInt32: (Bool) -> Void] = [:]

    func beginBuild() {
        for id in handlers.keys { NodeFrames.shared.register(id) }
        for id in navigators.keys { NodeFrames.shared.register(id) }
        handlers.removeAll(keepingCapacity: true)
        navigators.removeAll(keepingCapacity: true)
        hovers.removeAll(keepingCapacity: true)
    }

    func rekey(from old: UInt32, to new: UInt32) {
        if let handler = handlers.removeValue(forKey: old) { handlers[new] = handler }
        if let handler = navigators.removeValue(forKey: old) { navigators[new] = handler }
        if let handler = hovers.removeValue(forKey: old) { hovers[new] = handler }
    }
}

/// A request to move what a view shows: pan, zoom or turn it. For views
/// that show something larger than themselves, like a drawing canvas.
///
/// The desktop host sends these from a trackpad (two-finger scroll, pinch,
/// twist), a mouse wheel (⌘ or Ctrl with the wheel zooms) and middle-button
/// drags. Positions are local to the view, in points.
public enum NavigationEvent: Sendable {
    /// Move the content by this much, in points.
    case pan(dx: Float, dy: Float)
    /// Scale the content by `factor` about `anchor`.
    case zoom(factor: Float, anchor: Point)
    /// Turn the content clockwise by `radians` about `anchor`.
    case rotate(radians: Float, anchor: Point)
    /// The gesture that sent the events before this one has ended.
    case end
}

public struct NavigateModifier<Content: View>: View {
    public typealias Body = Never
    public var body: Never { fatalError() }
    let content: Content
    let action: (NavigationEvent) -> Void
}

extension NavigateModifier {
    public func toSFNode() -> SFNode {
        var node = content.toSFNode()
        let id = node.node_id != 0 ? node.node_id : BuildContext.shared.currentID(for: self)
        node.node_id = id
        PointerRegistry.shared.navigators[id] = action
        return node
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
        node.catchesPresses = 1
        PointerRegistry.shared.handlers[id] = action
        return node
    }
}

public struct HoverModifier<Content: View>: View {
    public typealias Body = Never
    public var body: Never { fatalError() }
    let content: Content
    let action: (Bool) -> Void
}

extension HoverModifier {
    public func toSFNode() -> SFNode {
        var node = content.toSFNode()
        let id = node.node_id != 0 ? node.node_id : BuildContext.shared.currentID(for: self)
        node.node_id = id
        PointerRegistry.shared.hovers[id] = action
        return node
    }
}

extension View {
    /// Calls `action` with true when the pointer comes over this view and
    /// false when it leaves, as SwiftUI's `onHover` does. Only hosts that
    /// track a pointer with nothing pressed send it (the desktop host does).
    /// Every view on the way down from the window to the pointer counts as
    /// under it, not only the innermost.
    public func onHover(perform action: @escaping (Bool) -> Void) -> HoverModifier<Self> {
        HoverModifier(content: self, action: action)
    }

    /// Receives pan, zoom and rotate requests aimed at this view: trackpad
    /// gestures, the scroll wheel, middle-button drags. A view with this
    /// takes the scroll wheel instead of any scroll view around it.
    public func onNavigate(perform action: @escaping (NavigationEvent) -> Void) -> NavigateModifier<Self> {
        NavigateModifier(content: self, action: action)
    }

    /// Receives the raw pointer stream (down, moves, up) on this view, before
    /// gesture interpretation — the pointer is captured on `down`, so the moves
    /// and up arrive here even if the pointer leaves the view.
    public func onPointer(perform action: @escaping (PointerEvent) -> Void) -> PointerModifier<Self> {
        PointerModifier(content: self, action: action)
    }
}

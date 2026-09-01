import CSwiftFlow

public final class NodeFrames {
    nonisolated(unsafe) public static let shared = NodeFrames()

    nonisolated(unsafe) private var registered: Set<UInt32> = []
    nonisolated(unsafe) private var frames: [UInt32: SFRect] = [:]

    nonisolated(unsafe) public private(set) var lastPressed: UInt32 = 0

    public func register(_ id: UInt32) {
        registered.insert(id)
    }

    public func notePressed(_ id: UInt32) {
        lastPressed = id
    }

    public var tracked: Set<UInt32> {
        var all = registered
        all.formUnion(NodeRegistry.shared.pressedNodes)
        if lastPressed != 0 {
            all.insert(lastPressed)
        }
        return all
    }

    public func record(_ id: UInt32, frame: SFRect) {
        if frame.width > 0 && frame.height > 0 {
            frames[id] = frame
        } else {
            frames.removeValue(forKey: id)
        }
    }

    public func frame(for id: UInt32) -> SFRect? {
        frames[id]
    }

    public func beginFrame() {
        registered.removeAll(keepingCapacity: true)
    }

    public static func id(for name: String) -> UInt32 {
        fnv1a("morph:\(name)")
    }
}

/// Labels a node so the host measures it, without going through
/// `NodeBuilder`.
///
/// `MatchedTransitionSource` gets its id assigned on the way out of
/// `buildAny`, which only reaches a view's outermost node — anything nested
/// inside a `ForEach` row loses to the row's own key. Writing the id here
/// instead survives that, at the price of only working on a node that claims
/// nothing of its own.
struct MeasuredNode<Content: View>: View {
    typealias Body = Never
    var body: Never { fatalError() }

    let content: Content
    let identity: UInt32

    func toSFNode() -> SFNode {
        var node = content.toSFNode()

        // Relabelling a node something has already registered against — a
        // button's tap handler, say — would strand that registration on an id
        // nothing carries. Measure nothing rather than break the tap.
        guard node.node_id == 0 else { return node }

        NodeFrames.shared.register(identity)
        node.node_id = identity
        return node
    }
}

extension View {
    /// Tracks this view's laid-out frame under `id`, readable on the next
    /// build through `NodeFrames`.
    func measured(as id: UInt32) -> MeasuredNode<Self> {
        MeasuredNode(content: self, identity: id)
    }
}

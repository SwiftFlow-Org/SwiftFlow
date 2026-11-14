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

    /// Keeps `frame` as where `id` was laid out. A frame that differs from
    /// the last one asks for another build, so views placed against it
    /// (read during this build, before layout) catch up.
    public func record(_ id: UInt32, frame: SFRect) {
        let old = frames[id]
        if frame.width > 0 && frame.height > 0 {
            frames[id] = frame
        } else {
            frames.removeValue(forKey: id)
        }
        guard registered.contains(id) else { return }
        let new = frames[id]
        let same = switch (old, new) {
        case (nil, nil): true
        case let (a?, b?): a.x == b.x && a.y == b.y && a.width == b.width && a.height == b.height
        default: false
        }
        if !same {
            NodeRegistry.shared.needsRender = true
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

/// Measures a view under an id of the caller's choosing.
///
/// The id goes on a wrapper node of its own around the view, laid out
/// exactly as the view (same sizing, same fixed size), so it never takes
/// the place of an id something else has given the view: a button's, a
/// tap handler's, a ForEach row's. The wrapper sits one level inside an
/// outer one, because the outermost node of a view built on its own takes
/// the identity the build gives it.
public struct MeasuredNode<Content: View>: View {
    public typealias Body = Never
    public var body: Never { fatalError() }

    let content: Content
    let identity: UInt32

    public func toSFNode() -> SFNode {
        let node = content.toSFNode()
        NodeFrames.shared.register(identity)
        var measured = Self.wrap(node)
        measured.node_id = identity
        return Self.wrap(measured)
    }

    /// A depth stack around `node` that lays out as `node` does.
    private static func wrap(_ node: SFNode) -> SFNode {
        var wrapper = SFNode.makeDefault()
        wrapper.kind = SF_NODE_STACK
        wrapper.axis = SF_AXIS_DEPTH
        wrapper.alignment = SF_ALIGNMENT_LEADING
        wrapper.verticalAlignment = SF_ALIGNMENT_LEADING
        wrapper.sizingX = node.sizingX
        wrapper.sizingY = node.sizingY
        wrapper.fixedWidth = node.fixedWidth
        wrapper.fixedHeight = node.fixedHeight
        var children = [node]
        FrameArena.shared.storeNodes(&children) { pointer in
            wrapper.children = pointer
            wrapper.childrenLen = 1
        }
        return wrapper
    }
}

extension View {
    /// Tracks this view's laid-out frame under `id`, readable on the next
    /// build through `NodeFrames.shared.frame(for:)`, in pixels. For placing
    /// something against where another view was laid out, such as a
    /// popover's tail under the button that opened it.
    public func measured(as id: UInt32) -> MeasuredNode<Self> {
        MeasuredNode(content: self, identity: id)
    }
}

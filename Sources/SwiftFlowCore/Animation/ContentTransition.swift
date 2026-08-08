import CSwiftFlow

public struct ContentTransitionModifier<Content: View>: View {
    public typealias Body = Never
    public var body: Never { fatalError() }

    let content: Content
    let transition: Transition
    let animation: Animation?
}

extension ContentTransitionModifier {
    public func toSFNode() -> SFNode {
        var node = content.toSFNode()

        let id = node.node_id != 0 ? node.node_id : BuildContext.shared.currentID(for: self)
        node.node_id = id

        return ContentTransitionRegistry.shared.build(
            node, id: id, transition: transition, animation: animation
        )
    }
}

extension View {

    /// How this view's *content* changes when it is replaced by different content.
    public func contentTransition(
        _ transition: Transition,
        animation: Animation? = nil
    ) -> ContentTransitionModifier<Self> {
        ContentTransitionModifier(content: self, transition: transition, animation: animation)
    }
}

final class ContentTransitionRegistry {
    nonisolated(unsafe) static let shared = ContentTransitionRegistry()

    private struct Entry {

        var content: UInt64

        var retained: RetainedNode

        var natural: AnimatableSnapshot

        var incoming: AnimationState?

        var outgoingNode: RetainedNode?
        var outgoing: AnimationState?
    }

    private var entries: [UInt32: Entry] = [:]
    private var seen: Set<UInt32> = []

    func build(
        _ node: SFNode,
        id: UInt32,
        transition: Transition,
        animation: Animation?
    ) -> SFNode {
        var node = node
        seen.insert(id)

        let content = Self.contentHash(node)
        let natural = AnimatableSnapshot.extract(from: node)
        let curve = animation ?? AnimationTransaction.ambient ?? .default

        var entry = entries[id]

        if let previous = entry, previous.content != content {

            let outgoing = AnimationState()
            outgoing.retarget(to: previous.natural, curve: curve)
            outgoing.retarget(to: transition.removal.applied(to: previous.natural), curve: curve)
            entry?.outgoing = outgoing

            entry?.outgoingNode = previous.retained

            let incoming = AnimationState()
            incoming.retarget(to: transition.insertion.applied(to: natural), curve: curve)
            incoming.retarget(to: natural, curve: curve)
            entry?.incoming = incoming
        }

        let retained = RetainedNode.capture(node)

        if let incoming = entry?.incoming {
            if incoming.isAnimating {
                incoming.current.apply(to: &node)
            } else {
                entry?.incoming = nil
            }
        }

        var result = node

        if let state = entry?.outgoing, let ghostNode = entry?.outgoingNode {
            if state.isAnimating {
                result = Self.overlay(
                    outgoing: ghostNode.emit(applying: state.current),
                    incoming: node,
                    id: id
                )
            } else {
                entry?.outgoing = nil
                entry?.outgoingNode = nil
            }
        }

        entries[id] = Entry(
            content: content,
            retained: retained,
            natural: natural,
            incoming: entry?.incoming,
            outgoingNode: entry?.outgoingNode,
            outgoing: entry?.outgoing
        )

        return result
    }

    private static func overlay(outgoing: SFNode, incoming: SFNode, id: UInt32) -> SFNode {
        var incoming = incoming

        incoming.node_id = 0

        var children = [outgoing, incoming]
        let count = children.count

        var stack = SFNode.makeDefault()
        stack.kind = SF_NODE_STACK
        stack.axis = SF_AXIS_DEPTH
        stack.sizing = SF_SIZING_HUG
        stack.node_id = id
        FrameArena.shared.storeNodes(&children) { pointer in
            stack.children = pointer
            stack.childrenLen = count
        }
        return stack
    }

    func endBuild() {
        entries = entries.filter { seen.contains($0.key) }
        seen.removeAll(keepingCapacity: true)
    }

    var hasActiveAnimations: Bool {
        entries.values.contains {
            ($0.incoming?.isAnimating ?? false) || ($0.outgoing?.isAnimating ?? false)
        }
    }

    func step(dt: Float) {
        for entry in entries.values {
            entry.incoming?.step(dt: dt)
            entry.outgoing?.step(dt: dt)
        }
    }

    private static func contentHash(_ node: SFNode) -> UInt64 {
        var hash: UInt64 = 14695981039346656037
        func mix(_ byte: UInt8) {
            hash ^= UInt64(byte)
            hash = hash &* 1099511628211
        }
        func mixBytes<T>(_ value: T) {
            withUnsafeBytes(of: value) { bytes in
                for byte in bytes { mix(byte) }
            }
        }

        if let text = node.text, node.textLen > 0 {
            for index in 0..<Int(node.textLen) { mix(text[index]) }
        }
        mixBytes(node.fontSize)
        mixBytes(node.fontWeight)
        mixBytes(node.imageId)
        return hash
    }
}

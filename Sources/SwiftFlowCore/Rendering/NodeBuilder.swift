import CSwiftFlow

public enum NodeBuilder {
    public static func build<V: View>(_ view: V) -> SFNode {
        BuildContext.shared.reset()

        NavigationConfigStore.shared.resetAll()
        AnimationTransaction.beginBuild()

        GestureRegistry.shared.beginBuild()

        TextInput.shared.beginBuild()

        EnvironmentValues.beginBuild()

        let root = buildAny(view)

        AnimationTransaction.endBuild()
        TransitionRegistry.shared.endBuild()
        ContentTransitionRegistry.shared.endBuild()
        NodeRegistry.shared.pruneNodeAnimationStates()
        return root
    }

    static func buildAny(_ view: any View) -> SFNode {

        let explicit = (view as? any ExplicitlyIdentifiedView)?.explicitIdentity

        BuildContext.shared.push()
        var node    = view.toSFNode()

        let claimed = node.node_id != 0 ? node.node_id : nil
        let identity = explicit ?? claimed ?? BuildContext.shared.currentID(for: view)

        if let claimed, claimed != identity {
            GestureRegistry.shared.rekey(from: claimed, to: identity)
            NodeRegistry.shared.rekeyTap(from: claimed, to: identity)
            TextInput.shared.rekey(from: claimed, to: identity)
        }
        node.node_id = identity
        BuildContext.shared.pop()
        BuildContext.shared.advance()

        TransitionRegistry.shared.observe(node)
        applyAmbientAnimation(to: &node)
        return node
    }

    private static func applyAmbientAnimation(to node: inout SFNode) {
        let registry = NodeRegistry.shared

        let explicitlyAnimated = AnimationTransaction.consumeExplicitlyAnimated()
        if AnimationTransaction.ambient == nil && registry.nodeAnimationStates.isEmpty {
            return
        }

        if explicitlyAnimated { return }

        if let curve = AnimationTransaction.ambient {
            let state = registry.nodeAnimationState(for: node.node_id)
            registry.touchedNodeAnimationIDs.insert(node.node_id)
            state.retarget(to: .extract(from: node), curve: curve)
            state.current.apply(to: &node)
            return
        }

        guard let state = registry.nodeAnimationStates[node.node_id] else { return }
        guard state.isAnimating else {

            registry.nodeAnimationStates.removeValue(forKey: node.node_id)
            return
        }
        registry.touchedNodeAnimationIDs.insert(node.node_id)
        state.retarget(to: .extract(from: node), curve: state.curve)
        state.current.apply(to: &node)
    }
}

public final class FrameArena {
    nonisolated(unsafe) public static let shared = FrameArena()
    private var strings: [ContiguousArray<UInt8>] = []
    private var nodes: [ContiguousArray<SFNode>] = []

    public func reset() {
        strings.removeAll(keepingCapacity: true)
        nodes.removeAll(keepingCapacity: true)
    }

    func store(_ string: String, _ block: (UnsafePointer<UInt8>, UInt) -> Void) {
        let bytes = ContiguousArray(string.utf8)
        strings.append(bytes)
        bytes.withUnsafeBufferPointer { buf in
            block(buf.baseAddress!, UInt(buf.count))
        }
    }

    func storeNodes(_ nodes: inout [SFNode], _ block: (UnsafeMutablePointer<SFNode>) -> Void) {
        var arr = ContiguousArray(nodes)
        arr.withUnsafeMutableBufferPointer { buf in
            block(buf.baseAddress!)
        }
        self.nodes.append(arr)
    }
}

extension Color {
    func toSFColor() -> SFColor {
        SFColor(r: r, g: g, b: b, a: a)
    }
}

extension EdgeInsets {
    func toSFEdgeInsets() -> SFEdgeInsets {
        SFEdgeInsets(top: top * DeviceScale.current, bottom: bottom * DeviceScale.current, leading: leading * DeviceScale.current, trailing: trailing * DeviceScale.current)
    }
}

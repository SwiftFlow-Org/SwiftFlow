import CSwiftFlow

final class TransitionRegistry {
    nonisolated(unsafe) static let shared = TransitionRegistry()

    var pending: Transition?

    private struct Alive {
        var transition: Transition
        var retained: RetainedNode
        var natural: AnimatableSnapshot
    }
    private var alive: [UInt32: Alive] = [:]

    private struct Departing {
        let retained: RetainedNode
        let state: AnimationState
        let index: Int
        let owner: UInt32
    }
    private var departing: [UInt32: Departing] = [:]

    private var lastKeys: [UInt32: [UInt32]] = [:]

    private var hasBuiltOnce = false

    private var seenThisFrame: Set<UInt32> = []

    func observe(_ node: SFNode) {

        let transition = pending
        pending = nil
        guard let transition else { return }

        let id = node.node_id
        let natural = AnimatableSnapshot.extract(from: node)

        let curve = AnimationTransaction.ambient ?? .default

        if let returning = departing.removeValue(forKey: id) {

            let state = NodeRegistry.shared.nodeAnimationState(for: id)
            NodeRegistry.shared.touchedNodeAnimationIDs.insert(id)
            state.retarget(to: returning.state.current, curve: curve)
            state.retarget(to: natural, curve: curve)
        } else if hasBuiltOnce && alive[id] == nil {
            let state = NodeRegistry.shared.nodeAnimationState(for: id)
            NodeRegistry.shared.touchedNodeAnimationIDs.insert(id)
            state.retarget(to: transition.insertion.applied(to: natural), curve: curve)
            state.retarget(to: natural, curve: curve)
        }

        seenThisFrame.insert(id)

        alive[id] = Alive(
            transition: transition,
            retained: RetainedNode.capture(node),
            natural: natural
        )
    }

    func endBuild() {
        hasBuiltOnce = true

        alive = alive.filter { seenThisFrame.contains($0.key) }
        seenThisFrame.removeAll(keepingCapacity: true)
    }

    func reconcile(owner: UInt32, keys: [UInt32]) {
        defer { lastKeys[owner] = keys }
        guard let previous = lastKeys[owner] else { return }

        let current = Set(keys)
        for (index, key) in previous.enumerated() where !current.contains(key) {
            guard let entry = alive.removeValue(forKey: key) else { continue }
            guard departing[key] == nil else { continue }

            let curve = AnimationTransaction.ambient ?? .default
            let state = AnimationState()
            state.retarget(to: entry.natural, curve: curve)
            state.retarget(to: entry.transition.removal.applied(to: entry.natural), curve: curve)

            departing[key] = Departing(
                retained: entry.retained,
                state: state,
                index: index,
                owner: owner
            )
        }
    }

    func departingNodes(owner: UInt32) -> [(index: Int, node: SFNode)] {
        departing
            .values
            .filter { $0.owner == owner }
            .sorted { $0.index < $1.index }
            .map { (index: $0.index, node: $0.retained.emit(applying: $0.state.current)) }
    }

    var hasActiveAnimations: Bool {
        departing.values.contains { $0.state.isAnimating }
    }

    func step(dt: Float) {
        for (_, entry) in departing {
            entry.state.step(dt: dt)
        }

        departing = departing.filter { $0.value.state.isAnimating }
    }
}

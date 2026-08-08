import CSwiftFlow

public final class NodeRegistry {
    nonisolated(unsafe) public static let shared = NodeRegistry()
    nonisolated(unsafe) public var pressedNodes: Set<UInt32> = []
    nonisolated(unsafe) var stateValues: [UInt32: Any] = [:]
    nonisolated(unsafe) var tapActions:  [UInt32: () -> Void] = [:]
    nonisolated(unsafe) var dirtyNodes:  Set<UInt32> = []

    nonisolated(unsafe) public var needsRender: Bool = true

    nonisolated(unsafe) public var scrollStates: [UInt32: ScrollPhysicsState] = [:]

    public func scrollState(for id: UInt32) -> ScrollPhysicsState {
        if let existing = scrollStates[id] { return existing }
        let created = ScrollPhysicsState()
        scrollStates[id] = created
        return created
    }

    nonisolated(unsafe) public var navTitleCollapsed: [UInt32: Bool] = [:]

    nonisolated(unsafe) public var animationStates: [UInt32: AnimationState] = [:]

    public func animationState(for id: UInt32) -> AnimationState {
        if let existing = animationStates[id] { return existing }
        let created = AnimationState()
        animationStates[id] = created
        return created
    }

    nonisolated(unsafe) public var nodeAnimationStates: [UInt32: AnimationState] = [:]

    func nodeAnimationState(for id: UInt32) -> AnimationState {
        if let existing = nodeAnimationStates[id] { return existing }
        let created = AnimationState()
        nodeAnimationStates[id] = created
        return created
    }

    nonisolated(unsafe) var touchedNodeAnimationIDs: Set<UInt32> = []

    func pruneNodeAnimationStates() {
        nodeAnimationStates = nodeAnimationStates.filter {
            touchedNodeAnimationIDs.contains($0.key)
        }
        touchedNodeAnimationIDs.removeAll(keepingCapacity: true)
    }

    public var hasActiveAnimations: Bool {
        animationStates.values.contains { $0.isAnimating }
            || nodeAnimationStates.values.contains { $0.isAnimating }

            || TransitionRegistry.shared.hasActiveAnimations
            || ContentTransitionRegistry.shared.hasActiveAnimations

            || LayerTransitionRegistry.shared.hasActiveAnimations
    }

    public func stepAnimations(dt: Float) {
        for (_, state) in animationStates { state.step(dt: dt) }
        for (_, state) in nodeAnimationStates { state.step(dt: dt) }
        TransitionRegistry.shared.step(dt: dt)
        ContentTransitionRegistry.shared.step(dt: dt)
        LayerTransitionRegistry.shared.step(dt: dt)

        SwipeActionRegistry.shared.step(dt: dt)

        TextFieldRegistry.shared.step(dt: dt)
    }

    func registerTap(_ id: UInt32, action: @escaping () -> Void) {
        tapActions[id] = action
    }

    func rekeyTap(from old: UInt32, to new: UInt32) {
        if let action = tapActions.removeValue(forKey: old) { tapActions[new] = action }
    }

    public func triggerTap(_ id: UInt32) {
        tapActions[id]?()
    }

    public func markDirty(_ id: UInt32) {
        dirtyNodes.insert(id)
        needsRender = true
    }

    public func clearDirty() {
        dirtyNodes.removeAll()
        needsRender = false
    }
}

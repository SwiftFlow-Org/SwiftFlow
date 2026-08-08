import CSwiftFlow

public enum NavPresentation: Sendable, Codable {

    case push

    case sheet

    case cover
}

public struct NavEntry: Identifiable {
    public let id: UInt64
    let view: any View
}

public struct LayerTransition: Sendable {

    public internal(set) var progress: Float

    var velocity: Float

    var popsOnCompletion: Bool
}

public struct NavTransition: Sendable {

    public var parallax: Float

    public var response: Float

    public var commitThreshold: Float

    public init(parallax: Float = 0.3, response: Float = 0.35, commitThreshold: Float = 0.4) {
        self.parallax = parallax
        self.response = response
        self.commitThreshold = commitThreshold
    }

    public static let standard = NavTransition()
}

public final class NavigationStackModel {

    nonisolated(unsafe) public private(set) var path: [NavEntry] = []

    nonisolated(unsafe) public private(set) var modals: [Modal] = []

    nonisolated(unsafe) private var counter: UInt64 = 0

    public init() {}

    public struct Modal {
        public let id: UInt64
        let view: any View
        public let style: NavPresentation

        let token: UInt64

        let morphSource: UInt32
    }

    nonisolated(unsafe) public private(set) var transition: LayerTransition?

    nonisolated(unsafe) public var transitionStyle: NavTransition = .standard

    public var transitionProgress: Float? { transition?.progress }

    public var isAtRoot: Bool { path.isEmpty }
    public var depth: Int { path.count }

    public var top: NavEntry? { path.last }
    public var isPresenting: Bool { !modals.isEmpty }

    @discardableResult
    public func push(_ view: any View) -> UInt64 {

        settleTransition()
        counter &+= 1
        path.append(NavEntry(id: counter, view: view))

        transition = LayerTransition(progress: 1, velocity: 0, popsOnCompletion: false)
        spring(to: 0)
        changed()
        return counter
    }

    @discardableResult
    public func present(
        _ view: any View,
        as style: NavPresentation = .sheet,
        morphingFrom morphSource: String? = nil
    ) -> UInt64 {
        guard style != .push else {
            return push(view)
        }
        let source =
            morphSource.map(NodeFrames.id(for:))
            ?? NodeFrames.shared.lastPressed
        counter &+= 1
        modals.append(
            Modal(id: counter, view: view, style: style, token: 0, morphSource: source)
        )
        changed()
        return counter
    }

    public func pop() {
        if !modals.isEmpty {
            let modal = modals.removeLast()
            ModalResults.shared.cancel(modal.token)
            changed()
            return
        }
        guard !path.isEmpty else { return }

        if transition == nil {
            transition = LayerTransition(progress: 0, velocity: 0, popsOnCompletion: true)
        } else {
            transition?.popsOnCompletion = true
        }
        spring(to: 1)
        changed()
    }

    public func popToRoot() {
        guard !path.isEmpty || !modals.isEmpty else { return }
        for modal in modals { ModalResults.shared.cancel(modal.token) }
        modals.removeAll()
        if path.count == 1 {
            pop()
            return
        }
        settleTransition()
        path.removeAll()
        changed()
    }

    public func pop(to id: UInt64) {
        guard let index = path.firstIndex(where: { $0.id == id }) else { return }
        for modal in modals { ModalResults.shared.cancel(modal.token) }
        modals.removeAll()

        if index == path.count - 2 {
            pop()
            return
        }
        guard index < path.count - 1 else { return }
        settleTransition()
        path.removeSubrange((index + 1)...)
        changed()
    }

    public func setPath(_ views: [any View]) {
        settleTransition()
        for modal in modals { ModalResults.shared.cancel(modal.token) }
        modals.removeAll()
        path = views.map { view in
            counter &+= 1
            return NavEntry(id: counter, view: view)
        }
        changed()
    }

    private func settleTransition() {
        LayerTransitionRegistry.shared.cancel(ObjectIdentifier(self))
        guard let inFlight = transition else { return }
        transition = nil
        if inFlight.popsOnCompletion, !path.isEmpty { path.removeLast() }
    }

    private func spring(to target: Float) {
        LayerTransitionRegistry.shared.settle(ObjectIdentifier(self)) { [weak self] dt in
            guard let self, var state = self.transition else { return false }
            let stepped = criticallyDampedStep(
                value: state.progress, velocity: state.velocity,
                target: target, response: self.transitionStyle.response, dt: dt
            )
            state.progress = stepped.value
            state.velocity = stepped.velocity
            guard stepped.settled else {
                self.transition = state
                self.changed()
                return true
            }
            self.transition = nil

            if state.popsOnCompletion, !self.path.isEmpty { self.path.removeLast() }
            self.changed()
            return false
        }
    }

    public func beginInteractiveBack() {
        updateInteractiveBack(0)
    }

    public func updateInteractiveBack(_ progress: Float) {
        guard !path.isEmpty else { return }
        LayerTransitionRegistry.shared.cancel(ObjectIdentifier(self))
        transition = LayerTransition(
            progress: min(max(progress, 0), 1), velocity: 0, popsOnCompletion: false)
        changed()
    }

    public func endInteractiveBack(commit: Bool, velocity: Float = 0) {
        guard transition != nil else { return }
        transition?.velocity = velocity
        transition?.popsOnCompletion = commit
        spring(to: commit ? 1 : 0)
    }

    public func present<Value: Sendable>(
        _ view: any View,
        as style: NavPresentation = .sheet,
        returning: Value.Type
    ) async -> Value? {
        let token = ModalResults.shared.nextToken()
        counter &+= 1
        modals.append(
            Modal(
                id: counter, view: view, style: style, token: token,
                morphSource: NodeFrames.shared.lastPressed
            )
        )
        changed()
        return await ModalResults.shared.wait(token, as: Value.self)
    }

    public func dismiss<Value: Sendable>(returning value: Value) {
        guard !modals.isEmpty else { return }
        let modal = modals.removeLast()
        ModalResults.shared.resume(modal.token, with: value)
        changed()
    }

    private func changed() {
        NodeRegistry.shared.needsRender = true
    }
}

/// Pushes, presents and dismisses screens, from anywhere.
///
/// Acts on the navigation stack of whichever tab is selected.
public enum Navigator {

    nonisolated(unsafe) public static var tabSelection: Int = 0 {
        didSet {
            guard tabSelection != oldValue else { return }
            NodeRegistry.shared.needsRender = true
        }
    }

    nonisolated(unsafe) private static var stacks: [Int: NavigationStackModel] = [:]

    nonisolated(unsafe) static var tabTitles: [String] = []

    public static func stack(_ tab: Int) -> NavigationStackModel {
        if let existing = stacks[tab] { return existing }
        let created = NavigationStackModel()
        stacks[tab] = created
        return created
    }

    public static var current: NavigationStackModel { stack(tabSelection) }

    @discardableResult
    public static func push(_ view: any View) -> UInt64 { current.push(view) }

    @discardableResult
    public static func present(
        _ view: any View,
        as style: NavPresentation = .sheet,
        morphingFrom morphSource: String? = nil
    ) -> UInt64 {
        current.present(view, as: style, morphingFrom: morphSource)
    }

    public static func pop() { current.pop() }
    public static func popToRoot() { current.popToRoot() }
    public static func pop(to id: UInt64) { current.pop(to: id) }
    public static func setPath(_ views: [any View]) { current.setPath(views) }

    public static var isAtRoot: Bool { current.isAtRoot }
    public static var isPresenting: Bool { current.isPresenting }
    public static var transitionProgress: Float? { current.transitionProgress }

    public static func present<Value: Sendable>(
        _ view: any View,
        as style: NavPresentation = .sheet,
        returning: Value.Type
    ) async -> Value? {
        await current.present(view, as: style, returning: returning)
    }

    public static func dismiss<Value: Sendable>(returning value: Value) {
        current.dismiss(returning: value)
    }

    public static func dismiss() { current.pop() }

    public static func select(_ tab: Int, then action: ((NavigationStackModel) -> Void)? = nil) {
        tabSelection = tab
        if let action { action(stack(tab)) }
    }

    public static func select(
        titled title: String, then action: ((NavigationStackModel) -> Void)? = nil
    ) {
        guard let index = tabTitles.firstIndex(of: title) else { return }
        select(index, then: action)
    }

    public static func reset() {
        for (_, stack) in stacks { stack.popToRoot() }
        stacks.removeAll()
        tabSelection = 0
        NodeRegistry.shared.needsRender = true
    }
}

final class ModalResults {
    nonisolated(unsafe) static let shared = ModalResults()

    nonisolated(unsafe) private var waiting:
        [UInt64: CheckedContinuation<(any Sendable)?, Never>] = [:]
    nonisolated(unsafe) private var counter: UInt64 = 0

    func nextToken() -> UInt64 {
        counter &+= 1
        return counter
    }

    func wait<Value: Sendable>(_ token: UInt64, as: Value.Type) async -> Value? {
        let result: (any Sendable)? = await withCheckedContinuation { continuation in
            waiting[token] = continuation
        }
        return result as? Value
    }

    func resume(_ token: UInt64, with value: (any Sendable)?) {
        guard let continuation = waiting.removeValue(forKey: token) else { return }
        continuation.resume(returning: value)
    }

    func cancel(_ token: UInt64) {
        guard token != 0 else { return }
        resume(token, with: nil)
    }
}

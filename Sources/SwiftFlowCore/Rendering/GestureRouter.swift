import CSwiftFlow

enum PressPhase {

    case undecided

    case drag

    case scroll
}

public final class GestureRouter {
    nonisolated(unsafe)
    public static let shared = GestureRouter()

    public static let multiTapInterval: Double = 0.3

    private static let tapSlop: Float = 20

    private static let scrollActivation: Float = 8

    private struct Press {
        var startX: Float
        var startY: Float
        var lastX: Float
        var lastY: Float
        var lastTime: Double
        var velocityX: Float = 0
        var velocityY: Float = 0
        var phase: PressPhase = .undecided

        var dragNode: UInt32 = 0
        var pressNode: UInt32 = 0
        var tapNode: UInt32 = 0

        var scrolls: [ScrollCandidate] = []

        var scrollID: UInt32 = 0

        var scrollLastX: Float = 0
        var scrollLastY: Float = 0
        var scrollLastTime: Double = 0
    }

    private var press: Press?

    private var lastTapNode: UInt32 = 0
    private var lastTapTime: Double = 0
    private var tapCount: Int = 0

    public var needsFrames: Bool {
        GestureRegistry.shared.presses.values.contains { !$0.fired }
    }

    public func pointerDown(
        x: Float, y: Float, t: Double, path: [UInt32], scrolls: [ScrollCandidate]
    ) {
        let registry = GestureRegistry.shared
        var press = Press(startX: x, startY: y, lastX: x, lastY: y, lastTime: t)
        press.scrolls = scrolls

        let scale = DeviceScale.current > 0 ? DeviceScale.current : 1
        for node in path {

            if press.dragNode == 0,
               let drag = registry.drags[node],
               drag.accepts(startX: x / scale, startY: y / scale) {
                press.dragNode = node
            }
            if press.pressNode == 0, registry.longPresses[node] != nil { press.pressNode = node }
            if press.tapNode == 0, hasTap(node) { press.tapNode = node }
        }

        if press.tapNode != 0 {
            NodeRegistry.shared.pressedNodes.insert(press.tapNode)
            NodeRegistry.shared.markDirty(press.tapNode)

            NodeFrames.shared.notePressed(press.tapNode)
        }
        if let gesture = registry.longPresses[press.pressNode] {
            registry.presses[gesture.id] = PressProgress(elapsed: 0, fired: false)
            NodeRegistry.shared.needsRender = true
        }

        self.press = press
    }

    public func pointerMoved(x: Float, y: Float, t: Double) {
        guard var press = self.press else { return }
        let scale = DeviceScale.current > 0 ? DeviceScale.current : 1
        let registry = GestureRegistry.shared

        let dx = x - press.startX
        let dy = y - press.startY
        let travelled = (dx * dx + dy * dy).squareRoot() / scale

        let dt = t - press.lastTime
        if dt > 0 {
            let smoothing: Float = 0.8
            let instantX = (x - press.lastX) / Float(dt)
            let instantY = (y - press.lastY) / Float(dt)
            press.velocityX = press.velocityX * smoothing + instantX * (1 - smoothing)
            press.velocityY = press.velocityY * smoothing + instantY * (1 - smoothing)
        }
        press.lastX = x
        press.lastY = y
        press.lastTime = t

        if !hasFired(press), let gesture = registry.longPresses[press.pressNode],
           travelled > gesture.maximumDistance {
            registry.presses.removeValue(forKey: gesture.id)
        }

        if press.phase == .undecided {
            if let gesture = registry.drags[press.dragNode] {

                if travelled >= gesture.minimumDistance {
                    if gesture.axis.claims(dx: dx, dy: dy) {
                        press.phase = .drag
                        cancelTap(&press)
                        if !hasFired(press) { cancelLongPress(&press) }
                    } else {

                        claimScroll(&press, dx: dx, dy: dy, x: x, y: y, t: t)
                    }
                }
            } else if travelled > Self.scrollActivation {
                claimScroll(&press, dx: dx, dy: dy, x: x, y: y, t: t)
            }
        }

        if press.phase == .drag, let gesture = registry.drags[press.dragNode] {
            let value = makeValue(press, x: x, y: y, scale: scale)
            registry.dragValues[gesture.id] = value
            gesture.changed?(value)
            NodeRegistry.shared.needsRender = true
        }

        if press.phase == .scroll, press.scrollID != 0 {
            driveScroll(&press, x: x, y: y, t: t)
        }

        self.press = press
    }

    private func claimScroll(
        _ press: inout Press, dx: Float, dy: Float, x: Float, y: Float, t: Double
    ) {
        press.phase = .scroll
        cancelTap(&press)
        cancelLongPress(&press)

        let horizontal = abs(dx) > abs(dy)
        let along = horizontal ? dx : dy

        var bouncer: ScrollCandidate?

        for candidate in press.scrolls {
            let wantsHorizontal = candidate.axis == .horizontal
            guard wantsHorizontal == horizontal else { continue }

            let state = NodeRegistry.shared.scrollState(for: candidate.id)

            guard state.maxOffset > 0 else { continue }
            if bouncer == nil { bouncer = candidate }

            guard canScroll(state, by: along) else { continue }
            begin(&press, on: candidate, x: x, y: y, t: t)
            return
        }

        if let bouncer {
            begin(&press, on: bouncer, x: x, y: y, t: t)
            return
        }

    }

    private func begin(
        _ press: inout Press, on candidate: ScrollCandidate, x: Float, y: Float, t: Double
    ) {
        NodeRegistry.shared.scrollState(for: candidate.id).beginDrag()
        press.scrollID = candidate.id
        press.scrollLastX = x
        press.scrollLastY = y
        press.scrollLastTime = t
    }

    private func canScroll(_ state: ScrollPhysicsState, by delta: Float) -> Bool {
        guard state.maxOffset > 0 else { return false }
        if delta < 0 { return state.offset < state.maxOffset - 0.5 }
        if delta > 0 { return state.offset > 0.5 }
        return false
    }

    private func driveScroll(_ press: inout Press, x: Float, y: Float, t: Double) {
        let state = NodeRegistry.shared.scrollState(for: press.scrollID)
        let delta = state.axis == .horizontal
            ? x - press.scrollLastX
            : y - press.scrollLastY
        state.applyDrag(delta: delta)

        let dt = t - press.scrollLastTime
        if dt > 0 {

            let instant = delta / Float(dt)
            let smoothing: Float = 0.8
            state.velocity = state.velocity * smoothing + instant * (1 - smoothing)
        }

        press.scrollLastX = x
        press.scrollLastY = y
        press.scrollLastTime = t
    }

    public func pointerUp(x: Float, y: Float, t: Double) {
        guard var press = self.press else { return }
        self.press = nil
        let scale = DeviceScale.current > 0 ? DeviceScale.current : 1
        let registry = GestureRegistry.shared

        if press.phase == .drag, let gesture = registry.drags[press.dragNode] {
            let value = makeValue(press, x: x, y: y, scale: scale)
            registry.dragValues.removeValue(forKey: gesture.id)
            gesture.ended?(value)
            NodeRegistry.shared.needsRender = true
        }

        cancelLongPress(&press)
        settleScroll(&press)

        guard press.tapNode != 0 else { return }
        NodeRegistry.shared.pressedNodes.remove(press.tapNode)
        NodeRegistry.shared.markDirty(press.tapNode)
        guard press.phase != .drag else { return }

        let dx = (x - press.startX) / scale
        let dy = (y - press.startY) / scale
        guard (dx * dx + dy * dy).squareRoot() < Self.tapSlop else { return }

        fireTap(on: press.tapNode, x: x, y: y, t: t, scale: scale)
    }

    public func pointerCancelled() {
        guard var press = self.press else { return }
        self.press = nil
        let registry = GestureRegistry.shared

        if let gesture = registry.drags[press.dragNode] {
            registry.dragValues.removeValue(forKey: gesture.id)
        }
        cancelLongPress(&press)
        cancelTap(&press)
        settleScroll(&press)
    }

    public func step(dt: Float) {
        let registry = GestureRegistry.shared
        guard let press = self.press,
              let gesture = registry.longPresses[press.pressNode],
              var progress = registry.presses[gesture.id]
        else { return }

        guard !progress.fired else { return }

        progress.elapsed += dt
        progress.fired = progress.elapsed >= gesture.minimumDuration
        registry.presses[gesture.id] = progress
        NodeRegistry.shared.needsRender = true

        if progress.fired {
            gesture.ended?()
        }
    }

    private func hasFired(_ press: Press) -> Bool {
        guard let gesture = GestureRegistry.shared.longPresses[press.pressNode] else {
            return false
        }
        return GestureRegistry.shared.presses[gesture.id]?.fired ?? false
    }

    private func hasTap(_ node: UInt32) -> Bool {
        GestureRegistry.shared.taps[node] != nil
            || NodeRegistry.shared.tapActions[node] != nil
    }

    private func fireTap(on node: UInt32, x: Float, y: Float, t: Double, scale: Float) {
        if node == lastTapNode, t - lastTapTime < Self.multiTapInterval {
            tapCount += 1
        } else {
            tapCount = 1
        }
        lastTapNode = node
        lastTapTime = t

        NodeRegistry.shared.triggerTap(node)

        if let handler = GestureRegistry.shared.taps[node], handler.count == tapCount {
            handler.action(Point(x: x / scale, y: y / scale))

            tapCount = 0
        }
    }

    private func makeValue(_ press: Press, x: Float, y: Float, scale: Float) -> DragValue {
        DragValue(
            startLocation: Point(x: press.startX / scale, y: press.startY / scale),
            location: Point(x: x / scale, y: y / scale),
            translation: Point(
                x: (x - press.startX) / scale,
                y: (y - press.startY) / scale
            ),
            velocity: Point(x: press.velocityX / scale, y: press.velocityY / scale)
        )
    }

    private func cancelTap(_ press: inout Press) {
        guard press.tapNode != 0 else { return }
        NodeRegistry.shared.pressedNodes.remove(press.tapNode)
        NodeRegistry.shared.markDirty(press.tapNode)
        press.tapNode = 0
    }

    private func cancelLongPress(_ press: inout Press) {
        if let gesture = GestureRegistry.shared.longPresses[press.pressNode] {
            GestureRegistry.shared.presses.removeValue(forKey: gesture.id)
        }
        press.pressNode = 0
    }

    private func settleScroll(_ press: inout Press) {
        guard press.scrollID != 0 else { return }
        let state = NodeRegistry.shared.scrollState(for: press.scrollID)
        state.isDragging = false
        state.isSettling = true
        press.scrollID = 0
    }

    public func wheel(x: Float, y: Float, dx: Float, dy: Float, scrolls: [ScrollCandidate]) {
        let horizontal = abs(dx) > abs(dy)
        let delta = horizontal ? dx : dy
        guard delta != 0 else { return }

        for candidate in scrolls where (candidate.axis == .horizontal) == horizontal {
            let state = NodeRegistry.shared.scrollState(for: candidate.id)

            guard canScroll(state, by: -delta) else { continue }
            state.applyWheel(delta: delta)
            NodeRegistry.shared.needsRender = true
            return
        }
    }
}

public struct ScrollCandidate: Sendable, Equatable {
    public let id: UInt32
    public let axis: Axis

    public init(id: UInt32, axis: Axis) {
        self.id = id
        self.axis = axis
    }
}

extension NodeBuilder {

    public static func scrollPath(_ root: inout SFNode, x: Float, y: Float) -> [ScrollCandidate] {
        var buffer = [SFScrollHit](repeating: SFScrollHit(scrollId: 0, axis: 0), count: 8)
        let count = buffer.withUnsafeMutableBufferPointer { buf in
            sf_hit_test_scroll_path(&root, x, y, buf.baseAddress, buf.count)
        }
        return buffer.prefix(count).map {

            ScrollCandidate(id: $0.scrollId, axis: $0.axis == 1 ? .horizontal : .vertical)
        }
    }

    public static func hitPath(_ root: inout SFNode, x: Float, y: Float) -> [UInt32] {
        var buffer = [UInt32](repeating: 0, count: 32)
        let count = buffer.withUnsafeMutableBufferPointer { buf in
            sf_hit_test_path(&root, x, y, buf.baseAddress, buf.count)
        }
        return Array(buffer.prefix(count))
    }
}

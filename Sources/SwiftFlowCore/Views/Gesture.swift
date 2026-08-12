import CSwiftFlow

public struct Point: Sendable, Equatable {
    public var x: Float
    public var y: Float

    public init(x: Float, y: Float) {
        self.x = x
        self.y = y
    }

    public static let zero = Point(x: 0, y: 0)

    public var magnitude: Float { (x * x + y * y).squareRoot() }
}

public struct DragValue: Sendable, Equatable {

    public let startLocation: Point

    public let location: Point

    public let translation: Point

    public let velocity: Point

    public var predictedEndTranslation: Point {
        Point(
            x: translation.x + velocity.x * 0.166,
            y: translation.y + velocity.y * 0.166
        )
    }

    public static let zero = DragValue(
        startLocation: .zero, location: .zero, translation: .zero, velocity: .zero
    )
}

/// A dragging motion along one axis.
///
/// Inside a scroll view the first movement decides: a drag that starts
/// across the scroll's axis is the gesture's for its whole life, and one
/// that starts along it stays a scroll.
public struct DragGesture {

    public enum Axis: Sendable {

        case all
        case horizontal
        case vertical

        func claims(dx: Float, dy: Float) -> Bool {
            switch self {
            case .all: return true
            case .horizontal: return abs(dx) > abs(dy)
            case .vertical: return abs(dy) > abs(dx)
            }
        }
    }

    let id: UInt32
    let axis: Axis
    let minimumDistance: Float

    let edge: Edge?
    let edgeWidth: Float
    var changed: ((DragValue) -> Void)?
    var ended: ((DragValue) -> Void)?

    public init(
        axis: Axis = .all,
        minimumDistance: Float = 8,
        edge: Edge? = nil,
        edgeWidth: Float = 20,
        fileID: String = #fileID, line: Int = #line, column: Int = #column
    ) {
        self.id = fnv1a("drag:\(fileID):\(line):\(column)")
        self.axis = axis
        self.minimumDistance = minimumDistance
        self.edge = edge
        self.edgeWidth = edgeWidth
    }

    func accepts(startX: Float, startY: Float) -> Bool {
        guard let edge else { return true }
        let w = DeviceMetrics.screenWidth
        let h = DeviceMetrics.screenHeight
        if edge.contains(.leading), startX <= edgeWidth { return true }
        if edge.contains(.trailing), w > 0, startX >= w - edgeWidth { return true }
        if edge.contains(.top), startY <= edgeWidth { return true }
        if edge.contains(.bottom), h > 0, startY >= h - edgeWidth { return true }
        return false
    }

    public func onChanged(_ action: @escaping (DragValue) -> Void) -> DragGesture {
        var copy = self
        copy.changed = action
        return copy
    }

    public func onEnded(_ action: @escaping (DragValue) -> Void) -> DragGesture {
        var copy = self
        copy.ended = action
        return copy
    }

    public var value: DragValue? {
        GestureRegistry.shared.dragValues[id]
    }

    public var isActive: Bool { value != nil }

    public var translation: Point { value?.translation ?? .zero }

    public var velocity: Point { value?.velocity ?? .zero }
}

/// A press held longer than a tap.
public struct LongPressGesture {
    let id: UInt32
    let minimumDuration: Float
    let maximumDistance: Float
    var ended: (() -> Void)?

    public init(
        minimumDuration: Float = 0.5,
        maximumDistance: Float = 10,
        fileID: String = #fileID, line: Int = #line, column: Int = #column
    ) {
        self.id = fnv1a("press:\(fileID):\(line):\(column)")
        self.minimumDuration = minimumDuration
        self.maximumDistance = maximumDistance
    }

    public func onEnded(_ action: @escaping () -> Void) -> LongPressGesture {
        var copy = self
        copy.ended = action
        return copy
    }

    public var isPressing: Bool {
        GestureRegistry.shared.presses[id] != nil
    }

    public var progress: Float {
        guard let press = GestureRegistry.shared.presses[id] else { return 0 }
        guard minimumDuration > 0 else { return 1 }
        return min(press.elapsed / minimumDuration, 1)
    }

    public var hasFired: Bool {
        GestureRegistry.shared.presses[id]?.fired ?? false
    }
}

public struct GestureModifier<Content: View>: View {
    public typealias Body = Never
    public var body: Never { fatalError() }
    let content: Content
    let attach: (UInt32) -> Void
}

extension GestureModifier {
    public func toSFNode() -> SFNode {
        var node = content.toSFNode()

        let id = node.node_id != 0 ? node.node_id : BuildContext.shared.currentID(for: self)
        node.node_id = id
        attach(id)
        return node
    }
}

extension View {

    /// Attaches a drag gesture to this view.
    public func gesture(_ gesture: DragGesture) -> GestureModifier<Self> {
        GestureModifier(content: self) { node in
            GestureRegistry.shared.drags[node] = gesture
        }
    }

    /// Attaches a long-press gesture to this view.
    public func gesture(_ gesture: LongPressGesture) -> GestureModifier<Self> {
        GestureModifier(content: self) { node in
            GestureRegistry.shared.longPresses[node] = gesture
        }
    }

    /// Adds an action to perform when this view is tapped.
    public func onTap(
        count: Int = 1,
        perform action: @escaping (Point) -> Void
    ) -> GestureModifier<Self> {
        GestureModifier(content: self) { node in
            GestureRegistry.shared.taps[node] = TapHandler(count: count, action: action)
        }
    }
}

struct TapHandler {
    let count: Int
    let action: (Point) -> Void
}

struct PressProgress {
    var elapsed: Float
    var fired: Bool
}

final class GestureRegistry {
    nonisolated(unsafe) static let shared = GestureRegistry()

    nonisolated(unsafe) var drags: [UInt32: DragGesture] = [:]
    nonisolated(unsafe) var longPresses: [UInt32: LongPressGesture] = [:]
    nonisolated(unsafe) var taps: [UInt32: TapHandler] = [:]

    nonisolated(unsafe) var dragValues: [UInt32: DragValue] = [:]
    nonisolated(unsafe) var presses: [UInt32: PressProgress] = [:]

    func rekey(from old: UInt32, to new: UInt32) {
        if let drag = drags.removeValue(forKey: old) { drags[new] = drag }
        if let press = longPresses.removeValue(forKey: old) { longPresses[new] = press }
        if let tap = taps.removeValue(forKey: old) { taps[new] = tap }
    }

    func beginBuild() {
        for id in taps.keys { NodeFrames.shared.register(id) }
        for id in drags.keys { NodeFrames.shared.register(id) }
        drags.removeAll(keepingCapacity: true)
        longPresses.removeAll(keepingCapacity: true)
        taps.removeAll(keepingCapacity: true)
    }
}

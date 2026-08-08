import CSwiftFlow

/// One button behind a swiped view.
public struct SwipeAction {
    public enum Role: Sendable {
        case normal

        case destructive
    }

    let title: String
    let icon: Icon?
    let tint: Color
    let role: Role
    let action: () -> Void

    public init(
        _ title: String,
        icon: Icon? = nil,
        tint: Color = .accent,
        role: Role = .normal,
        action: @escaping () -> Void
    ) {
        self.title = title
        self.icon = icon
        self.tint = tint
        self.role = role
        self.action = action
    }
}

/// Which side a swipe reveals its actions from.
public enum SwipeEdge: Sendable {
    case leading
    case trailing

    var sign: Float { self == .trailing ? -1 : 1 }
}

@resultBuilder
public struct SwipeActionBuilder {
    public static func buildBlock(_ actions: SwipeAction...) -> [SwipeAction] { actions }
    public static func buildOptional(_ actions: [SwipeAction]?) -> [SwipeAction] { actions ?? [] }
    public static func buildEither(first: [SwipeAction]) -> [SwipeAction] { first }
    public static func buildEither(second: [SwipeAction]) -> [SwipeAction] { second }
    public static func buildArray(_ parts: [[SwipeAction]]) -> [SwipeAction] { parts.flatMap { $0 } }
}

enum SwipeMetrics {

    static let buttonWidth: Float = 74
    static let iconSize: Float = 22
    static let cornerRadius: Float = 14

    static let spacing: Float = 6

    static let liftCornerRadius: Float = 12

    static let liftResponse: Float = 0.18
}

final class SwipeActionState {
    var offset: Float = 0
    var velocity: Float = 0

    var target: Float = 0
    var isDragging = false

    var lift: Float = 0
    var liftTarget: Float = 0

    func step(dt: Float) {
        guard dt > 0 else { return }

        if abs(liftTarget - lift) > 0.001 {
            let rate = min(1, dt / SwipeMetrics.liftResponse)
            lift += (liftTarget - lift) * rate
            NodeRegistry.shared.needsRender = true
        } else {
            lift = liftTarget
        }

        guard !isDragging else { return }
        let distance = target - offset
        if abs(distance) < 0.1 && abs(velocity) < 1 {
            offset = target
            velocity = 0

            if target == 0 { liftTarget = 0 }
            return
        }

        let omega = 2 * Float.pi / SwipeActionPhysics.response
        let acceleration = omega * omega * distance - 2 * omega * velocity
        velocity += acceleration * dt
        offset += velocity * dt
        NodeRegistry.shared.needsRender = true
    }

    var isOpen: Bool { abs(offset) > 0.5 }
}

final class SwipeActionRegistry {
    nonisolated(unsafe) static let shared = SwipeActionRegistry()

    nonisolated(unsafe) private var states: [UInt32: SwipeActionState] = [:]

    func state(for id: UInt32) -> SwipeActionState {
        if let existing = states[id] { return existing }
        let created = SwipeActionState()
        states[id] = created
        return created
    }

    func step(dt: Float) {
        for (_, state) in states { state.step(dt: dt) }
    }

    func closeAll(except id: UInt32) {
        for (key, state) in states where key != id && state.isOpen {
            state.target = 0
        }
    }

    func closeAll() {
        for (_, state) in states where state.isOpen {
            state.target = 0
        }
    }
}

public struct SwipeActionsModifier<Content: View>: View {
    public typealias Body = Never
    public var body: Never { fatalError() }

    let content: Content
    let edge: SwipeEdge
    let actions: [SwipeAction]

    let swipeID: UInt32

    var openWidth: Float {
        guard !actions.isEmpty else { return 0 }
        return Float(actions.count) * (SwipeMetrics.buttonWidth + SwipeMetrics.spacing)
            + SwipeMetrics.spacing
    }

    public func toSFNode() -> SFNode {
        guard !actions.isEmpty else { return content.toSFNode() }

        let state = SwipeActionRegistry.shared.state(for: swipeID)
        let open = openWidth
        let hasDestructive = actions.contains { $0.role == .destructive }

        NodeFrames.shared.register(swipeID)
        let containerWidth =
            (NodeFrames.shared.frame(for: swipeID)?.inPoints().width) ?? 0

        let gesture = DragGesture(axis: .horizontal, minimumDistance: 12)
            .onChanged { value in
                let state = SwipeActionRegistry.shared.state(for: swipeID)
                if !state.isDragging {
                    state.isDragging = true
                    state.liftTarget = 1
                    SwipeActionRegistry.shared.closeAll(except: swipeID)
                }

                let base = state.target
                state.offset = SwipeActionPhysics.offset(
                    translation: base + value.translation.x,
                    openWidth: open,
                    sign: edge.sign
                )
                NodeRegistry.shared.needsRender = true
            }
            .onEnded { value in
                let state = SwipeActionRegistry.shared.state(for: swipeID)
                state.isDragging = false
                state.velocity = value.velocity.x

                if SwipeActionPhysics.isFullSwipe(
                    offset: state.offset,
                    containerWidth: containerWidth,
                    hasDestructiveAction: hasDestructive
                ), let destructive = actions.first(where: { $0.role == .destructive }) {
                    state.target = 0
                    destructive.action()
                    return
                }

                state.target = SwipeActionPhysics.restingOffset(
                    offset: state.offset,
                    velocity: state.velocity,
                    openWidth: open,
                    sign: edge.sign
                )
            }

        let row = ZStack(alignment: edge == .trailing ? .trailing : .leading) {
            SwipeActionButtons(
                actions: actions,
                edge: edge,
                revealed: min(1, abs(state.offset) / max(open, 1))
            )

            RoundedRectangle(
                cornerRadius: 0
            )
            .fill(.background)
            .mergeable(false)
            .offset(x: state.offset)

            content
                .background(
                    RoundedRectangle(
                        cornerRadius: SwipeMetrics.liftCornerRadius * state.lift
                    )
                    .fill(Color.fill.opacity(state.lift))
                )
                .offset(x: state.offset)
                .gesture(gesture)
        }

        var node = row.toSFNode()
        node.node_id = swipeID
        return node
    }
}

struct SwipeActionButtons: View {
    let actions: [SwipeAction]
    let edge: SwipeEdge

    let revealed: Float

    var body: some View {

        HStack(alignment: .center, spacing: SwipeMetrics.spacing) {
            if edge == .trailing { Spacer() }
            ForEach(actions.indices, id: \.self) { index in
                SwipeActionButton(action: actions[index], revealed: revealed)
            }
            if edge == .leading { Spacer() }
        }
        .expands()
        .padding(
            EdgeInsets(
                top: 0, bottom: 0,
                leading: SwipeMetrics.spacing,
                trailing: SwipeMetrics.spacing
            )
        )
    }
}

struct SwipeActionButton: View {
    let action: SwipeAction
    let revealed: Float

    var body: some View {
        let label = VStack(alignment: .center, spacing: 4) {
            if let icon = action.icon {
                icon.size(SwipeMetrics.iconSize).foregroundColor(.white)
            }
            Text(action.title)
                .font(.caption)
                .foregroundColor(.white)
        }

        .frame(width: SwipeMetrics.buttonWidth, maxHeight: .infinity)
        .padding(EdgeInsets(top: 10, bottom: 10, leading: 0, trailing: 0))

        return label
            .background(
                RoundedRectangle(cornerRadius: SwipeMetrics.cornerRadius)
                    .fill(action.tint)
                    .specular()
            )

            .scale(0.6 + 0.4 * revealed)
            .opacity(revealed)
            .onTap { _ in
                SwipeActionRegistry.shared.closeAll()
                action.action()
            }
    }
}

extension View {

    /// Adds actions revealed by swiping this view sideways.
    ///
    /// Works on any view inside a scrollable container, not only on `List` rows.
    public func swipeActions(
        edge: SwipeEdge = .trailing,
        fileID: String = #fileID, line: Int = #line, column: Int = #column,
        @SwipeActionBuilder actions: () -> [SwipeAction]
    ) -> SwipeActionsModifier<Self> {
        SwipeActionsModifier(
            content: self,
            edge: edge,
            actions: actions(),
            swipeID: fnv1a("swipe:\(fileID):\(line):\(column)")
        )
    }
}

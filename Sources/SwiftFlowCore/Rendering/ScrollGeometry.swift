import CSwiftFlow

public struct ScrollGeometry: Sendable, Equatable {

    public let offset: Float

    public let contentLength: Float

    public let viewportLength: Float

    public let velocity: Float

    public let isDragging: Bool

    public let axis: Axis

    public init(
        offset: Float,
        contentLength: Float,
        viewportLength: Float,
        velocity: Float,
        isDragging: Bool,
        axis: Axis = .vertical
    ) {
        self.offset = offset
        self.contentLength = contentLength
        self.viewportLength = viewportLength
        self.velocity = velocity
        self.isDragging = isDragging
        self.axis = axis
    }

    public static let zero = ScrollGeometry(
        offset: 0, contentLength: 0, viewportLength: 0, velocity: 0, isDragging: false
    )

    public var maxOffset: Float {
        max(0, contentLength - viewportLength)
    }

    public var progress: Float {
        let range = maxOffset
        guard range > 0 else { return 0 }
        return min(max(offset / range, 0), 1)
    }

    public var overscroll: Float {
        if offset < 0 { return offset }
        let range = maxOffset
        if offset > range { return offset - range }
        return 0
    }

    public var isScrollable: Bool { maxOffset > 0 }
}

extension ScrollGeometry {

    init(_ state: ScrollPhysicsState) {

        let scale = DeviceScale.current > 0 ? DeviceScale.current : 1
        self.init(
            offset: state.offset / scale,
            contentLength: state.contentLength / scale,
            viewportLength: state.viewportLength / scale,
            velocity: -state.velocity / scale,
            isDragging: state.isDragging,
            axis: state.axis
        )
    }

    static func id(forName name: String) -> UInt32 {
        fnv1a("scroll:\(name)")
    }

    public init(name: String) {
        guard let state = NodeRegistry.shared.scrollStates[Self.id(forName: name)] else {
            self = .zero
            return
        }
        self = ScrollGeometry(state)
    }
}

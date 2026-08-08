import CSwiftFlow

/// A rectangle with rounded corners. A radius of -1 rounds it fully.
public struct RoundedRectangle: View {
    public typealias Body = Never
    public var body: Never { fatalError() }
    public let cornerRadius: Float

    public init(cornerRadius: Float) {
        self.cornerRadius = cornerRadius
    }
}

extension RoundedRectangle {
    public func toSFNode() -> SFNode {
        var node = SFNode()
        node.kind = SF_NODE_RECT
        node.fill = SFColor(r: 0, g: 0, b: 0, a: 0)
        node.cornerRadius = cornerRadius * DeviceScale.current
        node.sizing = SF_SIZING_FILL
        return node
    }
}

/// A circle inscribed in its frame.
public struct Circle: View {
    public typealias Body = Never
    public var body: Never { fatalError() }
    public init() {}
}

extension Circle {
    public func toSFNode() -> SFNode {
        var node = SFNode()
        node.kind = SF_NODE_RECT
        node.fill = SFColor(r: 0, g: 0, b: 0, a: 0)
        node.cornerRadius = -1
        node.sizing = SF_SIZING_FILL
        return node
    }
}

/// A capsule — a rectangle with fully rounded ends.
public struct Capsule: View {
    public typealias Body = Never
    public var body: Never { fatalError() }
    public init() {}
}

extension Capsule {
    public func toSFNode() -> SFNode {
        var node = SFNode.makeDefault()
        node.kind = SF_NODE_RECT
        node.fill = SFColor(r: 0, g: 0, b: 0, a: 0)
        node.cornerRadius = -1
        node.sizing = SF_SIZING_FILL
        return node
    }
}

/// A flexible gap along the enclosing stack's main axis.
public struct Spacer: View {
    public typealias Body = Never
    public var body: Never { fatalError() }
    public let minLength: Float

    public init(minLength: Float = 0) {
        self.minLength = minLength
    }
}

extension Spacer {
    public func toSFNode() -> SFNode {
        var node = SFNode.makeDefault()
        node.kind = SF_NODE_SPACER
        node.minLength = minLength
        node.sizing = SF_SIZING_FILL
        return node
    }
}

/// A hairline between two views.
public struct Divider: View {
    public typealias Body = Never
    public var body: Never { fatalError() }
    public init() {}
}

extension Divider {
    public func toSFNode() -> SFNode {
        var node = SFNode.makeDefault()
        node.kind = SF_NODE_RECT
        node.fill = Color.border.toSFColor()
        node.fixedHeight = 1
        node.sizing = SF_SIZING_FILL
        return node
    }
}

func shapeCornerRadius(_ shape: any View) -> Float? {
    if let rr = shape as? RoundedRectangle {
        return rr.cornerRadius * DeviceScale.current
    }
    if shape is Circle || shape is Capsule {
        return -1
    }
    return nil
}

public struct FilledShape<S: View>: View {
    public typealias Body = Never
    public var body: Never { fatalError() }
    let shape: S
    let fill: Color
}

extension FilledShape {
    public func toSFNode() -> SFNode {
        var node = SFNode.makeDefault()
        node.kind = SF_NODE_RECT
        node.fill = fill.toSFColor()
        node.sizing = SF_SIZING_FILL
        node.cornerRadius = shapeCornerRadius(shape) ?? 0
        return node
    }
}

public struct StrokedShape<S: View>: View {
    public typealias Body = Never
    public var body: Never { fatalError() }
    let shape: S
    let color: Color
    let lineWidth: Float
}

extension StrokedShape {
    public func toSFNode() -> SFNode {
        var node = SFNode.makeDefault()
        node.kind = SF_NODE_RECT
        node.fill = SFColor(r: 0, g: 0, b: 0, a: 0)
        node.border = SFBorder(
            color: color.toSFColor(),
            width: lineWidth,
            _pad: (0, 0, 0)
        )
        node.sizing = SF_SIZING_FILL
        node.cornerRadius = shapeCornerRadius(shape) ?? 0
        return node
    }
}

extension RoundedRectangle {
    public func fill(_ color: Color) -> FilledShape<RoundedRectangle> {
        FilledShape(shape: self, fill: color)
    }
    public func stroke(_ color: Color, lineWidth: Float = 1) -> StrokedShape<RoundedRectangle> {
        StrokedShape(shape: self, color: color, lineWidth: lineWidth)
    }
}

extension Circle {
    public func fill(_ color: Color) -> FilledShape<Circle> {
        FilledShape(shape: self, fill: color)
    }
    public func stroke(_ color: Color, lineWidth: Float = 1) -> StrokedShape<Circle> {
        StrokedShape(shape: self, color: color, lineWidth: lineWidth)
    }
}

extension Capsule {
    public func fill(_ color: Color) -> FilledShape<Capsule> {
        FilledShape(shape: self, fill: color)
    }
    public func stroke(_ color: Color, lineWidth: Float = 1) -> StrokedShape<Capsule> {
        StrokedShape(shape: self, color: color, lineWidth: lineWidth)
    }
}

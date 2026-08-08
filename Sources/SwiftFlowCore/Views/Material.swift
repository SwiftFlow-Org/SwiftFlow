import CSwiftFlow

/// A backdrop blur that shows what is behind it, tinted.
public struct Material: Sendable {
    public let blurRadius: Float
    public let tint: Color

    public let isProgressive: Bool

    public let progressiveStart: Float

    public init(
        blurRadius: Float,
        tint: Color,
        isProgressive: Bool = false,
        progressiveStart: Float = 0.5
    ) {
        self.blurRadius = blurRadius
        self.tint = tint
        self.isProgressive = isProgressive
        self.progressiveStart = progressiveStart
    }

    /// Ramps this material's blur out below `start`, measured 0…1 down the shape.
    public func progressive(start: Float = 0.5) -> Material {
        Material(
            blurRadius: blurRadius,
            tint: tint,
            isProgressive: true,
            progressiveStart: start
        )
    }

    public static let ultraThinMaterial = Material(blurRadius: 3, tint: Color(r: 1, g: 1, b: 1, a: 0.08))
    public static let thinMaterial = Material(blurRadius: 5, tint: Color(r: 1, g: 1, b: 1, a: 0.15))
    public static let regularMaterial = Material(blurRadius: 7, tint: Color(r: 1, g: 1, b: 1, a: 0.25))
    public static let thickMaterial = Material(blurRadius: 9, tint: Color(r: 1, g: 1, b: 1, a: 0.4))
    public static let ultraThickMaterial = Material(blurRadius: 11, tint: Color(r: 1, g: 1, b: 1, a: 0.55))
}

extension Material: PrimitiveView {
    public typealias Body = Swift.Never
}

extension Material {
    public func toSFNode() -> SFNode {
        var node = SFNode.makeDefault()
        node.kind = SF_NODE_RECT
        node.fill = tint.toSFColor()
        node.blurRadius = blurRadius * DeviceScale.current
        node.progressiveBlur = isProgressive ? 1 : 0
        node.progressiveStart = progressiveStart
        node.sizing = SF_SIZING_FILL
        return node
    }
}

public struct MaterialFilledShape<S: View>: View {
    public typealias Body = Never
    public var body: Never { fatalError() }
    let shape: S
    let material: Material
}

extension MaterialFilledShape {
    public func toSFNode() -> SFNode {
        var node = SFNode.makeDefault()
        node.kind = SF_NODE_RECT
        node.fill = material.tint.toSFColor()
        node.blurRadius = material.blurRadius * DeviceScale.current
        node.progressiveBlur = material.isProgressive ? 1 : 0
        node.progressiveStart = material.progressiveStart
        node.sizing = SF_SIZING_FILL
        node.cornerRadius = shapeCornerRadius(shape) ?? 0
        return node
    }
}

extension RoundedRectangle {
    public func fill(_ material: Material) -> MaterialFilledShape<RoundedRectangle> {
        MaterialFilledShape(shape: self, material: material)
    }
}

extension Circle {
    public func fill(_ material: Material) -> MaterialFilledShape<Circle> {
        MaterialFilledShape(shape: self, material: material)
    }
}

extension Capsule {
    public func fill(_ material: Material) -> MaterialFilledShape<Capsule> {
        MaterialFilledShape(shape: self, material: material)
    }
}

extension View {

    /// Backs this view with a material whose blur ramps out toward its bottom edge.
    public func progressiveBlur(
        _ material: Material = .regularMaterial,
        start: Float = 0.5
    ) -> BackgroundShapeModifier<Self, Material> {
        background(material.progressive(start: start))
    }
}

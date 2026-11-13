import CSwiftFlow

/// Which edge a progressive material's blur fades out toward.
public enum ProgressiveEdge: Sendable {

    /// Strongest at the top, gone by the bottom. What sits under a navigation
    /// bar.
    case bottom

    /// Strongest at the bottom, gone by the top. What sits under a tab bar or
    /// a bottom toolbar.
    case top
}

/// A backdrop blur that shows what is behind it, tinted.
public struct Material: Sendable {
    public let blurRadius: Float
    public let tint: Color

    public let isProgressive: Bool

    public let progressiveEdge: ProgressiveEdge

    public let progressiveStart: Float

    public init(
        blurRadius: Float,
        tint: Color,
        isProgressive: Bool = false,
        progressiveEdge: ProgressiveEdge = .bottom,
        progressiveStart: Float = 0.5
    ) {
        self.blurRadius = blurRadius
        self.tint = tint
        self.isProgressive = isProgressive
        self.progressiveEdge = progressiveEdge
        self.progressiveStart = progressiveStart
    }

    /// Ramps this material's blur out toward one edge, starting `start` of the
    /// way along the shape from the other one.
    public func progressive(
        toward edge: ProgressiveEdge = .bottom,
        start: Float = 0.5
    ) -> Material {
        Material(
            blurRadius: blurRadius,
            tint: tint,
            isProgressive: true,
            progressiveEdge: edge,
            progressiveStart: start
        )
    }

    /// The sign carries the direction: negative runs the ramp the other way
    /// up. A field of its own would mean a new slot in `SFNode`, which is a
    /// C struct three hosts and the renderer all agree on.
    var progressiveAmount: Float {
        guard isProgressive else { return 0 }
        return progressiveEdge == .top ? -1 : 1
    }

    public static let ultraThinMaterial = Material(blurRadius: 3, tint: Color(r: 1, g: 1, b: 1, a: 0.08))
    public static let thinMaterial = Material(blurRadius: 5, tint: Color(r: 1, g: 1, b: 1, a: 0.15))
    public static let regularMaterial = Material(blurRadius: 7, tint: Color(r: 1, g: 1, b: 1, a: 0.25))
    public static let thickMaterial = Material(blurRadius: 9, tint: Color(r: 1, g: 1, b: 1, a: 0.4))
    public static let ultraThickMaterial = Material(blurRadius: 11, tint: Color(r: 1, g: 1, b: 1, a: 0.55))
    public static let hardMaterial = Material(blurRadius: 6, tint: .background.opacity(0.55))
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
        node.progressiveBlur = progressiveAmount
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
    var tailOffset: Float = 0
    var tailLength: Float = 0
    var edgeColor: Color = .clear
    var edgeWidth: Float = 0

    init(shape: S, material: Material) {
        self.shape = shape
        self.material = material
    }

    /// Gives the shape a tail out of its top edge, as a popover points at
    /// what opened it: `offset` points along the edge from its middle
    /// (right is positive), reaching `length` points above it. The tail is
    /// part of the material, joined to the edge by a smooth curve, and is
    /// drawn outside the shape's frame without taking layout space.
    public func tail(at offset: Float, length: Float) -> Self {
        var copy = self
        copy.tailOffset = offset
        copy.tailLength = max(length, 0)
        return copy
    }

    /// A line just inside the outline, tail included.
    public func edge(_ color: Color, width: Float = 1) -> Self {
        var copy = self
        copy.edgeColor = color
        copy.edgeWidth = width
        return copy
    }
}

extension MaterialFilledShape {
    public func toSFNode() -> SFNode {
        var node = SFNode.makeDefault()
        node.kind = SF_NODE_RECT
        node.fill = material.tint.toSFColor()
        node.blurRadius = material.blurRadius * DeviceScale.current
        node.progressiveBlur = material.progressiveAmount
        node.progressiveStart = material.progressiveStart
        node.sizing = SF_SIZING_FILL
        node.cornerRadius = shapeCornerRadius(shape) ?? 0
        let scale = DeviceScale.current
        node.tailOffset = tailOffset * scale
        node.tailLength = tailLength * scale
        node.border = SFBorder(color: edgeColor.toSFColor(), width: edgeWidth * scale, _pad: (0, 0, 0))
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

    /// Backs this view with a material whose blur ramps out toward one edge.
    public func progressiveBlur(
        _ material: Material = .regularMaterial,
        toward edge: ProgressiveEdge = .bottom,
        start: Float = 0.5
    ) -> BackgroundShapeModifier<Self, Material> {
        background(material.progressive(toward: edge, start: start))
    }
}

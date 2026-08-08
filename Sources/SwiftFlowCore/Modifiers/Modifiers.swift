import CSwiftFlow

/// The inset distances for the sides of a rectangle.
public struct EdgeInsets: Sendable {
    public let top: Float
    public let bottom: Float
    public let leading: Float
    public let trailing: Float

    public static let zero = EdgeInsets(top: 0, bottom: 0, leading: 0, trailing: 0)

    public init(top: Float, bottom: Float, leading: Float, trailing: Float) {
        self.top = top
        self.bottom = bottom
        self.leading = leading
        self.trailing = trailing
    }

    public init(_ all: Float) {
        self.init(top: all, bottom: all, leading: all, trailing: all)
    }
}

/// An edge of a rectangle.
public struct Edge: OptionSet, Sendable {
    public let rawValue: Int
    public init(rawValue: Int) { self.rawValue = rawValue }
    public static let top: Edge = Edge(rawValue: 1 << 0)
    public static let bottom: Edge = Edge(rawValue: 1 << 1)
    public static let leading: Edge = Edge(rawValue: 1 << 2)
    public static let trailing: Edge = Edge(rawValue: 1 << 3)
    public static let horizontal: Edge = [.leading, .trailing]
    public static let vertical: Edge = [.top, .bottom]
    public static let all: Edge = [.top, .bottom, .leading, .trailing]
}

public struct FrameModifier<Content: View>: View {
    public typealias Body = Never
    public var body: Never { fatalError() }
    let content: Content
    let width: Float?
    let height: Float?
    let minWidth: Float?
    let maxWidth: Float?
    let minHeight: Float?
    let maxHeight: Float?
    let alignment: Alignment?
}

extension FrameModifier {
    public func toSFNode() -> SFNode {
        let child = content.toSFNode()
        let scale = DeviceScale.current

        var node = SFNode.makeDefault()
        node.kind = SF_NODE_STACK
        node.axis = SF_AXIS_DEPTH

        node.alignment = SF_ALIGNMENT_CENTER
        node.verticalAlignment = SF_ALIGNMENT_CENTER

        // TODO: minWidth/minHeight are ignored, and maxWidth/maxHeight only do
        // anything when they're .infinity.
        if let w = width {
            node.fixedWidth = w * scale
            node.sizingX = SF_SIZING_FIXED
        } else if maxWidth == .infinity {
            node.sizingX = SF_SIZING_FILL
        } else {
            node.sizingX = child.sizingX.inherited
        }

        if let h = height {
            node.fixedHeight = h * scale
            node.sizingY = SF_SIZING_FIXED
        } else if maxHeight == .infinity {
            node.sizingY = SF_SIZING_FILL
        } else {
            node.sizingY = child.sizingY.inherited
        }

        if let alignment {
            node.alignment = alignment.horizontal.toSFAlignment()
            node.verticalAlignment = alignment.vertical.toSFAlignment()
        }

        var children = [child]
        let count = children.count
        FrameArena.shared.storeNodes(&children) { pointer in
            node.children = pointer
            node.childrenLen = count
        }
        return node
    }
}

extension View {
    /// Positions this view within an invisible frame with the specified size.
    ///
    /// Each axis is independent: naming a width says nothing about the height,
    /// which comes from the content instead. Pass `.infinity` to `maxWidth` or
    /// `maxHeight` to fill the space offered on that axis.
    ///
    /// - Note: Arguments must be written in declaration order — `height:`
    ///   before `maxWidth:`, and so on.
    public func frame(
        width: Float? = nil,
        height: Float? = nil,
        minWidth: Float? = nil,
        maxWidth: Float? = nil,
        minHeight: Float? = nil,
        maxHeight: Float? = nil,
        alignment: Alignment? = nil
    ) -> FrameModifier<Self> {
        FrameModifier(
            content: self,
            width: width,
            height: height,
            minWidth: minWidth,
            maxWidth: maxWidth,
            minHeight: minHeight,
            maxHeight: maxHeight,
            alignment: alignment
        )
    }
}

public struct PaddingModifier<Content: View>: View {
    public typealias Body = Never
    public var body: Never { fatalError() }
    let content: Content
    let insets: EdgeInsets
}

extension PaddingModifier {
    public func toSFNode() -> SFNode {
        var child = content.toSFNode()
        let scale = DeviceScale.current

        var node = SFNode.makeDefault()
        node.kind = SF_NODE_STACK
        node.axis = SF_AXIS_DEPTH

        node.alignment = SF_ALIGNMENT_CENTER
        node.verticalAlignment = SF_ALIGNMENT_CENTER
        node.padding = SFEdgeInsets(
            top: insets.top * scale,
            bottom: insets.bottom * scale,
            leading: insets.leading * scale,
            trailing: insets.trailing * scale
        )

        node.sizingX = child.sizingX.inherited
        node.sizingY = child.sizingY.inherited

        var children = [child]
        let count = children.count
        FrameArena.shared.storeNodes(&children) { pointer in
            node.children = pointer
            node.childrenLen = count
        }
        return node
    }
}

extension View {
    /// Adds the specified padding to this view.
    public func padding(_ insets: EdgeInsets) -> PaddingModifier<Self> {
        PaddingModifier(content: self, insets: insets)
    }
    /// Adds the same padding to every edge of this view.
    public func padding(_ all: Float) -> PaddingModifier<Self> {
        PaddingModifier(content: self, insets: EdgeInsets(all))
    }
    /// Adds padding to the specified edges of this view.
    public func padding(_ edges: Edge, _ value: Float = 20) -> PaddingModifier<Self> {
        let top = edges.contains(.top) ? value : 0
        let bottom = edges.contains(.bottom) ? value : 0
        let leading = edges.contains(.leading) ? value : 0
        let trailing = edges.contains(.trailing) ? value : 0
        return PaddingModifier(
            content: self,
            insets: EdgeInsets(top: top, bottom: bottom, leading: leading, trailing: trailing)
        )
    }
    /// Adds the default padding to every edge of this view.
    public func padding() -> PaddingModifier<Self> { padding(16) }
}

public struct CornerRadiusModifier<Content: View>: View {
    public typealias Body = Never
    public var body: Never { fatalError() }
    let content: Content
    let cornerRadius: Float
}

extension CornerRadiusModifier {
    public func toSFNode() -> SFNode {
        var node = content.toSFNode()
        node.cornerRadius = cornerRadius * DeviceScale.current
        return node
    }
}

extension View {
    /// Rounds this view's corners.
    public func cornerRadius(_ radius: Float) -> CornerRadiusModifier<Self> {
        CornerRadiusModifier(content: self, cornerRadius: radius)
    }
}

public struct ShadowModifier<Content: View>: View {
    public typealias Body = Never
    public var body: Never { fatalError() }
    let content: Content
    let radius: Float
    let opacity: Float
}

extension ShadowModifier {
    public func toSFNode() -> SFNode {
        var node = content.toSFNode()
        node.shadowRadius = radius * DeviceScale.current
        node.shadowOpacity = opacity
        return node
    }
}

extension View {

    /// Draws a soft drop shadow behind this view.
    public func shadow(radius: Float, opacity: Float = 0.25) -> ShadowModifier<Self> {
        ShadowModifier(content: self, radius: radius, opacity: opacity)
    }
}

public struct MergeModifier<Content: View>: View {
    public typealias Body = Never
    public var body: Never { fatalError() }
    let content: Content
    let allowed: Bool
}

extension MergeModifier {
    public func toSFNode() -> SFNode {
        var node = content.toSFNode()
        node.noMerge = allowed ? 0 : 1
        return node
    }
}

extension View {

    /// Whether this shape may blend into a neighbour it overlaps.
    ///
    /// Pass `false` for anything whose outline is load-bearing rather than
    /// decorative.
    public func mergeable(_ allowed: Bool) -> MergeModifier<Self> {
        MergeModifier(content: self, allowed: allowed)
    }
}

public struct ClipShapeModifier<Content: View, S: View>: View {
    public typealias Body = Never
    public var body: Never { fatalError() }
    let content: Content
    let shape: S
}

extension ClipShapeModifier {
    public func toSFNode() -> SFNode {
        var node = content.toSFNode()

        guard let radius = shapeCornerRadius(shape) else { return node }

        node.clipContent = 1
        applyClip(radius, to: &node)
        return node
    }
}

// TODO: this rounds children that cover the whole parent, it doesn't clip.
// A child that overflows still draws. Needs a clip stack in the wgpu
// backend.
private func applyClip(_ radius: Float, to node: inout SFNode) {
    node.cornerRadius = radius

    guard node.kind == SF_NODE_STACK, node.axis == SF_AXIS_DEPTH else { return }

    guard let children = node.children else { return }
    let count = node.childrenLen

    for i in 0..<count {

        guard children[i].sizingX == SF_SIZING_FILL,
              children[i].sizingY == SF_SIZING_FILL,
              children[i].cornerRadius == 0
        else { continue }
        applyClip(radius, to: &children[i])
    }
}

extension View {

    /// Masks this view to the given shape.
    ///
    /// Exact for a child that covers the whole of this view. A child that
    /// overflows its parent still draws — see `.mergeable(_:)` and
    /// ARCHITECTURE.md.
    public func clipShape<S: View>(_ shape: S) -> ClipShapeModifier<Self, S> {
        ClipShapeModifier(content: self, shape: shape)
    }
}

public struct OpacityModifier<Content: View>: View {
    public typealias Body = Never
    public var body: Never { fatalError() }
    let content: Content
    let opacity: Float
}

extension OpacityModifier {
    public func toSFNode() -> SFNode {
        var node = content.toSFNode()
        node.color.a *= opacity
        node.fill.a *= opacity
        return node
    }
}

extension View {
    /// Sets the transparency of this view.
    public func opacity(_ value: Float) -> OpacityModifier<Self> {
        OpacityModifier(content: self, opacity: value)
    }
}

public struct OffsetModifier<Content: View>: View {
    public typealias Body = Never
    public var body: Never { fatalError() }
    let content: Content
    let x: Float
    let y: Float
}

extension OffsetModifier {
    public func toSFNode() -> SFNode {
        var node = content.toSFNode()
        let scale = DeviceScale.current

        node.offsetX += x * scale
        node.offsetY += y * scale
        return node
    }
}

extension View {

    /// Moves this view without affecting the layout around it.
    public func offset(x: Float = 0, y: Float = 0) -> OffsetModifier<Self> {
        OffsetModifier(content: self, x: x, y: y)
    }
}

public struct BlurModifier<Content: View>: View {
    public typealias Body = Never
    public var body: Never { fatalError() }
    let content: Content
    let radius: Float
}

extension BlurModifier {
    public func toSFNode() -> SFNode {
        var node = content.toSFNode()
        node.contentBlur = radius * DeviceScale.current
        return node
    }
}

extension View {

    /// Blurs this view's own content.
    public func blur(radius: Float) -> BlurModifier<Self> {
        BlurModifier(content: self, radius: radius)
    }
}

public struct BackgroundShapeModifier<Content: View, Background: View>: View {
    public typealias Body = Never
    public var body: Never { fatalError() }
    let content: Content
    let background: Background
}

extension BackgroundShapeModifier {
    public func toSFNode() -> SFNode {

        let bg = background.toSFNode()
        let fg = content.toSFNode()

        var children = [bg, fg]
        var node = SFNode.makeDefault()
        node.kind = SF_NODE_STACK
        node.axis = SF_AXIS_DEPTH

        node.sizingX = fg.sizingX
        node.sizingY = fg.sizingY

        node.fixedWidth = fg.fixedWidth
        node.fixedHeight = fg.fixedHeight

        let count = children.count
        FrameArena.shared.storeNodes(&children) { pointer in
            node.children = pointer
            node.childrenLen = count
        }
        return node
    }
}

extension View {
    /// Draws a view behind this one, sized to match.
    public func background<V: View>(_ background: V) -> BackgroundShapeModifier<Self, V> {
        BackgroundShapeModifier(content: self, background: background)
    }
}

public struct ScaleModifier<Content: View>: View {
    public typealias Body = Never
    public var body: Never { fatalError() }
    let content: Content
    let scale: Float
}

extension ScaleModifier {
    public func toSFNode() -> SFNode {
        var node = content.toSFNode()
        node.scale.x *= scale
        node.scale.y *= scale
        return node
    }
}

extension View {
    /// Scales this view about its centre, without affecting layout.
    public func scale(_ value: Float) -> ScaleModifier<Self> {
        ScaleModifier(content: self, scale: value)
    }
}

public struct SpecularModifier<Content: View>: View {
    public typealias Body = Never
    public var body: Never { fatalError() }
    let content: Content
}

extension SpecularModifier {
    public func toSFNode() -> SFNode {
        var node = content.toSFNode()
        node.specular = 1
        return node
    }
}

extension View {
    /// Adds the Liquid Glass rim highlight to this view's edge.
    public func specular() -> SpecularModifier<Self> {
        SpecularModifier(content: self)
    }
}

public struct WeightModifier<Content: View>: View {
    public typealias Body = Never
    public var body: Never { fatalError() }
    let content: Content
    let weight: Float
}

extension WeightModifier {
    public func toSFNode() -> SFNode {
        var node = content.toSFNode()
        node.weight = max(0, weight)
        return node
    }
}

extension View {

    /// Takes a share of the leftover space along the enclosing stack's main axis.
    ///
    /// Siblings that aren't flexible are measured first, so weights divide only
    /// what is genuinely left over. `Spacer()` is weight 1 in the same pool.
    ///
    /// - Note: Ignored inside a hugging container, whose size *is* the sum of
    ///   its children. Use `.expands()` on that container first.
    public func weight(_ weight: Float = 1) -> WeightModifier<Self> {
        WeightModifier(content: self, weight: weight)
    }
}

public struct ExpandModifier<Content: View>: View {
    public typealias Body = Never
    public var body: Never { fatalError() }
    let content: Content
}

extension ExpandModifier {
    public func toSFNode() -> SFNode {

        var node = content.toSFNode()
        node.sizing = SF_SIZING_FILL
        return node
    }
}

extension View {

    /// Makes a container take the space its parent offers instead of hugging
    /// its content.
    ///
    /// Unlike `.frame(maxWidth:maxHeight:)`, which wraps this view in a filling
    /// box and leaves it hugging, this sets the fill on the view itself — so
    /// `.weight(_:)` on its children has leftover space to divide.
    public func expands() -> ExpandModifier<Self> {
        ExpandModifier(content: self)
    }
}

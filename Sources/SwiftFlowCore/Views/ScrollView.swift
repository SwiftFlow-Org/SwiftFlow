import CSwiftFlow

/// A layout direction.
public enum Axis: Sendable {
    case vertical
    case horizontal
}

extension Axis {
    /// One or both layout axes.
    public struct Set: OptionSet, Sendable {
        public let rawValue: Int
        public init(rawValue: Int) { self.rawValue = rawValue }

        public static let horizontal = Set(rawValue: 1 << 0)
        public static let vertical = Set(rawValue: 1 << 1)
        public static let all: Set = [.horizontal, .vertical]
    }
}

/// A view that scrolls its content along one axis.
///
/// A horizontal scroll view has to be told its height — its content decides
/// how far it runs sideways, not how tall it is.
public struct ScrollView<Content: View>: View {
    public typealias Body = Never
    public var body: Never { fatalError() }
    let axis: Axis

    let content: (ScrollGeometry) -> Content
    let scrollID: UInt32

    let defaultAnchor: ScrollAnchor?

    private init(
        axis: Axis,
        scrollID: UInt32,
        defaultAnchor: ScrollAnchor? = nil,
        content: @escaping (ScrollGeometry) -> Content
    ) {
        self.axis = axis
        self.scrollID = scrollID
        self.defaultAnchor = defaultAnchor
        self.content = content
    }

    private static func identity(
        _ name: String?, _ fileID: String, _ line: Int, _ column: Int
    ) -> UInt32 {
        name.map(ScrollGeometry.id(forName:)) ?? fnv1a("\(fileID):\(line):\(column)")
    }

    public init(
        _ axis: Axis = .vertical,
        name: String? = nil,
        fileID: String = #fileID, line: Int = #line, column: Int = #column,
        @ViewBuilder content: @escaping () -> Content
    ) {
        self.init(
            axis: axis,
            scrollID: Self.identity(name, fileID, line, column),
            content: { _ in content() }
        )
    }

    public init(
        _ axis: Axis = .vertical,
        name: String? = nil,
        fileID: String = #fileID, line: Int = #line, column: Int = #column,
        @ViewBuilder content: @escaping (ScrollGeometry) -> Content
    ) {
        self.init(
            axis: axis,
            scrollID: Self.identity(name, fileID, line, column),
            content: content
        )
    }
}

extension ScrollView {
    public func toSFNode() -> SFNode {
        let state = NodeRegistry.shared.scrollState(for: scrollID)

        state.axis = axis
        state.defaultAnchor = defaultAnchor?.fraction

        var node = SFNode.makeDefault()
        node.kind = SF_NODE_SCROLL
        node.axis = axis == .vertical ? SF_AXIS_VERTICAL : SF_AXIS_HORIZONTAL
        node.sizing = SF_SIZING_FILL
        node.scrollId = scrollID
        switch axis {
        case .vertical: node.contentOffsetY = state.offset
        case .horizontal: node.contentOffsetX = state.offset
        }

        NavigationConfigStore.shared.registerScroll(scrollID, axis: axis)

        TabBuildContext.shared.register(scrollID, axis: axis)

        var children = buildChildren(content(ScrollGeometry(state)))

        let count = children.count
        FrameArena.shared.storeNodes(&children) { pointer in
            node.children = pointer
            node.childrenLen = count
        }
        return node
    }
}

/// Where a scroll view rests before anything has scrolled it.
///
/// Named along the axis rather than by compass point, since the same anchor
/// has to mean something on a horizontal scroll too.
public enum ScrollAnchor: Sendable {

    /// The top of a vertical scroll, the leading edge of a horizontal one.
    /// The default, and where a scroll sits with no anchor at all.
    case start

    case center

    /// The bottom of a vertical scroll, the trailing edge of a horizontal one.
    /// What a transcript wants: opens on the newest thing rather than the
    /// oldest.
    case end

    var fraction: Float {
        switch self {
        case .start: return 0
        case .center: return 0.5
        case .end: return 1
        }
    }
}

extension ScrollView {

    /// Opens this scroll view at `anchor` instead of at the top.
    ///
    /// It applies on the first frame that has travel to apply it to, so a list
    /// that only overflows once it has loaded still lands where it was asked
    /// to. After that the reader owns the position — the anchor never pulls
    /// content back once it has been scrolled.
    ///
    /// - Note: unlike SwiftUI's, this takes a `ScrollAnchor` rather than a
    ///   `UnitPoint`, and anchors the one axis the scroll actually runs on.
    public func defaultScrollAnchor(_ anchor: ScrollAnchor) -> ScrollView<Content> {
        ScrollView(
            axis: axis,
            scrollID: scrollID,
            defaultAnchor: anchor,
            content: content
        )
    }
}

/// Which edge of a scroll view an effect sits at.
public enum ScrollEdge: Sendable {
    case top
    case bottom
}

extension ScrollEdge {

    /// The bar that edge normally carries, which is what an unstated band
    /// height falls back to.
    var defaultEffectHeight: Float {
        switch self {
        case .top: return NavigationBarMetrics.collapsedHeight
        case .bottom: return TabBarMetrics.contentInset
        }
    }
}

/// What the two stock bands are cut from.
///
/// Every number the built-in styles look like is here, and nothing else reads
/// them — so retuning `.soft` or `.hard` app-wide is editing this enum, not
/// hunting through the builders. Reach for the `band:` closure only for a
/// shape these can't describe at all.
public enum ScrollEdgeEffectMetrics {

    // The soft band: what the navigation and tab bars lay down.

    nonisolated(unsafe) public static var softTint: Color = .background
    nonisolated(unsafe) public static var softBlurRadius: Float = 50

    /// Where the ramp begins, measured from the quiet end. 0 ramps across the
    /// whole band, which is what the bars do.
    nonisolated(unsafe) public static var softStart: Float = 0

    // The hard band: a plate with the blur tucked under its outer edge.

    nonisolated(unsafe) public static var hardTint: Color = .background
    nonisolated(unsafe) public static var hardBlurRadius: Float = 50

    /// Late on purpose. The blur is meant to disappear under the plate rather
    /// than read as a gradient of its own, so it only starts ramping near the
    /// inner edge where the plate stops covering it.
    nonisolated(unsafe) public static var hardBlurStart: Float = 0.75

    /// The plate laid over that blur.
    nonisolated(unsafe) public static var hardFillOpacity: Float = 0.55

    /// The line around the plate.
    nonisolated(unsafe) public static var hardStrokeOpacity: Float = 0.1
    nonisolated(unsafe) public static var hardStrokeWidth: Float = 0.67
}

/// How content behaves as it reaches the edge of a scroll view.
///
/// Two stock bands. Neither is a special case in the renderer — both are
/// ordinary views pinned to an edge — so anything they can't express is
/// reachable through the `band:` form of `.scrollEdgeEffect(for:height:)`,
/// and anything that is only a different number is in
/// `ScrollEdgeEffectMetrics`.
public enum ScrollEdgeEffectStyle: Sendable {

    /// The blur ramps out across the band, so content dissolves on its way
    /// under the bar rather than meeting a line partway.
    case soft

    /// A plate — flat fill, hairline around it — with a late-starting blur
    /// behind. For chrome meant to read as a surface of its own.
    case hard
}

/// The band a stock `.scrollEdgeEffect(_:for:)` draws.
public struct ScrollEdgeEffectBand: View {
    let style: ScrollEdgeEffectStyle
    let edge: ScrollEdge

    public init(style: ScrollEdgeEffectStyle, edge: ScrollEdge) {
        self.style = style
        self.edge = edge
    }

    /// Strongest against the edge it sits on, gone by the far side of the
    /// band — so the ramp runs toward the opposite edge from the one the band
    /// is pinned to.
    private var toward: ProgressiveEdge { edge == .top ? .bottom : .top }

    public var body: some View {
        switch style {
        case .soft:
            Material(
                blurRadius: ScrollEdgeEffectMetrics.softBlurRadius,
                tint: ScrollEdgeEffectMetrics.softTint
            )
            .progressive(toward: toward, start: ScrollEdgeEffectMetrics.softStart)

        case .hard:
            // Untinted, because the plate in front carries the colour. Tinting
            // both would lay the fill down twice.
            Material(
                blurRadius: ScrollEdgeEffectMetrics.hardBlurRadius,
                tint: .clear
            )
            .progressive(toward: toward, start: ScrollEdgeEffectMetrics.hardBlurStart)
            .overlay(plate)
        }
    }

    /// A square-cornered plate: the fill, and the line around it in one node —
    /// `.stroke()` on a rect writes its border rather than adding a layer.
    private var plate: some View {
        RoundedRectangle(cornerRadius: 0)
            .fill(ScrollEdgeEffectMetrics.hardTint
                .opacity(ScrollEdgeEffectMetrics.hardFillOpacity))
            .stroke(
                ScrollEdgeEffectMetrics.hardTint
                    .opacity(ScrollEdgeEffectMetrics.hardStrokeOpacity),
                lineWidth: ScrollEdgeEffectMetrics.hardStrokeWidth
            )
    }
}

extension View {

    /// Blurs content as it runs off the edge of a scroll view.
    ///
    /// The band is drawn over the content, so it reads whatever is underneath
    /// it — this is the same effect the navigation and tab bars lay down, made
    /// available on its own for a scroll that sits under neither.
    ///
    /// `height` defaults to the bar on that edge: the collapsed navigation bar
    /// at the top, the tab bar and its home indicator at the bottom.
    /// What either style looks like is `ScrollEdgeEffectMetrics` — change it
    /// there and every band in the app follows.
    public func scrollEdgeEffect(
        _ style: ScrollEdgeEffectStyle = .soft,
        for edge: ScrollEdge = .top,
        height: Float? = nil
    ) -> OverlayModifier<Self, FrameModifier<ScrollEdgeEffectBand>> {
        scrollEdgeEffect(for: edge, height: height) {
            ScrollEdgeEffectBand(style: style, edge: edge)
        }
    }

    /// The same band, built out of whatever you like.
    ///
    /// The stock styles are only a material pinned to an edge, and this is the
    /// seam they are built on — so a band that layers a material under a
    /// gradient, or draws a hairline along its inner edge, is composition
    /// rather than a new case here:
    ///
    /// ```swift
    /// ScrollView { Feed() }
    ///     .scrollEdgeEffect(for: .bottom, height: 120) {
    ///         Material.thickMaterial
    ///             .overlay(RoundedRectangle(cornerRadius: 0).fill(.surface.opacity(0.3)))
    ///             .overlay(hairline, alignment: .top)
    ///     }
    /// ```
    ///
    /// The band is laid over the content at full width and the given height,
    /// pinned to `edge`. Everything else is yours.
    public func scrollEdgeEffect<Band: View>(
        for edge: ScrollEdge = .top,
        height: Float? = nil,
        @ViewBuilder band: () -> Band
    ) -> OverlayModifier<Self, FrameModifier<Band>> {
        overlay(
            band().frame(height: height ?? edge.defaultEffectHeight, maxWidth: .infinity),
            alignment: edge == .top ? .top : .bottom
        )
    }
}

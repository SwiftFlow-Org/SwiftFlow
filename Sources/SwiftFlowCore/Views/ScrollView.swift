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

/// How content behaves as it reaches the edge of a scroll view.
///
/// Two stock bands. Neither is a special case in the renderer — both are
/// ordinary views pinned to an edge — so anything they can't express is
/// reachable through the `band:` form of `.scrollEdgeEffect(for:height:)`.
public enum ScrollEdgeEffectStyle: Sendable {

    /// The blur ramps out across the band, so content dissolves on its way
    /// under the bar rather than meeting a line partway.
    case soft

    /// The band blurs evenly and stops where it stops — a defined edge, for
    /// chrome that is meant to read as a surface of its own.
    case hard

    /// The material this style makes of `base` at `edge`.
    func material(_ base: Material, at edge: ScrollEdge) -> Material {
        switch self {
        case .hard:
            return base
        case .soft:
            // Strongest against the edge it sits on, gone by the far side of
            // the band — which is the opposite edge from the one it's pinned
            // to. `start: 0` ramps across the whole band, like the bars do.
            return base.progressive(toward: edge == .top ? .bottom : .top, start: 0)
        }
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
    public func scrollEdgeEffect(
        _ style: ScrollEdgeEffectStyle = .soft,
        for edge: ScrollEdge = .top,
        height: Float? = nil,
        material: Material = .hardMaterial
    ) -> OverlayModifier<Self, FrameModifier<Material>> {
        scrollEdgeEffect(for: edge, height: height) {
            style.material(material, at: edge)
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

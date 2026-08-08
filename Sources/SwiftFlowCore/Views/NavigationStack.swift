import CSwiftFlow

public enum NavigationBarTitleDisplayMode: Sendable {

    case automatic

    case large

    case inline
}

enum NavigationBarMetrics {

    static let barHeight: Float = 44

    static let largeTitleHeight: Float = 52

    static let horizontalPadding: Float = 20

    static let itemSpacing: Float = 12

    static var collapsedHeight: Float { SafeArea.top + barHeight }

    static func contentInset(mode: NavigationBarTitleDisplayMode, hasTitle: Bool) -> Float {
        collapsedHeight + largeBandHeight(mode: mode, hasTitle: hasTitle)
    }

    static func largeBandHeight(mode: NavigationBarTitleDisplayMode, hasTitle: Bool) -> Float {
        (mode == .large && hasTitle) ? largeTitleHeight : 0
    }

    static let collapseThreshold: Float = 44

    static let collapseHysteresis: Float = 8

    static let itemTransitionScale: Float = 0.15

    static let itemExitBlur: Float = 8

    static let inlineTitleRise: Float = 7

    static let inlineTitleBlur: Float = 6
}

final class NavigationConfigStore {
    nonisolated(unsafe) static let shared = NavigationConfigStore()

    nonisolated(unsafe) var title: String?
    nonisolated(unsafe) var displayMode: NavigationBarTitleDisplayMode = .automatic
    nonisolated(unsafe) var items: [ResolvedToolbarItem] = []
    nonisolated(unsafe) var scrollID: UInt32?

    nonisolated(unsafe) private var pendingTitle: String?
    nonisolated(unsafe) private var pendingDisplayMode: NavigationBarTitleDisplayMode = .automatic
    nonisolated(unsafe) private var pendingItems: [ResolvedToolbarItem] = []

    nonisolated(unsafe) private var depth: Int = 0

    var isCollecting: Bool { depth > 0 }

    func reset() {
        title = nil
        displayMode = .automatic
        items = []
        scrollID = nil
    }

    func resetAll() {
        reset()
        pendingTitle = nil
        pendingDisplayMode = .automatic
        pendingItems = []
        depth = 0
    }

    func setTitle(_ newTitle: String) {
        if isCollecting { title = newTitle } else { pendingTitle = newTitle }
    }

    func setDisplayMode(_ mode: NavigationBarTitleDisplayMode) {
        if isCollecting { displayMode = mode } else { pendingDisplayMode = mode }
    }

    func addItems(_ newItems: [ResolvedToolbarItem], first: Bool = false) {
        if isCollecting {
            items.insert(contentsOf: newItems, at: first ? 0 : items.count)
        } else {
            pendingItems.insert(contentsOf: newItems, at: first ? 0 : pendingItems.count)
        }
    }

    func registerScroll(_ id: UInt32, axis: Axis) {
        guard isCollecting, scrollID == nil, axis == .vertical else { return }
        scrollID = id
    }

    func beginCollecting() -> NavigationBarConfig {
        let saved = snapshot()
        title = pendingTitle
        displayMode = pendingDisplayMode
        items = pendingItems
        scrollID = nil
        pendingTitle = nil
        pendingDisplayMode = .automatic
        pendingItems = []
        depth += 1
        return saved
    }

    func endCollecting(restoring saved: NavigationBarConfig) -> NavigationBarConfig {
        let mine = snapshot()
        title = saved.title
        displayMode = saved.displayMode
        items = saved.items
        scrollID = saved.scrollID
        depth -= 1
        return mine
    }

    func snapshot() -> NavigationBarConfig {
        NavigationBarConfig(
            title: title, displayMode: displayMode, items: items, scrollID: scrollID
        )
    }
}

struct NavigationBarConfig {
    let title: String?
    let displayMode: NavigationBarTitleDisplayMode
    let items: [ResolvedToolbarItem]
    let scrollID: UInt32?

    func nodes(for placement: ToolbarItemPlacement, transitions: Bool? = nil) -> [SFNode] {
        items
            .filter { $0.placement == placement }
            .filter { transitions == nil || $0.transitions == transitions }
            .map { $0.node }
    }

    var resolvedDisplayMode: NavigationBarTitleDisplayMode {
        switch displayMode {
        case .large: return .large
        case .inline: return .inline
        case .automatic: return title == nil ? .inline : .large
        }
    }

    var isTitleCollapsed: Bool {
        let mode = resolvedDisplayMode
        let band = NavigationBarMetrics.largeBandHeight(mode: mode, hasTitle: title != nil)
        guard band > 0, let scrollID else { return false }

        let scale = DeviceScale.current
        let threshold = NavigationBarMetrics.collapseThreshold * scale
        let hysteresis = NavigationBarMetrics.collapseHysteresis * scale
        let offset = NodeRegistry.shared.scrollState(for: scrollID).offset

        let wasCollapsed = NodeRegistry.shared.navTitleCollapsed[scrollID] ?? false
        let collapsed = wasCollapsed ? offset > threshold - hysteresis : offset >= threshold
        NodeRegistry.shared.navTitleCollapsed[scrollID] = collapsed
        return collapsed
    }

    var largeTitleScroll: Float {
        let mode = resolvedDisplayMode
        let band = NavigationBarMetrics.largeBandHeight(mode: mode, hasTitle: title != nil)
        guard band > 0, let scrollID else { return 0 }
        let offset = NodeRegistry.shared.scrollState(for: scrollID).offset / DeviceScale.current
        return min(max(offset, 0), band)
    }
}

/// A view that shows a root screen and can push others over it.
///
/// Drive it with `Navigator`.
public struct NavigationStack<Content: View>: View {
    public typealias Body = Never
    public var body: Never { fatalError() }

    let content: Content
    let background: Material?

    let insetsContent: Bool

    let transitionProgress: Float?

    let barID: UInt32

    public init(
        background: Material? = Material(blurRadius: 50, tint: .background, isProgressive: true),
        progress: Float? = nil,
        @ViewBuilder content: () -> Content
    ) {
        self.init(
            background: background,
            insetsContent: true,
            transitionProgress: nil,
            barID: fnv1a("navbar"),
            content: content
        )
    }

    init(
        background: Material?,
        insetsContent: Bool,
        transitionProgress: Float?,
        barID: UInt32,
        @ViewBuilder content: () -> Content
    ) {
        self.background = background
        self.insetsContent = insetsContent
        self.transitionProgress = transitionProgress
        self.barID = barID
        self.content = content()
    }
}

extension NavigationStack {
    public func toSFNode() -> SFNode {
        let store = NavigationConfigStore.shared

        let saved = store.beginCollecting()
        let contentNodes = buildChildren(content)
        let config = store.endCollecting(restoring: saved)

        var contentNode: SFNode
        if contentNodes.count == 1 {
            contentNode = contentNodes[0]
        } else {

            contentNode = NodeListView(nodes: contentNodes).toSFNode()
        }

        let mode = config.resolvedDisplayMode

        if insetsContent {
            let inset = NavigationBarMetrics.contentInset(mode: mode, hasTitle: config.title != nil)
            applyContentInset(
                &contentNode,
                top: inset * DeviceScale.current,
                edge: background.map {
                    EdgeEffect(material: $0, height: barHeight(config, mode) * DeviceScale.current)
                }
            )
        }

        let bar = NavigationBar(
            config: config,

            background: nil,
            progress: transitionProgress,
            barID: barID
        )

        var children = [contentNode, NodeBuilder.buildAny(bar)]

        var node = SFNode.makeDefault()
        node.kind = SF_NODE_STACK
        node.axis = SF_AXIS_DEPTH
        node.spacing = 0
        node.sizing = SF_SIZING_FILL
        node.alignment = SF_ALIGNMENT_CENTER

        node.verticalAlignment = SF_ALIGNMENT_LEADING

        let count = children.count
        FrameArena.shared.storeNodes(&children) { pointer in
            node.children = pointer
            node.childrenLen = count
        }
        return node
    }
}

func barHeight(_ config: NavigationBarConfig, _ mode: NavigationBarTitleDisplayMode) -> Float {
    let band = NavigationBarMetrics.largeBandHeight(mode: mode, hasTitle: config.title != nil)
    guard band > 0, !config.isTitleCollapsed else {
        return NavigationBarMetrics.collapsedHeight
    }
    return NavigationBarMetrics.collapsedHeight + max(0, band - config.largeTitleScroll)
}

struct EdgeEffect {
    let material: Material

    let height: Float

    func apply(to node: inout SFNode) {
        guard height > 0 else { return }
        node.fill = material.tint.toSFColor()
        node.blurRadius = material.blurRadius
        node.progressiveBlur = material.isProgressive ? 1 : 0

        node.progressiveStart = 0
        node.edgeEffectHeight = height
    }
}

func applyContentInset(
    _ node: inout SFNode, top: Float = 0, bottom: Float = 0, edge: EdgeEffect? = nil
) {
    guard node.kind == SF_NODE_STACK,
          node.axis == SF_AXIS_DEPTH,

          node.sizingX == SF_SIZING_FILL,
          node.sizingY == SF_SIZING_FILL,
          let children = node.children,
          node.childrenLen >= 1
    else {
        node.padding.top += top
        node.padding.bottom += bottom
        edge?.apply(to: &node)
        return
    }

    let count = node.childrenLen
    for i in 0..<count {
        guard children[i].sizingX == SF_SIZING_FILL,
              children[i].sizingY == SF_SIZING_FILL
        else {

            node.padding.top += top
            node.padding.bottom += bottom
            edge?.apply(to: &node)
            return
        }
    }

    for i in 0..<count where !isBackdrop(children[i]) {
        applyContentInset(&children[i], top: top, bottom: bottom, edge: edge)
    }
}

private func isBackdrop(_ node: SFNode) -> Bool {
    node.kind == SF_NODE_RECT && node.childrenLen == 0
}

struct NavigationBar: View {
    let config: NavigationBarConfig

    let background: Material?

    let progress: Float?

    let barID: UInt32

    var body: some View {
        if let background {
            barRow.progressiveBlur(background, start: 0)
        } else {
            barRow
        }
    }

    private var itemScale: Float {
        1 - (progress ?? 0) * NavigationBarMetrics.itemTransitionScale
    }

    private var isLeaving: Bool { (progress ?? 0) >= 1 }

    private var barRow: some View {
        ZStack(alignment: .center) {
            NodeListView(nodes: config.nodes(for: .principal))
            HStack(spacing: NavigationBarMetrics.itemSpacing) {

                NodeListView(nodes: config.nodes(for: .topBarLeading, transitions: false))
                leaving(config.nodes(for: .topBarLeading, transitions: true), barID &+ 1)
                Spacer()
                leaving(config.nodes(for: .topBarTrailing, transitions: true), barID &+ 2)
                NodeListView(nodes: config.nodes(for: .topBarTrailing, transitions: false))
            }
            .padding(.horizontal, NavigationBarMetrics.horizontalPadding)
            .frame(height: NavigationBarMetrics.barHeight, maxWidth: .infinity)
        }

        .padding(.top, SafeArea.top)
        .frame(height: NavigationBarMetrics.collapsedHeight, maxWidth: .infinity)
    }

    private typealias LeavingGroup =
        ScaleModifier<AnimationModifier<BlurModifier<OpacityModifier<NodeListView>>>>

    private func leaving(_ nodes: [SFNode], _ id: UInt32) -> LeavingGroup? {
        guard !nodes.isEmpty else { return nil }
        return NodeListView(nodes: nodes)
            .opacity(isLeaving ? 0 : 1)
            .blur(radius: isLeaving ? NavigationBarMetrics.itemExitBlur : 0)
            .animation(.spring(response: 0.28, dampingFraction: 0.9), id: id)
            .scale(itemScale)
    }
}

struct NavigationTitles: View {
    let config: NavigationBarConfig
    let mode: NavigationBarTitleDisplayMode
    let isCollapsed: Bool
    let scrolled: Float

    let animationID: UInt32

    private typealias InlineTitle =
        AnimationModifier<BlurModifier<OffsetModifier<OpacityModifier<Text>>>>

    private var largeBand: Float {
        NavigationBarMetrics.largeBandHeight(mode: mode, hasTitle: config.title != nil)
    }

    private var totalHeight: Float {
        NavigationBarMetrics.collapsedHeight + (isCollapsed ? 0 : max(0, largeBand - scrolled))
    }

    var body: some View {
        VStack(spacing: 0) {

            ZStack(alignment: .center) {
                inlineTitle
            }
            .padding(.top, SafeArea.top)
            .frame(height: NavigationBarMetrics.collapsedHeight, maxWidth: .infinity)

            largeTitle
        }
        .frame(height: totalHeight, maxWidth: .infinity)
    }

    private var largeTitle: OffsetModifier<LargeTitleBand>? {
        guard largeBand > 0, !isCollapsed, let title = config.title else { return nil }
        return LargeTitleBand(title: title, height: largeBand)
            .offset(y: -scrolled)
    }

    private var inlineTitle: InlineTitle? {
        guard let title = config.title, config.nodes(for: .principal).isEmpty else { return nil }
        let shown = largeBand > 0 ? isCollapsed : true
        return Text(title)
            .font(.headline)
            .opacity(shown ? 1 : 0)
            .offset(y: shown ? 0 : NavigationBarMetrics.inlineTitleRise)
            .blur(radius: shown ? 0 : NavigationBarMetrics.inlineTitleBlur)
            .animation(.spring(response: 0.3, dampingFraction: 0.85), id: animationID)
    }
}

struct LargeTitleBand: View {
    let title: String
    let height: Float

    var body: some View {
        HStack(spacing: 0) {
            Text(title)
                .font(.largeTitle)
            Spacer()
        }
        .padding(.horizontal, NavigationBarMetrics.horizontalPadding)
        .frame(height: height, maxWidth: .infinity)
    }
}

public struct NavigationTitleModifier<Content: View>: View {
    public typealias Body = Never
    public var body: Never { fatalError() }
    let content: Content
    let title: String
}

extension NavigationTitleModifier {
    public func toSFNode() -> SFNode {

        NavigationConfigStore.shared.setTitle(title)
        return content.toSFNode()
    }
}

public struct NavigationBarTitleDisplayModeModifier<Content: View>: View {
    public typealias Body = Never
    public var body: Never { fatalError() }
    let content: Content
    let mode: NavigationBarTitleDisplayMode
}

extension NavigationBarTitleDisplayModeModifier {
    public func toSFNode() -> SFNode {

        NavigationConfigStore.shared.setDisplayMode(mode)
        return content.toSFNode()
    }
}

extension View {

    /// Sets the title in the navigation bar for this view.
    public func navigationTitle(_ title: String) -> NavigationTitleModifier<Self> {
        NavigationTitleModifier(content: self, title: title)
    }

    /// Whether the navigation title shows large or inline.
    public func navigationBarTitleDisplayMode(
        _ mode: NavigationBarTitleDisplayMode
    ) -> NavigationBarTitleDisplayModeModifier<Self> {
        NavigationBarTitleDisplayModeModifier(content: self, mode: mode)
    }
}

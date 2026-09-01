import CSwiftFlow

public enum TabRole: Sendable {

    case leading
    case trailing
}

public enum TabReselect {

    case popToRoot

    case scrollToTop

    case run(() -> Void)
}

public protocol TabIcon {
    func resolveIcon(size: Float, color: Color) -> AnyView
}

extension Image: TabIcon {
    public func resolveIcon(size: Float, color: Color) -> AnyView {
        AnyView(
            resizable()
                .aspectRatio(contentMode: .fit)
                .foregroundColor(color)
                .frame(width: size, height: size)
        )
    }
}

extension Icon: TabIcon {

    public func resolveIcon(size defaultSize: Float, color defaultColor: Color) -> AnyView {

        AnyView(
            size(explicitSize ?? defaultSize)
                .foregroundColor(explicitTint ?? defaultColor)
        )
    }
}

/// One tab in a `TabView`.
public struct Tab<Content: View>: TabContent {
    let title: String
    let icon: (any TabIcon)?
    let role: TabRole?
    let reselect: TabReselect?
    let badgeCount: Int?
    let content: () -> Content

    public init(
        _ title: String,
        _ icon: (any TabIcon)? = nil,
        role: TabRole? = nil,
        onReselect: TabReselect? = nil,
        @ViewBuilder content: @escaping () -> Content
    ) {
        self.title = title
        self.icon = icon
        self.role = role
        self.reselect = onReselect
        self.badgeCount = nil
        self.content = content
    }

    public init(
        _ title: String,
        _ icon: Icon,
        role: TabRole? = nil,
        onReselect: TabReselect? = nil,
        @ViewBuilder content: @escaping () -> Content
    ) {
        self.init(
            title, icon as any TabIcon,
            role: role, onReselect: onReselect, content: content
        )
    }

    private init(
        title: String, icon: (any TabIcon)?, role: TabRole?,
        reselect: TabReselect?, badgeCount: Int?, content: @escaping () -> Content
    ) {
        self.title = title
        self.icon = icon
        self.role = role
        self.reselect = reselect
        self.badgeCount = badgeCount
        self.content = content
    }

    public func badge(_ count: Int) -> Tab {
        Tab(
            title: title, icon: icon, role: role,
            reselect: reselect, badgeCount: count > 0 ? count : nil, content: content
        )
    }

    public func resolveTabs() -> [ResolvedTab] {

        [
            ResolvedTab(
                title: title,
                icon: icon,
                role: role,
                reselect: reselect,
                badge: badgeCount,
                build: { self.content() }
            )
        ]
    }
}

public struct ResolvedTab {
    let title: String
    let icon: (any TabIcon)?

    var index: Int = 0
    let role: TabRole?
    let reselect: TabReselect?
    let badge: Int?

    let build: () -> any View
}

public protocol TabContent {
    func resolveTabs() -> [ResolvedTab]
}

@resultBuilder
public struct TabBuilder {
    public static func buildBlock(_ parts: any TabContent...) -> [ResolvedTab] {
        parts.flatMap { $0.resolveTabs() }
    }

    public static func buildOptional(_ part: [ResolvedTab]?) -> [ResolvedTab] {
        part ?? []
    }

    public static func buildEither(first: [ResolvedTab]) -> [ResolvedTab] { first }

    public static func buildEither(second: [ResolvedTab]) -> [ResolvedTab] { second }
}

enum TabBarMetrics {

    static let barHeight: Float = 64
    static let iconSize: Float = 24
    static let horizontalPadding: Float = 20

    /// The gap between the group and a `role: .trailing` tab, and the ring of
    /// glass the group keeps around its items — the room the pill moves in.
    static let groupSpacing: Float = 12
    static let groupInset: Float = 4
    static let trailingDiameter: Float = 62

    static var contentInset: Float { barHeight + SafeArea.bottom }

    /// The pill's travel spring: quick, damped just short of a visible bounce.
    static let pillResponse: Float = 0.35
    static let pillDamping: Float = 0.85

    /// The same spring, stiffened, for a pill being dragged.
    static let pillDragResponse: Float = 0.18

    /// How far a drag has to run before it is a drag and not a tap.
    static let dragSlop: Float = 6

    /// How much of a flick's speed is read as intent to travel further.
    static let flickProjection: Float = 0.09

    /// How far the pill runs past its own width, as a fraction of it, per
    /// item-per-second of travel — calibrated against the 7.3 items/s that
    /// spring peaks at on a hop to the next tab, for about a sixth over.
    static let pillStretchPerVelocity: Float = 0.024
    static let pillMaxStretch: Float = 0.3

    /// How much of that stretch runs ahead of the pill rather than behind it.
    static let pillStretchBias: Float = 0.8

    /// The height the stretch costs. Glass keeps its volume.
    static let pillSquash: Float = 0.16

    /// The kick the newly selected item takes, and the spring that absorbs it.
    static let popImpulse: Float = 3.2
    static let popResponse: Float = 0.3
    static let popDamping: Float = 0.55

    /// How long the pill takes to fade when the selection leaves the group.
    static let presenceDuration: Float = 0.18

    /// The spring the bar morphs on between tabs and search field. Barely any
    /// bounce: this one moves boxes, not a highlight.
    static let expansionResponse: Float = 0.45
    static let expansionDamping: Float = 0.95

    /// How much faster than the collapse the covered items fade. Nothing here
    /// clips, so they have to be gone before the group closes past them.
    static let collapseFadeRate: Float = 5

    /// The gap between the search icon and its placeholder, and the margin
    /// the pair keeps inside the expanded field.
    static let searchSpacing: Float = 8
    static let searchInset: Float = 20
    static let searchPlaceholderSize: Float = 15

    /// How long an item takes to pick up (or drop) the accent tint.
    static let tintDuration: Double = 0.2

    /// Identities that outlive a build, keyed by the tab's position: the item
    /// box the pill measures itself against, and the two things that tint.
    static func itemID(_ index: Int) -> UInt32 { fnv1a("swiftflow.tabbar.item.\(index)") }
    static func iconID(_ index: Int) -> UInt32 { fnv1a("swiftflow.tabbar.icon.\(index)") }
    static func labelID(_ index: Int) -> UInt32 { fnv1a("swiftflow.tabbar.label.\(index)") }

    /// The group at its natural width, which is what the collapse starts from.
    static let groupID = fnv1a("swiftflow.tabbar.group")
}

/// A view that switches between tabs.
///
/// Each tab gets its own `NavigationStack` — don't add one.
public struct TabView: View {
    public typealias Body = Never
    public var body: Never { fatalError() }

    let tabs: [ResolvedTab]
    let background: Material?
    let navigationBackground: Material?
    let transition: NavTransition
    let screenBackground: Color
    let interactiveBack: Bool

    public init(
        background: Material? = Material.ultraThinMaterial, //Material(blurRadius: 50, tint: .background, isProgressive: false),
        navigationBackground: Material? = Material.ultraThinMaterial, //Material(blurRadius: 50, tint: .background, isProgressive: true),
        transition: NavTransition = .standard,
        screenBackground: Color = .background,
        interactiveBack: Bool = true,
        @TabBuilder content: () -> [ResolvedTab]
    ) {
        var resolved = content()

        for index in resolved.indices { resolved[index].index = index }
        self.tabs = resolved
        self.background = background
        self.navigationBackground = navigationBackground
        self.transition = transition
        self.screenBackground = screenBackground
        self.interactiveBack = interactiveBack
    }
}

extension TabView {
    public func toSFNode() -> SFNode {

        Navigator.tabTitles = tabs.map(\.title)

        let selected = tabs.indices.contains(Navigator.tabSelection) ? Navigator.tabSelection : 0

        var content: SFNode
        if tabs.indices.contains(selected) {
            let tab = tabs[selected]
            TabBuildContext.shared.begin(tab: selected)

            content = NodeBuilder.buildAny(
                NavStack(
                    stack: Navigator.stack(tab.index),
                    background: navigationBackground,
                    transition: transition,
                    screenBackground: screenBackground,
                    interactiveBack: interactiveBack,
                    content: { AnyView(erasing: tab.build()) }
                )
            )
            TabBuildContext.shared.end()
        } else {
            content = SFNode.makeDefault()
            content.kind = SF_NODE_EMPTY
        }

        let bar = NodeBuilder.buildAny(
            TabBar(tabs: tabs, selected: selected, background: background))

        var children = [content, bar] + PresentationStore.shared.drain()
        let count = children.count

        var node = SFNode.makeDefault()
        node.kind = SF_NODE_STACK
        node.axis = SF_AXIS_DEPTH
        node.sizing = SF_SIZING_FILL
        node.alignment = SF_ALIGNMENT_CENTER

        node.verticalAlignment = SF_ALIGNMENT_TRAILING
        FrameArena.shared.storeNodes(&children) { pointer in
            node.children = pointer
            node.childrenLen = count
        }
        return node
    }
}

struct TabBar: View {
    let tabs: [ResolvedTab]
    let selected: Int
    let background: Material?
    
    @State var isDragging = false

    var body: some View {

        let filtered = tabs.filter { $0.role != .leading && $0.role != .trailing }
        let searchTab = tabs.first { $0.role == .trailing }
        let animator = aimedAnimator(for: filtered, searchTab: searchTab)

        let ids = filtered.map { TabBarMetrics.itemID($0.index) }
        let pill = animator.pill(measuring: ids)
        let layout = animator.layout(measuring: ids, group: TabBarMetrics.groupID)
        let anchor = animator.anchorSlot

        HStack(spacing: TabBarMetrics.groupSpacing) {
            HStack(spacing: 0) {
                ForEach(filtered.indices, id: \.self) { position in
                    TabBarItem(
                        tab: filtered[position],
                        isSelected: filtered[position].index == selected,
                        showLabel: true,
                        iconOpacity: layout.itemOpacity(slot: position, anchor: anchor),
                        labelOpacity: layout.labelOpacity(slot: position, anchor: anchor),
                        tap: { self.activate(filtered[position]) }
                    )
                }
            }
            .offset(x: layout.contentOffset)
            .gesture(dragAcross(filtered, ids: ids))
            .background(
                Capsule()
                    .fill(Color.overlay.opacity(pill.opacity))
                    .frame(width: pill.width)
                    .scale(y: pill.squash)
                    .offset(x: pill.x)
                    .scale(isDragging ? 1.07 : 1.0)
                    .animation(.easeInOut(), value: isDragging)
            )
            .frame(width: layout.rowWidth)
            .padding(.vertical, TabBarMetrics.groupInset - (selected == searchTab?.index ? 4 : 0.0))
            .padding(.horizontal, TabBarMetrics.groupInset - (selected == searchTab?.index ? 12 : 0.0))
            // Measured before the width is forced, so what it reports stays
            // the natural one no matter how far the group has collapsed.
            .measured(as: TabBarMetrics.groupID)
            .frame(width: layout.groupWidth)
            .background(
                Capsule()
                    .fill(.surface)
                    .specular()
                    .mergeable(true)
            )
            .scale(isDragging ? 1.07 : 1.0)
            .animation(.easeInOut(), value: isDragging)

            if let searchTab {
                TabBarSearchField(
                    tab: searchTab,
                    isSelected: searchTab.index == selected,
                    expansion: layout.expansion,
                    contentWidth: max(
                        TabBarMetrics.iconSize,
                        layout.trailingWidth - 2 * TabBarMetrics.searchInset
                    ),
                    tap: { self.activate(searchTab) }
                )
                .frame(width: layout.trailingWidth, height: searchTab.index == selected ? 48 : TabBarMetrics.trailingDiameter)
                .background(
                    Capsule()
                        .fill(.surface)
                        .specular()
                        .mergeable(true)
                )
            }
        }
        .padding(.horizontal, TabBarMetrics.horizontalPadding)
        .padding(.bottom, SafeArea.bottom)
        .frame(height: TabBarMetrics.contentInset, maxWidth: .infinity)
        // TODO: Make it be flipped vertically
        //.progressiveBlur(.hardMaterial, start: 0)
    }

    /// Aims the pill at the selected item and tells the bar which shape it is
    /// meant to be. The selection can sit outside the group — a `role:
    /// .trailing` tab has its own capsule — in which case the pill stays put
    /// and fades, and the group collapses behind the tab it left.
    private func aimedAnimator(
        for filtered: [ResolvedTab], searchTab: ResolvedTab?
    ) -> TabBarAnimator {
        let animator = TabBarAnimator.shared
        animator.retarget(
            to: filtered.firstIndex { $0.index == selected },
            itemCount: filtered.count,
            expanded: searchTab.map { $0.index == selected } ?? false
        )
        return animator
    }

    /// Dragging along the bar carries the selection with it: the pill tracks
    /// the finger, each item it passes becomes the selected tab, and letting
    /// go throws it the rest of the way.
    private func dragAcross(_ filtered: [ResolvedTab], ids: [UInt32]) -> DragGesture {
        let animator = TabBarAnimator.shared

        func select(_ slot: Int) {
            guard filtered.indices.contains(slot) else { return }
            let tab = filtered[slot]
            guard tab.index != Navigator.tabSelection else { return }
            Navigator.tabSelection = tab.index
        }

        return DragGesture(axis: .horizontal, minimumDistance: TabBarMetrics.dragSlop)
            .onChanged { value in
                isDragging = true
                guard animator.searchExpansion < 0.01,
                      let slot = animator.slot(atRowX: value.location.x, measuring: ids)
                else { return }
                animator.drag(to: slot)
                select(Int(slot.rounded()))
            }
            .onEnded { value in
                guard let slot = animator.slot(atRowX: value.location.x, measuring: ids)
                else { return }

                // A flick past halfway into the next tab takes it, the same
                // way a scroll's throw picks its resting page.
                let carried = animator.slot(
                    atRowX: value.location.x + value.velocity.x * TabBarMetrics.flickProjection,
                    measuring: ids
                ) ?? slot
                let landing = min(max(Int(carried.rounded()), 0), ids.count - 1)
                animator.endDrag(landingOn: landing)
                isDragging = false
                select(landing)
            }
    }

    private func activate(_ tab: ResolvedTab) {
        if tab.index == selected {
            reselect(tab)
        } else {
            Navigator.tabSelection = tab.index
        }
    }

    private func reselect(_ tab: ResolvedTab) {
        switch tab.reselect {
        case .none:
            break
        case .popToRoot:
            Navigator.stack(tab.index).popToRoot()
        case .scrollToTop:
            TabBuildContext.shared.scrollToTop(in: tab.index)
        case .run(let action):
            action()
        }
    }
}

struct TabBarItem: View {
    let tab: ResolvedTab
    let isSelected: Bool
    let showLabel: Bool

    /// What is left of the item as the group closes over it. Both ride
    /// outside the tint animation — they change every frame, and animating an
    /// animation would only make the fade lag the box that has to cover it.
    var iconOpacity: Float = 1
    var labelOpacity: Float = 1

    let tap: () -> Void

    var body: some View {
        Button(action: tap) {
            VStack(spacing: 2) {
                TabItemIcon(tab: tab, isSelected: isSelected, opacity: iconOpacity)
                if showLabel {
                    Text(tab.title)
                        // A tab label is one line. It also has to stay one
                        // line while the group closes over it, or the bar
                        // would grow taller on the way down.
                        .lineLimit(1)
                        .font(.system(size: 10, weight: isSelected ? .semibold : .regular))
                        .foregroundColor(isSelected ? .accent : .primary)
                        .animation(
                            .easeOut(duration: TabBarMetrics.tintDuration),
                            id: TabBarMetrics.labelID(tab.index)
                        )
                        .opacity(labelOpacity)
                }
            }
            // The bounce lands on the content, not on the box around it: the
            // box is what the pill measures itself against, and measurement
            // reads through scale.
            .scale(isSelected ? 1 + TabBarAnimator.shared.pop : 1)
            .padding(.vertical, 9.5 - (4.0 * (1.0 - labelOpacity)))
            .padding(.horizontal, 17 - (12.0 * (1.0 - labelOpacity)))
            // A clear layer the size of the item, which is where the pill
            // wants to land. The button's own node can't carry the id — the
            // enclosing `ForEach` relabels it with the row key.
            .background(Color.clear.measured(as: TabBarMetrics.itemID(tab.index)))
            .offset(y: showLabel ? 0 : 1)
        }
        .buttonStyle(PlainButtonStyle())
    }
}

/// The trailing tab, which is a button at rest and a search field once it is
/// the selected one.
struct TabBarSearchField: View {
    let tab: ResolvedTab
    let isSelected: Bool
    let expansion: Float

    /// The width the icon and placeholder are laid out in. Collapsed it is
    /// the icon and nothing else, which is what leaves the icon centred in
    /// its circle; expanded it is the field inside its margins, which is what
    /// puts the icon at the leading edge with the placeholder beside it.
    let contentWidth: Float

    let tap: () -> Void

    var body: some View {
        Button(action: tap) {
            HStack(spacing: TabBarMetrics.searchSpacing) {
                TabItemIcon(tab: tab, isSelected: false, opacity: 1)
                    .scale(isSelected ? 0.8 : 1.0)
                
                Text(tab.title)
                    .lineLimit(1)
                    .font(.system(size: TabBarMetrics.searchPlaceholderSize, weight: .regular))
                    .foregroundColor(.secondary)
                    .opacity(expansion)
            }
            .frame(width: contentWidth, alignment: .leading)
            .scale(isSelected ? 1 + TabBarAnimator.shared.pop : 1)
            .offset(y: 1)
        }
        .buttonStyle(PlainButtonStyle())
    }
}

/// A tab's icon, with the badge that sits on it.
struct TabItemIcon: View {
    let tab: ResolvedTab
    let isSelected: Bool
    let opacity: Float

    @ViewBuilder
    var body: some View {
        if let icon = tab.icon {
            ZStack(alignment: .topTrailing) {
                icon.resolveIcon(
                    size: TabBarMetrics.iconSize,
                    color: isSelected ? .accent : .primary
                )
                .fontWeight(isSelected ? .black : .semibold)
                .animation(
                    .easeOut(duration: TabBarMetrics.tintDuration),
                    id: TabBarMetrics.iconID(tab.index)
                )
                .opacity(opacity)
                badge
            }
        }
    }

    @ViewBuilder
    private var badge: some View {
        if let count = tab.badge {
            Text(count > 99 ? "99+" : "\(count)")
                .font(.system(size: 10, weight: .bold))
                .foregroundColor(Color.primary.opacity(opacity))
                .padding(.horizontal, 5)
                .padding(.vertical, 1)
                .background(Capsule().fill(Color.accent.opacity(opacity)))
                .offset(x: 10, y: -4)
        }
    }
}

final class TabBuildContext {
    nonisolated(unsafe) static let shared = TabBuildContext()

    nonisolated(unsafe) private var building: Int?
    nonisolated(unsafe) private var scrolls: [Int: UInt32] = [:]

    var isBuildingTab: Bool { building != nil }

    func begin(tab: Int) {
        building = tab
        scrolls[tab] = nil
    }

    func end() {
        building = nil
    }

    func register(_ scrollID: UInt32, axis: Axis) {
        guard let building, axis == .vertical, scrolls[building] == nil else { return }
        scrolls[building] = scrollID
    }

    func scrollToTop(in tab: Int) {
        guard let id = scrolls[tab] else { return }
        NodeRegistry.shared.scrollState(for: id).scrollToTop()
    }
}

extension Image {

    public static func system(_ name: String) -> Image { Image(name) }

    public static var house: Image { .system("house") }
    public static var magnifyingGlass: Image { .system("magnifyingglass") }
    public static var person: Image { .system("person") }
    public static var bell: Image { .system("bell") }
    public static var gear: Image { .system("gear") }
}

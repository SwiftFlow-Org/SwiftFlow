import CSwiftFlow

public struct NavStack<Root: View>: View {
    public typealias Body = Never
    public var body: Never { fatalError() }

    let explicitStack: NavigationStackModel?
    let background: Material?
    let transition: NavTransition
    let screenBackground: Color
    let interactiveBack: Bool
    let root: () -> Root

    var stack: NavigationStackModel { explicitStack ?? Navigator.current }

    static var backDrag: DragGesture {
        DragGesture(axis: .horizontal, minimumDistance: 12, edge: .leading, edgeWidth: 24)
    }

    public init(
        stack: NavigationStackModel? = nil,
        background: Material? = Material(blurRadius: 50, tint: .background, isProgressive: true),
        transition: NavTransition = .standard,
        screenBackground: Color = .background,
        interactiveBack: Bool = true,
        @ViewBuilder content: @escaping () -> Root
    ) {
        self.explicitStack = stack
        self.background = background
        self.transition = transition
        self.screenBackground = screenBackground
        self.interactiveBack = interactiveBack
        self.root = content
    }
}

extension NavStack {
    public func toSFNode() -> SFNode {

        let stack = self.stack
        let back = stack.isAtRoot
            ? nil
            : backItem(transitions: stack.path.count <= 1)

        let context = NavContext.shared
        let saved = context.push
        context.push = { view in stack.push(view) }
        defer { context.push = saved }

        stack.transitionStyle = transition

        let depth = stack.path.count
        let width = max(DeviceMetrics.screenWidth, 1)
        let progress = stack.transitionProgress

        let levels: [Int] = (progress != nil && depth > 0) ? [depth - 1, depth] : [depth]

        var screens: [Int: AnyView] = [:]
        for level in levels {

            let content = AnyView(
                NavigationLayer(
                    content: layer(at: level),
                    background: background,
                    identity: fnv1a("navlayer#\(level)"),
                    publishesItems: level == depth
                )
            )
            screens[level] = screen(
                content,
                offsetX: slide(level: level, depth: depth, progress: progress, width: width)
            )
        }

        let built = screens

        if let back {
            NavigationConfigStore.shared.addItems([back], first: true)
        }

        let chrome = NavigationStack(
            background: background,
            insetsContent: false,
            transitionProgress: progress,

            barID: fnv1a("navbar#\(UInt(bitPattern: ObjectIdentifier(stack).hashValue))")
        ) {

            ForEach(levels, id: \.self) { level in

                built[level]
            }
        }

        let gestured: AnyView
        if interactiveBack && !stack.isAtRoot {
            gestured = AnyView(chrome.gesture(backGesture(width: width)))
        } else {
            gestured = AnyView(chrome)
        }

        guard !stack.modals.isEmpty else { return gestured.toSFNode() }

        if TabBuildContext.shared.isBuildingTab {
            PresentationStore.shared.hand(over: ModalHost.layerNodes(for: stack))
            return gestured.toSFNode()
        }
        return ModalHost(base: gestured, stack: stack).toSFNode()
    }

    private func layer(at level: Int) -> AnyView {
        guard level > 0, level <= stack.path.count else {
            return AnyView(root())
        }
        return AnyView(erasing: stack.path[level - 1].view)
    }

    private func screen(_ content: AnyView, offsetX: Float) -> AnyView {
        AnyView(
            content
                .frame(maxWidth: .infinity, maxHeight: .infinity)

                .clipShape(RoundedRectangle(cornerRadius: DeviceMetrics.screenCornerRadius))
                .background(
                    RoundedRectangle(cornerRadius: DeviceMetrics.screenCornerRadius)
                        .fill(screenBackground)
                        .mergeable(false)
                        .shadow(radius: 18, opacity: 0.3)
                )

                .offset(x: offsetX)
        )
    }

    private func slide(level: Int, depth: Int, progress: Float?, width: Float) -> Float {
        guard let progress else { return 0 }
        return level == depth
            ? width * progress
            : -width * transition.parallax * (1 - progress)
    }

    private func backGesture(width: Float) -> DragGesture {
        let threshold = transition.commitThreshold
        let stack = self.stack
        return Self.backDrag
            .onChanged { value in
                stack.updateInteractiveBack(value.translation.x / width)
            }
            .onEnded { value in
                let projected = value.predictedEndTranslation.x / width
                stack.endInteractiveBack(
                    commit: projected > threshold,
                    velocity: value.velocity.x / width
                )
            }
    }

    private func backItem(transitions: Bool) -> ResolvedToolbarItem {
        let stack = self.stack
        let button = Button {
            stack.pop()
        } label: {
            Icon.caretLeft.size(22).weight(.bold)
        }
        .buttonStyle(NavBarButtonStyle())

        return ResolvedToolbarItem(
            placement: .topBarLeading,
            node: NodeBuilder.buildAny(button),
            transitions: transitions
        )
    }
}

struct NavigationLayer<Content: View>: View {
    typealias Body = Never
    var body: Never { fatalError() }

    let content: Content
    let background: Material?

    let identity: UInt32

    let publishesItems: Bool

    func toSFNode() -> SFNode {
        let store = NavigationConfigStore.shared
        let saved = store.beginCollecting()
        var node = content.toSFNode()
        let config = store.endCollecting(restoring: saved)

        let mode = config.resolvedDisplayMode
        let scale = DeviceScale.current

        let inset = NavigationBarMetrics.contentInset(mode: mode, hasTitle: config.title != nil)
        applyContentInset(
            &node,
            top: inset * scale,
            edge: background.map {
                EdgeEffect(material: $0, height: barHeight(config, mode) * scale)
            }
        )

        if publishesItems {
            store.addItems(config.items)
        }

        let titles = NavigationTitles(
            config: config,
            mode: mode,
            isCollapsed: config.isTitleCollapsed,
            scrolled: config.largeTitleScroll,

            animationID: fnv1a("navtitle#\(identity)#\(config.scrollID ?? 0)")
        )

        var children = [node, NodeBuilder.buildAny(titles)]
        let count = children.count

        var stack = SFNode.makeDefault()
        stack.kind = SF_NODE_STACK
        stack.axis = SF_AXIS_DEPTH
        stack.sizing = SF_SIZING_FILL
        stack.alignment = SF_ALIGNMENT_CENTER
        stack.verticalAlignment = SF_ALIGNMENT_LEADING
        FrameArena.shared.storeNodes(&children) { pointer in
            stack.children = pointer
            stack.childrenLen = count
        }
        return stack
    }
}

public struct NavLink<Label: View>: View {
    public typealias Body = Never
    public var body: Never { fatalError() }

    let destination: any View
    let label: Label

    public init(_ destination: any View, @ViewBuilder label: () -> Label) {
        self.destination = destination
        self.label = label()
    }
}

extension NavLink {
    public func toSFNode() -> SFNode {

        let push = NavContext.shared.push ?? { Navigator.push($0) }
        let destination = self.destination
        let button = Button {
            push(destination)
        } label: {
            label
        }
        .buttonStyle(PlainButtonStyle())
        return button.toSFNode()
    }
}

final class NavContext {
    nonisolated(unsafe) static let shared = NavContext()
    nonisolated(unsafe) var push: ((any View) -> Void)?
}

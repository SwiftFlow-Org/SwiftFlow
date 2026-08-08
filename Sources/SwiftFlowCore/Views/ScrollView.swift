import CSwiftFlow

/// A layout direction.
public enum Axis: Sendable {
    case vertical
    case horizontal
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

    private init(axis: Axis, scrollID: UInt32, content: @escaping (ScrollGeometry) -> Content) {
        self.axis = axis
        self.scrollID = scrollID
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

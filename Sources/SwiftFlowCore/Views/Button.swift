import CSwiftFlow

/// A control that performs an action when tapped.
public struct Button<Label: View>: View {
    public typealias Body = Never
    public var body: Never { fatalError() }

    let action    : () -> Void
    let label     : Label
    var style     : any ButtonStyle = DefaultButtonStyle()
    var isPressed : Bool = false

    public init(action: @escaping () -> Void, @ViewBuilder label: () -> Label) {
        self.action = action
        self.label  = label()
    }

    public init(_ title: String, action: @escaping () -> Void) where Label == Text {
        self.action = action
        self.label  = Text(title)
    }
}

extension Button {
    public func toSFNode() -> SFNode {
        let id = BuildContext.shared.currentID(for: self)

        let pressed = NodeRegistry.shared.pressedNodes.contains(id)

        let config = ButtonStyleConfiguration(
            label:     AnyView(label),
            isPressed: pressed,
            id:        id
        )

        var node     = style.makeBody(configuration: config).toSFNode()
        node.node_id = id

        NodeRegistry.shared.registerTap(id, action: action)
        return node
    }
}

public extension Button {
    func buttonStyle<S: ButtonStyle>(_ style: S) -> Button<Label> {
        var copy = self
        copy.style = style
        return copy
    }
}

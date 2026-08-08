/// A custom appearance for a button.
public protocol ButtonStyle {
    associatedtype Body: View
    @ViewBuilder func makeBody(configuration: ButtonStyleConfiguration) -> Body
}

public struct ButtonStyleConfiguration {
    public let label: AnyView
    public let isPressed: Bool

    public let id: UInt32
}

public struct DefaultButtonStyle: ButtonStyle {
    public init() {}
    public func makeBody(configuration: ButtonStyleConfiguration) -> some View {
        configuration.label
            .padding(.vertical, 12)
            .padding(.horizontal, 22)
            .background(
                Capsule()
                    .fill(.accent)
                    .specular()

            )
            .scale(configuration.isPressed ? 1.10 : 1.0)
            .animation(.spring(), id: configuration.id)
    }
}

public struct NavBarButtonStyle: ButtonStyle {
    public init() {}
    public func makeBody(configuration: ButtonStyleConfiguration) -> some View {
        configuration.label
            .padding(.vertical, 8)
            .padding(.horizontal, 7)
            .frame(width: 44, height: 44)
            .glassEffect(.clear, in: Circle())

            .scale(configuration.isPressed ? 1.10 : 1.0)
            .animation(.spring(), id: configuration.id)
    }
}

public struct PlainButtonStyle: ButtonStyle {
    public init() {}
    public func makeBody(configuration: ButtonStyleConfiguration) -> some View {
        configuration.label
            .opacity(configuration.isPressed ? 0.6 : 1.0)
    }
}

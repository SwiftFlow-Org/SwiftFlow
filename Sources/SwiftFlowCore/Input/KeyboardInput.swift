import CSwiftFlow

public enum Key: UInt32, Sendable {
    case other = 0
    case backspace, delete, enter, tab, escape
    case left, right, up, down
    case home, end, pageUp, pageDown

    public init(raw: UInt32) {
        self = Key(rawValue: raw) ?? .other
    }
}

public struct KeyModifiers: OptionSet, Sendable {
    public let rawValue: UInt32
    public init(rawValue: UInt32) { self.rawValue = rawValue }

    public static let shift = KeyModifiers(rawValue: 1 << 0)
    public static let control = KeyModifiers(rawValue: 1 << 1)

    public static let option = KeyModifiers(rawValue: 1 << 2)

    public static let command = KeyModifiers(rawValue: 1 << 3)

    public static var primary: KeyModifiers {
        #if os(macOS) || os(iOS)
            return .command
        #else
            return .control
        #endif
    }
}

public struct TextInputModifier<Content: View>: View {
    public typealias Body = Never
    public var body: Never { fatalError() }

    let content: Content
    let isFocused: Bool
    let onInsert: (String) -> Void
    let onKey: (Key, KeyModifiers) -> Bool
}

extension TextInputModifier {
    public func toSFNode() -> SFNode {
        var node = content.toSFNode()

        let id = node.node_id != 0 ? node.node_id : BuildContext.shared.currentID(for: self)
        node.node_id = id

        TextInput.shared.register(
            id,
            TextInputHandlers(
                insert: onInsert,
                key: { code, modifiers in
                    onKey(Key(raw: code), KeyModifiers(rawValue: modifiers))
                }
            )
        )

        if isFocused {

            TextInput.shared.focus(id)
        } else if TextInput.shared.isFocused(id) {
            TextInput.shared.resignFocus()
        }

        return node
    }
}

extension View {

    /// Receives typed text and key commands while this view has focus.
    ///
    /// The raw channel, for an editor. For an ordinary field use `TextField`.
    /// Return `true` from `onKey` to consume the key.
    public func textInput(
        isFocused: Bool = true,
        onInsert: @escaping (String) -> Void,
        onKey: @escaping (Key, KeyModifiers) -> Bool = { _, _ in false }
    ) -> TextInputModifier<Self> {
        TextInputModifier(
            content: self,
            isFocused: isFocused,
            onInsert: onInsert,
            onKey: onKey
        )
    }
}

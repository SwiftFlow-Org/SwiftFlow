import CSwiftFlow

public enum Key: Sendable, Equatable {
    case other

    // Special keys
    case backspace
    case delete
    case enter
    case tab
    case escape
    case left
    case right
    case up
    case down
    case home
    case end
    case pageUp
    case pageDown
    case space

    // Unicode scalar
    case unicode(UInt32)

    public init(raw: UInt32) {
        print("Raw: \(raw)")
        switch raw {
        case 0:
            self = .other
        case 1:
            self = .backspace
        case 2:
            self = .delete
        case 3:
            self = .enter
        case 4:
            self = .tab
        case 5:
            self = .escape
        case 6:
            self = .left
        case 7:
            self = .right
        case 8:
            self = .up
        case 9:
            self = .down
        case 10:
            self = .home
        case 11:
            self = .end
        case 12:
            self = .pageUp
        case 13:
            self = .pageDown
        case 14:
            self = .space

        default:
            if raw <= 0x10FFFF,
               let scalar = Unicode.Scalar(raw)
            {
                self = .unicode(raw)
            } else {
                self = .other
            }
        }
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
            if TextInput.shared.focused == nil || TextInput.shared.isFocused(id) {
                TextInput.shared.focus(id)
            }
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

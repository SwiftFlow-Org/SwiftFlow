import CSwiftFlow

final class TextFieldState {

    var caret: Int = 0

    var blinkPhase: Float = 0

    var caretVisible: Bool { blinkPhase.truncatingRemainder(dividingBy: 1.06) < 0.53 }

    func resetBlink() { blinkPhase = 0 }

    func clamp(to text: String) {
        caret = max(0, min(caret, text.count))
    }
}

final class TextFieldRegistry {
    nonisolated(unsafe) static let shared = TextFieldRegistry()
    nonisolated(unsafe) private var states: [UInt32: TextFieldState] = [:]

    func state(for id: UInt32) -> TextFieldState {
        if let existing = states[id] { return existing }
        let created = TextFieldState()
        states[id] = created
        return created
    }

    func step(dt: Float) {
        guard let focused = TextInput.shared.focused,
              let state = states[focused]
        else { return }
        let before = state.caretVisible
        state.blinkPhase += dt
        if state.caretVisible != before {
            NodeRegistry.shared.needsRender = true
        }
    }
}

enum TextFieldMetrics {
    static let height: Float = 44
    static let cornerRadius: Float = 10
    static let insets = EdgeInsets(top: 0, bottom: 0, leading: 12, trailing: 12)

    static let caretWidth: Float = 2

    static let caretHeightRatio: Float = 1.15
}

/// A control for entering a single line of text.
///
/// ```swift
/// @State private var name = ""
///
/// TextField("Your name", text: $name)
/// ```
///
/// Tap to focus. Handles committed text, an input method's composition,
/// backspace and delete, the arrow keys, home and end, and escape to give
/// up focus.
public struct TextField: View {
    public typealias Body = Never
    public var body: Never { fatalError() }

    let placeholder: String
    let text: Binding<String>
    let font: Font
    let onSubmit: (() -> Void)?

    let id: UInt32

    /// Creates a text field with a placeholder and a binding to its contents.
    ///
    /// - Parameters:
    ///   - placeholder: Shown while the field is empty and unfocused.
    ///   - text: The text to display and edit.
    ///   - font: The font for both the text and the placeholder.
    ///   - onSubmit: Called when the return key is pressed.
    public init(
        _ placeholder: String,
        text: Binding<String>,
        font: Font = .body,
        onSubmit: (() -> Void)? = nil,
        fileID: String = #fileID, line: Int = #line, column: Int = #column
    ) {
        self.placeholder = placeholder
        self.text = text
        self.font = font
        self.onSubmit = onSubmit
        self.id = fnv1a("textfield:\(fileID):\(line):\(column)")
    }

    private func insert(_ inserted: String, into state: TextFieldState) {
        var value = text.wrappedValue
        state.clamp(to: value)
        let at = value.index(value.startIndex, offsetBy: state.caret)
        value.insert(contentsOf: inserted, at: at)
        text.wrappedValue = value
        state.caret += inserted.count
        state.resetBlink()
    }

    private func handle(_ key: Key, _ modifiers: KeyModifiers, _ state: TextFieldState) -> Bool {
        var value = text.wrappedValue
        state.clamp(to: value)

        switch key {
        case .backspace:
            guard state.caret > 0 else { return true }
            let at = value.index(value.startIndex, offsetBy: state.caret - 1)
            value.remove(at: at)
            text.wrappedValue = value
            state.caret -= 1

        case .delete:
            guard state.caret < value.count else { return true }
            let at = value.index(value.startIndex, offsetBy: state.caret)
            value.remove(at: at)
            text.wrappedValue = value

        case .left:
            state.caret = max(0, state.caret - 1)

        case .right:
            state.caret = min(value.count, state.caret + 1)

        case .home, .up:
            state.caret = 0

        case .end, .down:
            state.caret = value.count

        case .enter:
            onSubmit?()

        case .escape:
            TextInput.shared.resignFocus()

        default:

            return false
        }

        state.resetBlink()
        return true
    }

    public func toSFNode() -> SFNode {
        let state = TextFieldRegistry.shared.state(for: id)
        let value = text.wrappedValue
        state.clamp(to: value)

        let isFocused = TextInput.shared.isFocused(id)
        let preedit = isFocused ? TextInput.shared.preedit?.text ?? "" : ""

        let field = ZStack(alignment: .leading) {
            RoundedRectangle(cornerRadius: TextFieldMetrics.cornerRadius)
                .fill(.fill)
                .frame(maxWidth: .infinity, maxHeight: .infinity)

            if value.isEmpty && preedit.isEmpty && !isFocused {
                Text(placeholder)
                    .font(font)
                    .foregroundColor(.placeholder)
                    .lineLimit(1)
            } else {
                content(value: value, preedit: preedit, state: state, isFocused: isFocused)
            }
        }
        .padding(TextFieldMetrics.insets)
        .frame(height: TextFieldMetrics.height, maxWidth: .infinity, alignment: .leading)
        // TODO: put the caret where the tap was. Needs glyph positions back from
        // the renderer; nothing reports them yet, so it goes to the end.
        .onTap { _ in
            TextInput.shared.focus(self.id)

            TextFieldRegistry.shared.state(for: self.id).caret = self.text.wrappedValue.count
        }
        .textInput(
            isFocused: isFocused,
            onInsert: { inserted in
                self.insert(inserted, into: TextFieldRegistry.shared.state(for: self.id))
            },
            onKey: { key, modifiers in
                self.handle(key, modifiers, TextFieldRegistry.shared.state(for: self.id))
            }
        )

        var node = field.toSFNode()
        node.node_id = id
        return node
    }

    private func content(
        value: String,
        preedit: String,
        state: TextFieldState,
        isFocused: Bool
    ) -> some View {
        let split = value.index(value.startIndex, offsetBy: state.caret)
        let before = String(value[value.startIndex..<split])
        let after = String(value[split...])

        return HStack(alignment: .center, spacing: 0) {
            Text(before).font(font).foregroundColor(.primary).lineLimit(1)

            if !preedit.isEmpty {

                VStack(alignment: .leading, spacing: 1) {
                    Text(preedit).font(font).foregroundColor(.primary).lineLimit(1)
                    RoundedRectangle(cornerRadius: 0)
                        .fill(.primary)
                        .frame(height: 1, maxWidth: .infinity)
                }
            }

            if isFocused && state.caretVisible {
                RoundedRectangle(cornerRadius: TextFieldMetrics.caretWidth / 2)
                    .fill(.accent)
                    .frame(
                        width: TextFieldMetrics.caretWidth,
                        height: font.size * TextFieldMetrics.caretHeightRatio
                    )
            } else {

                Spacer().frame(width: TextFieldMetrics.caretWidth)
            }

            Text(after).font(font).foregroundColor(.primary).lineLimit(1)

            Spacer()
        }
    }
}

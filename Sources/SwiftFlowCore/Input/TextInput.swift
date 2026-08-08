import CSwiftFlow

public struct TextInputHandlers {

    public var insert: (String) -> Void

    public var key: (UInt32, UInt32) -> Bool

    public init(
        insert: @escaping (String) -> Void,
        key: @escaping (UInt32, UInt32) -> Bool = { _, _ in false }
    ) {
        self.insert = insert
        self.key = key
    }
}

public struct Preedit: Equatable {
    public let text: String

    public let cursor: Range<Int>?

    public init(text: String, cursor: Range<Int>?) {
        self.text = text
        self.cursor = cursor
    }
}

public final class TextInput {
    nonisolated(unsafe) public static let shared = TextInput()

    nonisolated(unsafe) public private(set) var focused: UInt32?

    nonisolated(unsafe) public private(set) var preedit: Preedit?

    nonisolated(unsafe) private var handlers: [UInt32: TextInputHandlers] = [:]

    nonisolated(unsafe) public var setIMEAllowed: ((Bool) -> Void)?

    nonisolated(unsafe) public var setIMECursorArea: ((Float, Float, Float, Float) -> Void)?

    public func register(_ node: UInt32, _ handlers: TextInputHandlers) {
        self.handlers[node] = handlers
    }

    public func focus(_ node: UInt32) {
        guard focused != node else { return }
        focused = node
        preedit = nil
        setIMEAllowed?(true)
        NodeRegistry.shared.needsRender = true
    }

    public func resignFocus() {
        guard focused != nil else { return }
        focused = nil

        preedit = nil
        setIMEAllowed?(false)
        NodeRegistry.shared.needsRender = true
    }

    public func isFocused(_ node: UInt32) -> Bool { focused == node }

    public func reportCaret(x: Float, y: Float, width: Float, height: Float) {
        setIMECursorArea?(x, y, width, height)
    }

    public func commit(_ text: String) {
        preedit = nil
        guard let focused, let handler = handlers[focused], !text.isEmpty else { return }
        handler.insert(text)
        NodeRegistry.shared.needsRender = true
    }

    public func setPreedit(_ text: String, cursorBegin: Int, cursorEnd: Int) {

        if text.isEmpty {
            preedit = nil
        } else {
            let cursor = cursorBegin >= 0 && cursorEnd >= cursorBegin
                ? cursorBegin..<cursorEnd
                : nil
            preedit = Preedit(text: text, cursor: cursor)
        }
        NodeRegistry.shared.needsRender = true
    }

    public func imeEnabled(_ enabled: Bool) {
        if !enabled { preedit = nil }
        NodeRegistry.shared.needsRender = true
    }

    @discardableResult
    public func key(_ code: UInt32, modifiers: UInt32, pressed: Bool, isRepeat: Bool) -> Bool {

        guard pressed else { return false }
        guard let focused, let handler = handlers[focused] else { return false }
        let handled = handler.key(code, modifiers)
        if handled { NodeRegistry.shared.needsRender = true }
        return handled
    }

    public func beginBuild() {
        handlers.removeAll(keepingCapacity: true)
    }

    func rekey(from old: UInt32, to new: UInt32) {
        if let handler = handlers.removeValue(forKey: old) { handlers[new] = handler }
        if focused == old { focused = new }
    }
}

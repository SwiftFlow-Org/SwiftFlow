import CSwiftFlow

/// A key going down or up, for app-wide shortcuts.
public struct KeyCommandEvent: Sendable {
    public let key: Key
    public let modifiers: KeyModifiers
    public let isPressed: Bool
    public let isRepeat: Bool

    /// The key as a lowercase character, with Shift's symbols mapped back
    /// to their unshifted keys on a US layout (`{` reads as `[`), so a
    /// shortcut can be named by the key printed on the keycap.
    public var character: Character? {
        guard case .unicode(let raw) = key, let scalar = Unicode.Scalar(raw) else { return nil }
        let c = Character(scalar)
        if let unshifted = Self.unshifted[c] { return unshifted }
        return Character(c.lowercased())
    }

    private static let unshifted: [Character: Character] = [
        "{": "[", "}": "]", "+": "=", "_": "-", ":": ";", "\"": "'",
        "<": ",", ">": ".", "?": "/", "|": "\\", "~": "`",
        "!": "1", "@": "2", "#": "3", "$": "4", "%": "5",
        "^": "6", "&": "7", "*": "8", "(": "9", ")": "0",
    ]
}

/// App-wide keyboard shortcuts.
///
/// The host offers every key press and release here that a focused text
/// field did not take. While a text field has focus, only presses with ⌘ or
/// Ctrl held come here, so typing a letter never fires a shortcut.
///
/// ```swift
/// KeyCommands.shared.handler = { event in
///     guard event.isPressed, event.modifiers.contains(.primary),
///           event.character == "z" else { return false }
///     undo()
///     return true
/// }
/// ```
public final class KeyCommands {
    nonisolated(unsafe) public static let shared = KeyCommands()

    /// Returns whether it handled the event.
    nonisolated(unsafe) public var handler: ((KeyCommandEvent) -> Bool)?

    /// Called by the hosts.
    @discardableResult
    public func dispatch(code: UInt32, modifiers: UInt32, pressed: Bool, isRepeat: Bool) -> Bool {
        guard let handler else { return false }
        let mods = KeyModifiers(rawValue: modifiers)
        if TextInput.shared.hasFocus && !(mods.contains(.command) || mods.contains(.control)) {
            return false
        }
        let handled = handler(KeyCommandEvent(
            key: Key(raw: code), modifiers: mods, isPressed: pressed, isRepeat: isRepeat
        ))
        if handled { NodeRegistry.shared.needsRender = true }
        return handled
    }
}

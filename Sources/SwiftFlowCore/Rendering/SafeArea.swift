/// The insets that keep content clear of the system's own chrome.
public enum SafeArea {
    nonisolated(unsafe) public static var top: Float = 0
    nonisolated(unsafe) public static var bottom: Float = 0
    nonisolated(unsafe) public static var leading: Float = 0
    nonisolated(unsafe) public static var trailing: Float = 0
}

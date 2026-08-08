/// The size of the screen, in points.
public enum Screen {
    nonisolated(unsafe) public static var width: Float = 0
    nonisolated(unsafe) public static var height: Float = 0

    public static var isKnown: Bool { width > 0 && height > 0 }
}

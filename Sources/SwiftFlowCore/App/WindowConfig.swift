public struct WindowConfig {
    public var minWidth: Double? = nil
    public var minHeight: Double? = nil

    public init() {}
}

public extension Scene {
    func minWindowSize(width: Double, height: Double) -> Self {
        var s = self
        s.windowConfig.minWidth = width
        s.windowConfig.minHeight = height
        return s
    }
}

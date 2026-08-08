public enum DeviceScale {
    nonisolated(unsafe) public static var current: Float = 1.0
}

public enum DeviceMetrics {
    nonisolated(unsafe) public static var screenWidth: Float = 0
    nonisolated(unsafe) public static var screenHeight: Float = 0

    nonisolated(unsafe) public static var screenCornerRadius: Float = 44
}

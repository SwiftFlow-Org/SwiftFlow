import CSwiftFlow

public final class ListRowHeights {
    nonisolated(unsafe) public static let shared = ListRowHeights()

    nonisolated(unsafe) private var heights: [UInt32: Float] = [:]

    static let estimate: Float = 44

    static func id(list: UInt32, element: Int) -> UInt32 {
        fnv1a("listrow:\(list)#\(element)")
    }

    func height(for id: UInt32) -> Float {
        heights[id] ?? ListRowHeights.estimate
    }

    func isMeasured(_ id: UInt32) -> Bool {
        heights[id] != nil
    }

    public func record(_ id: UInt32, physicalHeight: Float) {
        guard physicalHeight > 0 else { return }
        let scale = DeviceScale.current > 0 ? DeviceScale.current : 1
        heights[id] = physicalHeight / scale
    }
}

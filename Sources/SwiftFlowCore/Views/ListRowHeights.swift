import CSwiftFlow

public final class ListRowHeights {
    nonisolated(unsafe) public static let shared = ListRowHeights()
    nonisolated(unsafe) private var heights: [UInt32: Float] = [:]
    nonisolated(unsafe) private var owner: [UInt32: UInt32] = [:]
    nonisolated(unsafe) private var means: [UInt32: (total: Float, count: Int)] = [:]
    
    static func id(list: UInt32, element: Int) -> UInt32 {
        fnv1a("listrow:\(list)#\(element)")
    }

    func register(_ id: UInt32, list: UInt32) { owner[id] = list }

    func height(for id: UInt32, fallback: Float) -> Float {
        if let measured = heights[id] { return measured }
        guard let list = owner[id], let mean = means[list], mean.count > 0 else {
            return fallback
        }
        return mean.total / Float(mean.count)
    }

    public func record(_ id: UInt32, physicalHeight: Float) {
        guard physicalHeight > 0 else { return }
        let scale = DeviceScale.current > 0 ? DeviceScale.current : 1
        let logical = physicalHeight / scale
        let previous = heights.updateValue(logical, forKey: id)
        guard let list = owner[id] else { return }
        var mean = means[list] ?? (total: 0, count: 0)
        if let previous { mean.total -= previous } else { mean.count += 1 }
        mean.total += logical
        means[list] = mean
    }
}


import CSwiftFlow

public final class NodeFrames {
    nonisolated(unsafe) public static let shared = NodeFrames()

    nonisolated(unsafe) private var registered: Set<UInt32> = []
    nonisolated(unsafe) private var frames: [UInt32: SFRect] = [:]

    nonisolated(unsafe) public private(set) var lastPressed: UInt32 = 0

    public func register(_ id: UInt32) {
        registered.insert(id)
    }

    public func notePressed(_ id: UInt32) {
        lastPressed = id
    }

    public var tracked: Set<UInt32> {
        var all = registered
        all.formUnion(NodeRegistry.shared.pressedNodes)
        if lastPressed != 0 {
            all.insert(lastPressed)
        }
        return all
    }

    public func record(_ id: UInt32, frame: SFRect) {
        if frame.width > 0 && frame.height > 0 {
            frames[id] = frame
        } else {
            frames.removeValue(forKey: id)
        }
    }

    public func frame(for id: UInt32) -> SFRect? {
        frames[id]
    }

    public func beginFrame() {
        registered.removeAll(keepingCapacity: true)
    }

    public static func id(for name: String) -> UInt32 {
        fnv1a("morph:\(name)")
    }
}

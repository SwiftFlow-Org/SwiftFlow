import Foundation

func fnv1a(_ string: String) -> UInt32 {
    var hash: UInt32 = 2166136261
    for byte in string.utf8 {
        hash ^= UInt32(byte)
        hash = hash &* 16777619
    }
    return hash
}

final class BuildContext {
    nonisolated(unsafe)
    static let shared = BuildContext()

    private var path     : [Int] = []
    private var counters : [Int] = [0]

    func push() {
        path.append(counters[counters.count - 1])
        counters.append(0)
    }

    func pop() {
        if !path.isEmpty     { path.removeLast() }
        if counters.count > 1 { counters.removeLast() }
    }

    func advance() {
        counters[counters.count - 1] += 1
    }

    func reset() {
        path     = []
        counters = [0]
    }

    func currentID<V: View>(for view: V) -> UInt32 {
        let typeName = String(describing: type(of: view))
        let pathStr  = path.map(String.init).joined(separator: ".")
        return fnv1a("\(typeName)@\(pathStr)")
    }
}

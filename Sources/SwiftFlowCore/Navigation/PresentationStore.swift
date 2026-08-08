import CSwiftFlow

final class PresentationStore {
    nonisolated(unsafe) static let shared = PresentationStore()

    nonisolated(unsafe) private var pending: [SFNode] = []

    func hand(over layers: [SFNode]) {
        pending.append(contentsOf: layers)
    }

    func drain() -> [SFNode] {
        defer { pending.removeAll(keepingCapacity: true) }
        return pending
    }
}

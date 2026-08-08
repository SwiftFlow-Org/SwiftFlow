final class LayerTransitionRegistry {
    nonisolated(unsafe) static let shared = LayerTransitionRegistry()

    private var settling: [ObjectIdentifier: (Float) -> Bool] = [:]

    func settle(_ owner: ObjectIdentifier, step: @escaping (Float) -> Bool) {
        settling[owner] = step
    }

    func cancel(_ owner: ObjectIdentifier) {
        settling.removeValue(forKey: owner)
    }

    var hasActiveAnimations: Bool { !settling.isEmpty }

    func step(dt: Float) {
        guard !settling.isEmpty else { return }

        for (owner, advance) in settling where !advance(dt) {
            settling.removeValue(forKey: owner)
        }
    }
}

func criticallyDampedStep(
    value: Float, velocity: Float, target: Float, response: Float, dt: Float
) -> (value: Float, velocity: Float, settled: Bool) {

    let step = min(dt, 1.0 / 30.0)
    let omega = (2 * Float.pi) / max(response, 0.01)

    let displacement = value - target
    var v = velocity + (-omega * omega * displacement - 2 * omega * velocity) * step
    var x = value + v * step

    if abs(x - target) < 0.002 && abs(v) < 0.02 {
        x = target
        v = 0
        return (x, v, true)
    }
    return (x, v, false)
}

import CSwiftFlow
import Foundation

public final class AnimationState {
    private(set) var current: AnimatableSnapshot = .zero
    private var from: AnimatableSnapshot = .zero
    private var velocity: AnimatableSnapshot = .zero
    private var target: AnimatableSnapshot = .zero

    private(set) var curve: Animation = .default
    private var elapsed: Double = 0
    private var hasTarget = false
    public private(set) var isAnimating = false

    func retarget(to newTarget: AnimatableSnapshot, curve: Animation) {
        guard hasTarget else {

            current = newTarget
            from = newTarget
            target = newTarget
            velocity = .zero
            hasTarget = true
            return
        }
        guard newTarget != target else { return }

        if newTarget.cornerRadius == -1 || current.cornerRadius == -1 {
            current.cornerRadius = newTarget.cornerRadius

            velocity.cornerRadius = 0
        }

        if newTarget.sizingX != SF_SIZING_FIXED || current.sizingX != SF_SIZING_FIXED {
            current.fixedWidth = newTarget.fixedWidth
            velocity.fixedWidth = 0
        }
        if newTarget.sizingY != SF_SIZING_FIXED || current.sizingY != SF_SIZING_FIXED {
            current.fixedHeight = newTarget.fixedHeight
            velocity.fixedHeight = 0
        }
        current.sizingX = newTarget.sizingX
        current.sizingY = newTarget.sizingY

        from = current
        target = newTarget
        self.curve = curve
        elapsed = 0
        isAnimating = true
    }

    private var observed: AnyEquatableBox?

    func retargetIfObservedChanged(
        to newTarget: AnimatableSnapshot, curve: Animation, observed newObserved: AnyEquatableBox
    ) {
        defer { observed = newObserved }

        guard let previous = observed else {

            retarget(to: newTarget, curve: curve)
            return
        }
        if previous == newObserved {

            guard !isAnimating else { return }
            current = newTarget
            target = newTarget
            velocity = .zero
            return
        }
        retarget(to: newTarget, curve: curve)
    }

    @discardableResult
    public func step(dt: Float) -> Bool {
        guard isAnimating else { return false }

        switch curve.kind {
        case .linear, .easeIn, .easeOut, .easeInOut:
            guard let duration = curve.kind.duration, duration > 0 else {
                current = target
                isAnimating = false
                return false
            }
            elapsed += Double(dt)
            let t = min(Float(elapsed / duration), 1)
            let eased = curve.eased(t)
            current = AnimatableSnapshot.lerp(from, target, eased)
            if t >= 1 {
                current = target
                isAnimating = false
            }

        case .spring(let response, let dampingFraction):
            let (newCurrent, newVelocity, settled) = AnimatableSnapshot.springStep(
                current: current, velocity: velocity, target: target,
                response: Float(response), dampingFraction: Float(dampingFraction), dt: dt)
            current = newCurrent
            velocity = newVelocity
            if settled {
                current = target
                velocity = .zero
                isAnimating = false
            }
        }

        return isAnimating
    }
}

import CSwiftFlow
import Foundation

public final class ScrollPhysicsState {

    public var axis: Axis = .vertical

    public var offset: Float = 0
    public var velocity: Float = 0

    public var contentLength: Float = 0

    public var viewportLength: Float = 0
    public var isDragging: Bool = false
    public var isSettling: Bool = false
    private var isBouncing: Bool = false
    private var bounceTarget: Float = 0

    private var rawOffset: Float = 0

    public init() {}

    public var maxOffset: Float {
        max(0, contentLength - viewportLength)
    }

    public func adopt(_ metrics: SFScrollMetrics) {
        switch axis {
        case .vertical:
            viewportLength = metrics.viewportHeight
            contentLength = metrics.contentHeight
        case .horizontal:
            viewportLength = metrics.viewportWidth
            contentLength = metrics.contentWidth
        }
    }

    public func beginDrag() {
        isDragging = true
        isSettling = false
        isBouncing = false
        velocity = 0
        rawOffset = offset
    }

    public func applyDrag(delta: Float) {
        rawOffset -= delta
        offset = Self.rubberBanded(
            rawOffset, min: 0, max: maxOffset, dimension: max(viewportLength, 1))
    }

    public func applyWheel(delta: Float) {
        isDragging = false
        isSettling = false
        isBouncing = false
        velocity = 0
        offset = min(max(offset + delta, 0), maxOffset)

        rawOffset = offset
    }

    public func scrollToTop() {
        guard offset > 0.5 else { return }
        isDragging = false
        isSettling = true
        isBouncing = true
        bounceTarget = 0
        velocity = 0
        rawOffset = 0
        NodeRegistry.shared.needsRender = true
    }

    @discardableResult
    public func step(dt: Float) -> Bool {
        guard !isDragging, isSettling else { return false }

        let low: Float = 0
        let high = maxOffset

        if !isBouncing {

            let decelerationRate: Float = 0.998
            velocity *= sfPow(decelerationRate, dt * 1000)
            offset -= velocity * dt

            if offset < low {
                isBouncing = true
                bounceTarget = low
            } else if offset > high {
                isBouncing = true
                bounceTarget = high
            }

            if !isBouncing {
                if abs(velocity) < 5 {
                    velocity = 0
                    isSettling = false
                    return false
                }
                return true
            }
        }

        let target = bounceTarget
        let x0 = offset - target

        let v0 = -velocity
        let omega: Float = 14.0

        let zeta: Float = 1.3

        let sqrtTerm = sfSqrt(zeta * zeta - 1.0)
        let r1 = omega * (-zeta + sqrtTerm)
        let r2 = omega * (-zeta - sqrtTerm)

        let A = (v0 - x0 * r2) / (r1 - r2)
        let B = x0 - A

        let e1 = sfExp(r1 * dt)
        let e2 = sfExp(r2 * dt)

        let newX = A * e1 + B * e2
        let newV = A * r1 * e1 + B * r2 * e2

        offset = target + newX

        velocity = -newV

        if abs(newX) < 0.5 && abs(newV) < 5 {
            offset = target
            velocity = 0
            isBouncing = false
            isSettling = false
            return false
        }
        return true
    }

    private static func rubberBanded(
        _ value: Float, min minBound: Float, max maxBound: Float, dimension d: Float
    ) -> Float {
        let c: Float = 0.55
        if value < minBound {
            return minBound - rubberBandDistance(minBound - value, dimension: d, constant: c)
        } else if value > maxBound {
            return maxBound + rubberBandDistance(value - maxBound, dimension: d, constant: c)
        }
        return value
    }

    private static func rubberBandDistance(_ x: Float, dimension d: Float, constant c: Float) -> Float {
        (1 - (1 / ((x * c / d) + 1))) * d
    }
}

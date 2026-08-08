struct SwipeActionPhysics {

    static let rubberBandRange: Float = 120

    static let openThreshold: Float = 0.5

    static let projectionTime: Float = 0.12

    static let fullSwipeThreshold: Float = 0.65

    static let response: Float = 0.32

    static func rubberBand(_ excess: Float, range: Float = rubberBandRange) -> Float {
        guard excess > 0, range > 0 else { return 0 }
        return (excess * range) / (range + excess)
    }

    static func offset(translation: Float, openWidth: Float, sign: Float) -> Float {

        let travel = translation * sign
        guard travel > 0 else { return 0 }
        if travel <= openWidth {
            return travel * sign
        }
        return (openWidth + rubberBand(travel - openWidth)) * sign
    }

    static func isFullSwipe(
        offset: Float,
        containerWidth: Float,
        hasDestructiveAction: Bool
    ) -> Bool {
        guard hasDestructiveAction, containerWidth > 0 else { return false }
        return abs(offset) >= containerWidth * fullSwipeThreshold
    }

    static func restingOffset(
        offset: Float,
        velocity: Float,
        openWidth: Float,
        sign: Float
    ) -> Float {
        guard openWidth > 0 else { return 0 }
        let projected = (offset + velocity * projectionTime) * sign
        return projected >= openWidth * openThreshold ? openWidth * sign : 0
    }
}

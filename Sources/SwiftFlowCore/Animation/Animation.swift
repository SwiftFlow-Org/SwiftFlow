import Foundation

/// The curve a change is animated along.
public struct Animation: Sendable {
    enum Kind {
        case linear(duration: Double)
        case easeIn(duration: Double)
        case easeOut(duration: Double)
        case easeInOut(duration: Double)
        case spring(response: Double, dampingFraction: Double)
    }

    let kind: Kind

    public static func linear(duration: Double = 0.3) -> Animation {
        Animation(kind: .linear(duration: duration))
    }
    public static func easeIn(duration: Double = 0.3) -> Animation {
        Animation(kind: .easeIn(duration: duration))
    }
    public static func easeOut(duration: Double = 0.3) -> Animation {
        Animation(kind: .easeOut(duration: duration))
    }
    public static func easeInOut(duration: Double = 0.3) -> Animation {
        Animation(kind: .easeInOut(duration: duration))
    }
    public static func spring(response: Double = 0.4, dampingFraction: Double = 0.8) -> Animation {
        Animation(kind: .spring(response: response, dampingFraction: dampingFraction))
    }

    public static let `default`: Animation = .easeInOut()
}

extension Animation.Kind {

    var duration: Double? {
        switch self {
        case .linear(let d), .easeIn(let d), .easeOut(let d), .easeInOut(let d): return d
        case .spring: return nil
        }
    }
}

extension Animation {

    func eased(_ t: Float) -> Float {
        switch kind {
        case .linear:
            return t
        case .easeIn:
            return t * t * t
        case .easeOut:
            let f = 1 - t
            return 1 - f * f * f
        case .easeInOut:
            return t < 0.5 ? 4 * t * t * t : 1 - sfPow(-2 * t + 2, 3) / 2
        case .spring:
            return t
        }
    }
}

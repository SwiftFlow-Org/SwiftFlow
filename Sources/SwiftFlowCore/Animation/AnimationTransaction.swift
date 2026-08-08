import Foundation

public enum AnimationTransaction {

    nonisolated(unsafe) static var latched: Animation?

    nonisolated(unsafe) static var ambient: Animation?

    static func beginBuild() {
        ambient = latched
        latched = nil
    }

    static func endBuild() {
        ambient = nil
        explicitlyAnimated = false
    }

    nonisolated(unsafe) private static var explicitlyAnimated = false

    static func markExplicitlyAnimated() {
        explicitlyAnimated = true
    }

    static func consumeExplicitlyAnimated() -> Bool {
        defer { explicitlyAnimated = false }
        return explicitlyAnimated
    }
}

@discardableResult
public func withAnimation<Result>(
    _ animation: Animation = .default,
    _ body: () throws -> Result
) rethrows -> Result {
    let previous = AnimationTransaction.latched
    AnimationTransaction.latched = animation
    defer {

        if AnimationTransaction.latched == nil {
            AnimationTransaction.latched = previous
        }
    }
    return try body()
}

@discardableResult
public func withoutAnimation<Result>(_ body: () throws -> Result) rethrows -> Result {
    let previous = AnimationTransaction.latched
    AnimationTransaction.latched = nil
    defer { AnimationTransaction.latched = previous }
    return try body()
}

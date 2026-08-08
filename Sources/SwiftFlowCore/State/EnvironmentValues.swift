import CSwiftFlow

/// A key for one value in the environment.
public protocol EnvironmentKey {
    associatedtype Value
    static var defaultValue: Value { get }
}

public struct EnvironmentValues {

    private var storage: [ObjectIdentifier: Any] = [:]

    public init() {}

    public subscript<K: EnvironmentKey>(key: K.Type) -> K.Value {
        get { storage[ObjectIdentifier(key)] as? K.Value ?? K.defaultValue }
        set { storage[ObjectIdentifier(key)] = newValue }
    }

    nonisolated(unsafe) static var current = EnvironmentValues()

    static func beginBuild() {
        current = EnvironmentValues()
    }
}

@propertyWrapper
/// Reads a value handed down by an ancestor.
public struct Environment<Value> {
    private let read: (EnvironmentValues) -> Value

    public init(_ keyPath: KeyPath<EnvironmentValues, Value>) {
        self.read = { $0[keyPath: keyPath] }
    }

    public var wrappedValue: Value {
        read(EnvironmentValues.current)
    }
}

public struct EnvironmentModifier<Content: View, Value>: View {
    public typealias Body = Never
    public var body: Never { fatalError() }

    let content: Content
    let keyPath: WritableKeyPath<EnvironmentValues, Value>
    let value: Value
}

extension EnvironmentModifier {
    public func toSFNode() -> SFNode {

        let saved = EnvironmentValues.current
        EnvironmentValues.current[keyPath: keyPath] = value

        defer { EnvironmentValues.current = saved }
        return content.toSFNode()
    }
}

extension View {

    /// Sets an environment value for this view and everything inside it.
    public func environment<Value>(
        _ keyPath: WritableKeyPath<EnvironmentValues, Value>,
        _ value: Value
    ) -> EnvironmentModifier<Self, Value> {
        EnvironmentModifier(content: self, keyPath: keyPath, value: value)
    }
}

public enum ColorScheme: Sendable, Equatable {
    case light
    case dark
}

private struct ColorSchemeKey: EnvironmentKey {

    static let defaultValue = ColorScheme.dark
}

extension EnvironmentValues {
    public var colorScheme: ColorScheme {
        get { self[ColorSchemeKey.self] }
        set { self[ColorSchemeKey.self] = newValue }
    }
}

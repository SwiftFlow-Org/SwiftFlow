/// A class whose changes can rebuild the views that read it. Apply the
/// `@Observable` macro rather than conforming by hand.
public protocol Observable: AnyObject {}

public final class ObservationRegistrar {
    public init() {}

    @inlinable
    public func access() {}

    @inlinable
    public func didMutate() {
        NodeRegistry.shared.needsRender = true
    }
}

extension ObservationRegistrar {

    public static func invalidate() {
        NodeRegistry.shared.needsRender = true
    }
}

@propertyWrapper
/// A reference type this view reads, rebuilt when it changes.
///
/// The one spelling for reference state — there is no `@StateObject` or
/// `@ObservedObject` here.
public struct Observed<Value> {
    private var value: Value

    public init(wrappedValue: Value) {
        self.value = wrappedValue
    }

    public var wrappedValue: Value {
        get { value }
        set {
            value = newValue
            NodeRegistry.shared.needsRender = true
        }
    }

}

extension Observable {

    public func binding<Value>(
        to keyPath: ReferenceWritableKeyPath<Self, Value>
    ) -> Binding<Value> {
        Binding(
            get: { self[keyPath: keyPath] },
            set: { self[keyPath: keyPath] = $0 }
        )
    }
}

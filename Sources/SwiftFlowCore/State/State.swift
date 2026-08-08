@propertyWrapper
/// A two-way handle on a value someone else owns.
public struct Binding<Value> {
    let get: () -> Value
    let set: (Value) -> Void

    public var wrappedValue: Value {
        get { get() }
        nonmutating set { set(newValue) }
    }

    public var projectedValue: Binding<Value> { self }

    public init(get: @escaping () -> Value, set: @escaping (Value) -> Void) {
        self.get = get
        self.set = set
    }
}

@propertyWrapper
/// A value owned by this view that survives the per-frame rebuild.
public struct State<Value> {
    private let id: UInt32

    public init(
        wrappedValue : Value,
        fileID       : String = #fileID,
        line         : Int    = #line,
        column       : Int    = #column
    ) {
        self.id = fnv1a("\(fileID):\(line):\(column)")

        let key = self.id
        if NodeRegistry.shared.stateValues[key] == nil {
            NodeRegistry.shared.stateValues[key] = wrappedValue
        }
    }

    public var wrappedValue: Value {
        get {
            NodeRegistry.shared.stateValues[id] as! Value
        }
        nonmutating set {
            NodeRegistry.shared.stateValues[id] = newValue
            NodeRegistry.shared.markDirty(id)
        }
    }

    public var projectedValue: Binding<Value> {
        Binding(
            get: { self.wrappedValue },
            set: { self.wrappedValue = $0 }
        )
    }
}

import CSwiftFlow

/// A piece of a user interface.
///
/// Describe one by composing other views in `body`. The whole tree is
/// rebuilt every frame, so a view is a value rather than an object — keep
/// anything that has to survive that in `@State` or an `@Observable`.
public protocol View {
    associatedtype Body: View
    @ViewBuilder var body: Self.Body { get }

    func toSFNode() -> SFNode
}

public protocol RecursiveView: View where Body: View {}
public protocol PrimitiveView: View where Body == Never {}

extension Never: View {
    public typealias Body = Never
    public var body: Never { fatalError() }
    public func toSFNode() -> SFNode {
        var node = SFNode.makeDefault()
        node.kind = SF_NODE_EMPTY
        return node
    }
}

extension PrimitiveView {
    public var body: Swift.Never {
        fatalError()
    }
}

extension View {
    public func toSFNode() -> SFNode {

        func open<V: View>(_ v: V) -> SFNode { v.toSFNode() }
        return open(body)
    }
}

extension PrimitiveView {
    public func toSFNode() -> SFNode {
        SFLog.debug("PrimitiveView default hit — missing toSFNode on \(String(describing: Self.self))")
        var node = SFNode.makeDefault()
        node.kind = SF_NODE_EMPTY
        return node
    }
}

extension SFNode {

    static func makeDefault() -> SFNode {
        var node = SFNode()
        node.scale = SFScale(x: 1, y: 1)
        return node
    }

    /// Both axes at once. Reads the horizontal one, writes both.
    public var sizing: SFSizing {
        get { sizingX }
        set {
            sizingX = newValue
            sizingY = newValue
        }
    }
}

extension SFSizing {

    /// What a wrapper takes on an axis nothing was said about: keep filling if
    /// the content fills, hug it otherwise.
    public var inherited: SFSizing {
        self == SF_SIZING_FILL ? SF_SIZING_FILL : SF_SIZING_HUG
    }
}

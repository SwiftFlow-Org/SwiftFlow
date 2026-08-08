import CSwiftFlow

final class RetainedNode {

    private var node: SFNode
    private var text: [UInt8]?
    private var children: [RetainedNode]

    private init(node: SFNode, text: [UInt8]?, children: [RetainedNode]) {
        self.node = node
        self.text = text
        self.children = children
    }

    static func capture(_ node: SFNode) -> RetainedNode {
        var copy = node

        var text: [UInt8]?
        if let pointer = node.text, node.textLen > 0 {
            text = Array(UnsafeBufferPointer(start: pointer, count: Int(node.textLen)))
        }

        copy.text = nil
        copy.textLen = 0

        var children: [RetainedNode] = []
        if let base = node.children, node.childrenLen > 0 {
            let buffer = UnsafeBufferPointer(start: base, count: Int(node.childrenLen))
            children = buffer.map { capture($0) }
        }
        copy.children = nil
        copy.childrenLen = 0

        return RetainedNode(node: copy, text: text, children: children)
    }

    func emit(applying snapshot: AnimatableSnapshot?) -> SFNode {
        var out = node

        if let text {

            FrameArena.shared.store(String(decoding: text, as: UTF8.self)) { pointer, length in
                out.text = pointer
                out.textLen = Int(length)
            }
        }

        if !children.isEmpty {
            var built = children.map { $0.emit(applying: nil) }
            let count = built.count
            FrameArena.shared.storeNodes(&built) { pointer in
                out.children = pointer
                out.childrenLen = count
            }
        }

        snapshot?.apply(to: &out)

        out.node_id = 0
        return out
    }
}

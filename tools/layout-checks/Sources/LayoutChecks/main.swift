import CSwiftFlow
import Foundation
import SwiftFlow

// ── Harness ──────────────────────────────────────────────────────────────

/// A node as laid out, with its frame on screen.
struct Laid {
    let kind: SFNodeKind
    let id: UInt32
    let frame: SFRect
    let text: String?
    let fill: SFColor
}

let window = (width: Float(400), height: Float(400))

/// Builds `view`, lays it out in the window and returns the root with
/// every node flattened, frames on screen.
func layOut<V: View>(_ view: V) -> (SFNode, [Laid]) {
    FrameArena.shared.reset()
    var root = NodeBuilder.build(view)
    sf_layout_tree(&root, window.width, window.height, 1)
    var out: [Laid] = []
    func walk(_ n: SFNode, _ ox: Float, _ oy: Float) {
        let x = ox + n.frame.x + n.offsetX, y = oy + n.frame.y + n.offsetY
        var text: String?
        if let t = n.text, n.textLen > 0 {
            text = String(decoding: UnsafeBufferPointer(start: t, count: Int(n.textLen)), as: UTF8.self)
        }
        out.append(Laid(kind: n.kind, id: n.node_id, frame: SFRect(x: x, y: y, width: n.frame.width, height: n.frame.height), text: text, fill: n.fill))
        if let kids = n.children {
            for i in 0..<Int(n.childrenLen) { walk(kids[i], x, y) }
        }
    }
    walk(root, 0, 0)
    return (root, out)
}

func text(_ s: String, in nodes: [Laid]) -> SFRect? {
    nodes.first { $0.text == s }?.frame
}

/// Presses and lets go at (`x`, `y`), as the desktop host does.
func tap(_ root: inout SFNode, _ x: Float, _ y: Float) {
    let path = NodeBuilder.hitPath(&root, x: x, y: y)
    GestureRouter.shared.pointerDown(x: x, y: y, t: 0, path: path, scrolls: [])
    GestureRouter.shared.pointerUp(x: x, y: y, t: 0.05)
}

nonisolated(unsafe) var failures = 0
func check(_ name: String, _ ok: Bool, _ detail: @autoclosure () -> String = "") {
    print(ok ? "ok    " : "FAIL  ", name, ok ? "" : "— \(detail())")
    if !ok { failures += 1 }
}

// ── Views with several children where one was expected ─────────────────

struct Rows: View {
    var body: some View {
        VStack(spacing: 0) {
            ForEach([1, 2], id: \.self) { i in
                Text("a\(i)")
                Text("b\(i)")
            }
        }
    }
}

do {
    let (_, n) = layOut(Rows())
    let f = ["a1", "b1", "a2", "b2"].map { text($0, in: n) }
    check("a ForEach row of two views shows both",
          f.allSatisfy { ($0?.height ?? 0) > 0 }, "\(f)")
    if f.allSatisfy({ $0 != nil }) {
        check("…stacked in order", f[0]!.y < f[1]!.y && f[1]!.y < f[2]!.y && f[2]!.y < f[3]!.y, "\(f)")
    }
}

struct Branch: View {
    let flag: Bool
    var body: some View {
        HStack(spacing: 10) {
            Text("x")
            if flag {
                Text("y")
                Text("z")
            }
            Text("w")
        }
    }
}

do {
    let (_, n) = layOut(Branch(flag: true))
    let x = text("x", in: n), y = text("y", in: n), z = text("z", in: n), w = text("w", in: n)
    check("an if branch of two views shows both", (y?.width ?? 0) > 0 && (z?.width ?? 0) > 0, "\(String(describing: y)) \(String(describing: z))")
    if let x, let y, let z, let w {
        check("…laid out in the row", x.x < y.x && y.x < z.x && z.x < w.x, "\(x.x) \(y.x) \(z.x) \(w.x)")
    }
    let (_, m) = layOut(Branch(flag: false))
    if let x = text("x", in: m), let w = text("w", in: m) {
        check("an empty if takes no gap in a row", abs(w.x - (x.x + x.width) - 10) < 0.5, "gap \(w.x - (x.x + x.width))")
    }
}

struct Chain: View {
    let mode: Int
    var body: some View {
        VStack(spacing: 0) {
            if mode == 0 {
                Text("p")
            } else if mode == 1 {
                Text("q")
            } else {
                Text("r")
                Text("s")
            }
        }
    }
}

do {
    for (mode, names) in [(0, ["p"]), (1, ["q"]), (2, ["r", "s"])] {
        let (_, n) = layOut(Chain(mode: mode))
        check("if / else if / else, case \(mode), shows its views",
              names.allSatisfy { (text($0, in: n)?.height ?? 0) > 0 }, "\(names.map { text($0, in: n) as Any })")
    }
}

// ── Pressing ─────────────────────────────────────────────────────────────

final class Box<T> { var value: T; init(_ v: T) { value = v } }
nonisolated(unsafe) let tapped = Box(false)

struct GapButton: View {
    var body: some View {
        VStack {
            Button(action: { tapped.value = true }) {
                VStack(spacing: 30) {
                    Text("top")
                    Text("bottom")
                }
            }
            .buttonStyle(PlainStyle())
            Spacer()
        }
    }
}

struct PlainStyle: ButtonStyle {
    func makeBody(configuration: ButtonStyleConfiguration) -> some View { configuration.label }
}

do {
    var (root, n) = layOut(GapButton())
    if let top = text("top", in: n), let bottom = text("bottom", in: n) {
        let gapY = (top.y + top.height + bottom.y) / 2
        tapped.value = false
        tap(&root, top.x + 2, gapY)
        check("a press between a button's label views taps it", tapped.value, "at y \(gapY)")
    } else {
        check("a press between a button's label views taps it", false, "labels missing")
    }
}

// ── Sizing ───────────────────────────────────────────────────────────────

struct SpacerRow: View {
    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            HStack {
                Text("row")
                Spacer()
            }
            Text("next")
            Spacer()
        }
    }
}

do {
    let (_, n) = layOut(SpacerRow())
    if let row = text("row", in: n), let next = text("next", in: n) {
        check("a Spacer in a row doesn't make it fill downwards", next.y < row.y + row.height + 2, "row at \(row.y), next at \(next.y)")
    }
}

struct ShapesBehind: View {
    var body: some View {
        VStack(spacing: 0) {
            Text("card")
                .background(ZStack {
                    RoundedRectangle(cornerRadius: 4).fill(Color.red)
                    RoundedRectangle(cornerRadius: 4).stroke(Color.blue)
                })
            Text("after")
            Spacer()
        }
    }
}

do {
    let (_, n) = layOut(ShapesBehind())
    if let card = text("card", in: n), let after = text("after", in: n) {
        check("a background stack of shapes doesn't make its view fill", after.y < card.y + card.height + 2, "card at \(card.y), after at \(after.y)")
    }
}

struct BigBehind: View {
    var body: some View {
        VStack(spacing: 0) {
            Text("small")
                .background(RoundedRectangle(cornerRadius: 0).fill(Color.red).frame(width: 200, height: 200))
            Text("after")
            Spacer()
        }
    }
}

do {
    let (_, n) = layOut(BigBehind())
    if let small = text("small", in: n), let after = text("after", in: n) {
        check("a background bigger than its view doesn't make it bigger", after.y < small.y + small.height + 2, "small at \(small.y), after at \(after.y)")
    }
}

struct FilledFrame: View {
    var body: some View {
        VStack {
            ZStack {
                RoundedRectangle(cornerRadius: 2).fill(Color(r: 1, g: 0, b: 0.5, a: 1))
                Text("i")
            }
            .frame(width: 26, height: 26)
            Spacer()
        }
    }
}

do {
    let (_, n) = layOut(FilledFrame())
    let shape = n.first { $0.kind == SF_NODE_RECT && $0.fill.b == 0.5 }
    check("a shape in a framed ZStack fills the frame",
          shape.map { abs($0.frame.width - 26) < 0.5 && abs($0.frame.height - 26) < 0.5 } ?? false,
          "\(String(describing: shape?.frame))")
}

// ── Text ─────────────────────────────────────────────────────────────────

struct SnugText: View {
    var body: some View {
        VStack {
            // 10.90 pt of text in an 11 pt box.
            Text("1s").font(Font(size: 11, weight: .bold)).frame(width: 11)
            Spacer()
        }
    }
}

struct BiggerFirst: View {
    var body: some View {
        Text("1s").font(Font(size: 12, weight: .bold))
    }
}

do {
    // The same letters at 12 pt first, so a cache shared between sizes
    // would hand the 11 pt text the 12 pt widths.
    _ = layOut(BiggerFirst())
    let (_, n) = layOut(SnugText())
    let t = text("1s", in: n)
    check("text in a box as wide as it doesn't wrap", (t?.height ?? 99) < 18, "\(String(describing: t))")
}

struct SnugTextInGlass: View {
    var body: some View {
        HStack(spacing: 10) {
            Button(action: {}) {
                ZStack {
                    Text("1s").font(Font(size: 11, weight: .bold)).frame(width: 11, height: 15)
                }
                .frame(width: 44, height: 44)
                .glassEffect(Glass(blurRadius: 0.4, tint: Color(r: 0, g: 0, b: 0, a: 1), refraction: 8, clarity: 1), in: Circle())
            }
            .buttonStyle(PlainStyle())
            Spacer()
        }
    }
}

do {
    let (_, n) = layOut(SnugTextInGlass())
    let t = text("1s", in: n)
    check("…nor in a glass button", (t?.height ?? 99) < 18, "\(String(describing: t))")
}

// ── Measuring ────────────────────────────────────────────────────────────

struct MeasuredButton: View {
    var body: some View {
        VStack {
            Button(action: {}) { Text("m") }
                .buttonStyle(PlainStyle())
                .measured(as: 0xABCD_0001)
            Spacer()
        }
    }
}

do {
    var (root, _) = layOut(MeasuredButton())
    let f = sf_get_node_frame(&root, 0xABCD_0001)
    check("measured(as:) on a button measures it", f.width > 0, "\(f)")
}

// ── Matched geometry ───────────────────────────────────────────────────

let checksSpace = GeometryNamespace("checks")
nonisolated(unsafe) var matchedMoved = false

struct MatchedSmall: View {
    var body: some View {
        RoundedRectangle(cornerRadius: 0).fill(Color(hex: 0xFF0000)).frame(width: 40, height: 40)
            .matchedGeometryEffect(id: "box", in: checksSpace)
            .padding(EdgeInsets(top: 10, bottom: 0, leading: 10, trailing: 0))
    }
}

struct MatchedLarge: View {
    var body: some View {
        RoundedRectangle(cornerRadius: 0).fill(Color(hex: 0xFF0000)).frame(width: 100, height: 80)
            .matchedGeometryEffect(id: "box", in: checksSpace)
            .padding(EdgeInsets(top: 200, bottom: 0, leading: 200, trailing: 0))
    }
}

/// The ZStack hugs its view and sits in the middle of the 400 × 400 frame:
/// the small box's centre is at (205, 205), the large one's at (300, 300).
struct Matched: View {
    var body: some View {
        ZStack(alignment: .topLeading) {
            // Two places, not two branches of one if: one place holds one view.
            if !matchedMoved { MatchedSmall() }
            if matchedMoved { MatchedLarge() }
        }
        .frame(width: 400, height: 400)
    }
}

/// Builds and lays out as a host does, keeping the frames asked for.
func hostFrame<V: View>(_ view: V) -> [Laid] {
    NodeFrames.shared.beginFrame()
    var (root, laid) = layOut(view)
    for id in NodeFrames.shared.tracked {
        NodeFrames.shared.record(id, frame: sf_get_node_frame(&root, id))
    }
    return laid
}

do {
    matchedMoved = false
    _ = hostFrame(Matched())
    _ = hostFrame(Matched())
    matchedMoved = true
    _ = hostFrame(Matched())
    var root = NodeBuilder.build(Matched())
    sf_layout_tree(&root, window.width, window.height, 1)
    // Offset and scale sit on the box or a node above it; add them up.
    func drawn(_ n: SFNode, _ ox: Float, _ oy: Float, _ sx: Float, _ sy: Float, _ dx: Float, _ dy: Float) -> (Float, Float, Float, Float)? {
        let x = ox + n.frame.x, y = oy + n.frame.y
        let ddx = dx + n.offsetX, ddy = dy + n.offsetY
        let ssx = sx * n.scale.x, ssy = sy * n.scale.y
        if n.fill.r == 1 && n.fill.g == 0 && n.frame.width > 0 {
            return (x + n.frame.width / 2 + ddx, y + n.frame.height / 2 + ddy, n.frame.width * ssx, n.frame.height * ssy)
        }
        if let kids = n.children {
            for i in 0..<Int(n.childrenLen) {
                if let f = drawn(kids[i], x, y, ssx, ssy, ddx, ddy) { return f }
            }
        }
        return nil
    }
    if let b = drawn(root, 0, 0, 1, 1, 0, 0) {
        check("a matched view starts where the last one with its id was",
              abs(b.0 - 205) < 3 && abs(b.1 - 205) < 3 && abs(b.2 - 40) < 3 && abs(b.3 - 40) < 3, "\(b)")
    } else {
        check("a matched view starts where the last one with its id was", false, "no box")
    }
    Thread.sleep(forTimeInterval: 1.5)
    _ = hostFrame(Matched())
    root = NodeBuilder.build(Matched())
    root = NodeBuilder.build(Matched())
    sf_layout_tree(&root, window.width, window.height, 1)
    if let b = drawn(root, 0, 0, 1, 1, 0, 0) {
        check("…and ends at its own frame", abs(b.0 - 300) < 1 && abs(b.1 - 300) < 1 && abs(b.2 - 100) < 1 && abs(b.3 - 80) < 1, "\(b)")
    }
}

print(failures == 0 ? "all passed" : "\(failures) failed")
exit(failures == 0 ? 0 : 1)

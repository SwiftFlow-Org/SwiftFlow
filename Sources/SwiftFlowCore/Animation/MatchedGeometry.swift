import CSwiftFlow
import Foundation

/// A set of matched-geometry ids, so two screens can each use "card"
/// without their views flying into each other.
public struct GeometryNamespace: Hashable, Sendable {
    let name: String

    public init(_ name: String = #fileID) {
        self.name = name
    }
}

/// Moves a view from where another view with the same id was to where it
/// is, as SwiftUI's `matchedGeometryEffect`.
///
/// When a view marked with an id appears in a different place in the tree
/// from the view that had it in the last build (one screen's thumbnail replaced by the
/// next screen's picture, say), it starts at that view's last frame,
/// position and size, and springs to its own. Frames are measured by
/// layout, after a build, so on the build where it appears the new view
/// has no frame yet and is left out; it shows from the next one.
///
/// The move is drawn with offset and scale about the view's centre, so it
/// does not disturb the layout around it.
///
/// Views are told apart by their place in the tree, so the two views must
/// sit in different places: two `if`s, or two screens, not the two branches
/// of one `if`/`else`, which share a place and count as one view.
public struct MatchedGeometryEffect<Content: View>: View {
    public typealias Body = Never
    public var body: Never { fatalError() }

    let content: Content
    let key: String
    let response: Double
    let dampingFraction: Double

    public func toSFNode() -> SFNode {
        let holder = BuildContext.shared.currentID(for: self)
        let measureID = fnv1a("matched:\(key)#\(holder)")
        let registry = MatchedGeometry.shared
        registry.saw(key, holder: holder, measureID: measureID, response: response, dampingFraction: dampingFraction)

        var node = content.toSFNode()
        NodeFrames.shared.register(measureID)
        if let (dx, dy, sx, sy) = registry.displacement(key, measureID: measureID) {
            node.offsetX += dx
            node.offsetY += dy
            node.scale.x *= sx
            node.scale.y *= sy
        }
        // Measured on a wrapper of its own, as `MeasuredNode` does, so the
        // effect's offset and scale don't feed back into the frame it
        // measures.
        var measured = Self.wrap(node)
        measured.node_id = measureID
        return Self.wrap(measured)
    }

    /// A depth stack around `node` that lays out as `node` does.
    private static func wrap(_ node: SFNode) -> SFNode {
        var wrapper = SFNode.makeDefault()
        wrapper.kind = SF_NODE_STACK
        wrapper.axis = SF_AXIS_DEPTH
        wrapper.alignment = SF_ALIGNMENT_LEADING
        wrapper.verticalAlignment = SF_ALIGNMENT_LEADING
        wrapper.sizingX = node.sizingX
        wrapper.sizingY = node.sizingY
        wrapper.fixedWidth = node.fixedWidth
        wrapper.fixedHeight = node.fixedHeight
        var children = [node]
        FrameArena.shared.storeNodes(&children) { pointer in
            wrapper.children = pointer
            wrapper.childrenLen = 1
        }
        return wrapper
    }
}

/// What the matched-geometry views were last seen as, and the moves in
/// progress.
public final class MatchedGeometry {
    nonisolated(unsafe) public static let shared = MatchedGeometry()

    private struct Seen {
        var holder: UInt32
        var measureID: UInt32
    }

    private struct Flight {
        let from: SFRect
        let started: TimeInterval
        let response: Double
        let dampingFraction: Double
        let measureID: UInt32
    }

    private var seen: [String: Seen] = [:]
    private var seenThisBuild: Set<String> = []
    private var flights: [String: Flight] = [:]

    private var now: TimeInterval { ProcessInfo.processInfo.systemUptime }

    /// Forgets the ids no view had in the last build. A move happens only
    /// when one view takes another's place from one build to the next, not
    /// when a view with the id comes back after a time without one.
    func beginBuild() {
        for key in seen.keys where !seenThisBuild.contains(key) {
            seen.removeValue(forKey: key)
            flights.removeValue(forKey: key)
        }
        seenThisBuild.removeAll(keepingCapacity: true)
    }

    /// Notes that `key` is shown by `holder` this build, starting a move
    /// from the last holder's frame when the holder changed.
    fileprivate func saw(_ key: String, holder: UInt32, measureID: UInt32, response: Double, dampingFraction: Double) {
        if let last = seen[key], last.holder != holder,
           let from = NodeFrames.shared.frame(for: last.measureID), from.width > 0, from.height > 0 {
            flights[key] = Flight(from: from, started: now, response: response, dampingFraction: dampingFraction, measureID: measureID)
        }
        seen[key] = Seen(holder: holder, measureID: measureID)
        seenThisBuild.insert(key)
    }

    /// The offset (pixels) and scale that put the view at the point of its
    /// move it has reached, or nil when it isn't moving.
    fileprivate func displacement(_ key: String, measureID: UInt32) -> (Float, Float, Float, Float)? {
        guard let flight = flights[key], flight.measureID == measureID else { return nil }
        NodeRegistry.shared.needsRender = true
        guard let to = NodeFrames.shared.frame(for: measureID), to.width > 0, to.height > 0 else {
            // Not laid out yet, so there is no frame to move to: shrink
            // it out of sight until the next build knows one.
            return (0, 0, 0.0001, 0.0001)
        }
        let t = Float(progress(of: flight))
        if t >= 1 {
            flights.removeValue(forKey: key)
            return nil
        }
        let left = 1 - t
        let dx = ((flight.from.x + flight.from.width / 2) - (to.x + to.width / 2)) * left
        let dy = ((flight.from.y + flight.from.height / 2) - (to.y + to.height / 2)) * left
        let sx = 1 + (flight.from.width / to.width - 1) * left
        let sy = 1 + (flight.from.height / to.height - 1) * left
        return (dx, dy, sx, sy)
    }

    /// Where the view with `id` is drawn now, in points: partway along its
    /// move while it moves, its laid-out frame otherwise, or nil when no
    /// view has the id. Read after that view is built in the same build
    /// (place the reader after it) to see a move that starts this build.
    /// For drawing something outside SwiftFlow, a GPU canvas say, along
    /// with it.
    public func frame(id: String, in namespace: GeometryNamespace) -> SFRect? {
        let key = MatchedGeometry.key(id, namespace)
        guard let holder = seen[key] else { return nil }
        let to = NodeFrames.shared.frame(for: holder.measureID)
        guard let flight = flights[key], flight.measureID == holder.measureID else {
            return to?.inPoints()
        }
        guard let to, to.width > 0, to.height > 0 else { return flight.from.inPoints() }
        let t = Float(min(progress(of: flight), 1))
        let from = flight.from
        return SFRect(
            x: from.x + (to.x - from.x) * t,
            y: from.y + (to.y - from.y) * t,
            width: from.width + (to.width - from.width) * t,
            height: from.height + (to.height - from.height) * t
        ).inPoints()
    }

    /// How far `key`'s move has got, 0 at the start and 1 when it has
    /// settled, or nil when it isn't moving. A spring can pass 1 on the way.
    /// For fading other things in time with it.
    public func progress(id: String, in namespace: GeometryNamespace) -> Double? {
        flights[MatchedGeometry.key(id, namespace)].map(progress(of:))
    }

    private func progress(of flight: Flight) -> Double {
        Self.spring(elapsed: now - flight.started, response: flight.response, dampingFraction: flight.dampingFraction)
    }

    /// How far a spring from 0 to 1 has got after `elapsed` seconds: the
    /// curve the moves follow, for timing other things to match them.
    /// Exactly 1 once it has settled.
    public static func spring(elapsed t: Double, response: Double, dampingFraction: Double) -> Double {
        if t <= 0 { return 0 }
        let omega = 2 * Double.pi / max(response, 0.01)
        let zeta = min(max(dampingFraction, 0.05), 1)
        let decay = exp(-zeta * omega * t)
        let x: Double
        if zeta >= 0.999 {
            x = 1 - (1 + omega * t) * decay
        } else {
            let wd = omega * (1 - zeta * zeta).squareRoot()
            x = 1 - decay * (cos(wd * t) + zeta * omega / wd * sin(wd * t))
        }
        // Settled: close enough, and slow enough not to come back.
        if abs(1 - x) < 0.002, decay < 0.01 {
            return 1
        }
        return x
    }

    static func key(_ id: String, _ namespace: GeometryNamespace) -> String {
        "\(namespace.name)#\(id)"
    }
}

extension View {
    /// Moves this view from where the last view with the same `id` in
    /// `namespace` was, when it takes that view's place, springing over
    /// `response` seconds. See `MatchedGeometryEffect`.
    public func matchedGeometryEffect(
        id: String,
        in namespace: GeometryNamespace,
        response: Double = 0.5,
        dampingFraction: Double = 0.86
    ) -> MatchedGeometryEffect<Self> {
        MatchedGeometryEffect(
            content: self,
            key: MatchedGeometry.key(id, namespace),
            response: response,
            dampingFraction: dampingFraction
        )
    }
}

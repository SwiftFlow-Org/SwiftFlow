import CSwiftFlow

enum SliderMetrics {

    /// The row the slider occupies. Wider than the track on purpose: the
    /// whole height is draggable, so the finger does not have to find a 6pt
    /// line.
    static let height: Float = 30

    static let trackHeight: Float = 6
    static let heldTrackHeight: Float = 11

    static let knobWidth: Float = 38
    static let knobHeight: Float = 24
    static let heldKnobWidth: Float = 57
    static let heldKnobHeight: Float = 38

    static let tickSize: Float = 3

    /// Past this many steps the ticks stop being marks and start being noise,
    /// so they are dropped.
    static let maxTicks = 40

    static let grow: Animation = .spring(response: 0.28, dampingFraction: 0.82)

    static func trackID(_ id: UInt32) -> UInt32 { fnv1a("swiftflow.slider.track.\(id)") }
    static func fillID(_ id: UInt32) -> UInt32 { fnv1a("swiftflow.slider.fill.\(id)") }
    static func knobID(_ id: UInt32) -> UInt32 { fnv1a("swiftflow.slider.knob.\(id)") }
}

/// Which sliders have a finger on them.
///
/// The drag itself needs no state, since the value it writes is the caller's.
/// This is only what the track and knob grow against, and it is keyed by node
/// so two sliders on one screen don't grow together.
final class SliderRegistry {
    nonisolated(unsafe) static let shared = SliderRegistry()

    nonisolated(unsafe) private var held: Set<UInt32> = []

    func isHeld(_ id: UInt32) -> Bool { held.contains(id) }

    func begin(_ id: UInt32) {
        held.insert(id)
        NodeRegistry.shared.needsRender = true
    }

    func end(_ id: UInt32) {
        held.remove(id)
        NodeRegistry.shared.needsRender = true
    }
}

/// A value picked by dragging along a track.
///
/// ```swift
/// @State private var volume: Float = 0.4
/// @State private var rating: Float = 3
///
/// Slider(value: $volume)
/// Slider(value: $rating, in: 1...5, step: 1)     // snaps, and shows ticks
/// ```
///
/// The drag starts anywhere on the row, and the value follows the finger
/// rather than the distance travelled, so a press at the far end jumps
/// straight there. Holding it thickens the track and grows the knob.
public struct Slider: View {
    public typealias Body = Never
    public var body: Never { fatalError() }

    let value: Binding<Float>
    let bounds: ClosedRange<Float>
    let step: Float?

    public init(
        value: Binding<Float>,
        in bounds: ClosedRange<Float> = 0...1,
        step: Float? = nil
    ) {
        self.value = value
        self.bounds = bounds
        self.step = step
    }
}

extension Slider {
    public func toSFNode() -> SFNode {
        let id = BuildContext.shared.currentID(for: self)
        let held = SliderRegistry.shared.isHeld(id)

        // A frame behind, which is what the renderer can offer. The track is a
        // fixed row, so the only build that gets this wrong is the first one.
        let width = NodeFrames.shared.frame(for: id)?.inPoints().width ?? 0

        return track(id: id, held: held, width: width).toSFNode()
    }

    private func track(id: UInt32, held: Bool, width: Float) -> some View {
        let knobWidth = held ? SliderMetrics.heldKnobWidth : SliderMetrics.knobWidth
        let knobHeight = held ? SliderMetrics.heldKnobHeight : SliderMetrics.knobHeight
        let thickness = held ? SliderMetrics.heldTrackHeight : SliderMetrics.trackHeight

        let travel = max(width - knobWidth, 0)
        let knobX = fraction * travel
        let knobY: Float = held ? -6 : 0
        let fillWidth = knobX + knobWidth / 2

        return ZStack(alignment: .leading) {
            Capsule()
                .fill(.fill)
                .frame(height: SliderMetrics.trackHeight)
                .animation(SliderMetrics.grow, id: SliderMetrics.trackID(id))

            // The height animates on the inner node and the width is set on
            // the outer one, so the fill can follow the finger exactly while
            // its thickness still springs.
            Capsule()
                .fill(.accent)
                .frame(height: SliderMetrics.trackHeight)
                .animation(SliderMetrics.grow, id: SliderMetrics.fillID(id))
                .frame(width: fillWidth)

            ticks(travel: travel, knob: knobWidth)

            Capsule()
                .fill(.primary)
                .frame(width: knobWidth, height: knobHeight)
                .offset(y: knobY)
                .animation(SliderMetrics.grow, id: SliderMetrics.knobID(id))
                .offset(x: knobX)
        }
        .expands()
        .frame(height: SliderMetrics.height, maxWidth: .infinity)
        // Claims the id the gesture then attaches to, so the drag and the
        // width it is measured against are the same node.
        .measured(as: id)
        .gesture(drag(id: id, width: width))
    }

    @ViewBuilder
    private func ticks(travel: Float, knob: Float) -> some View {
        if let marks = tickFractions {
            ForEach(marks.indices, id: \.self) { index in
                Capsule()
                    .fill(.background)
                    .frame(width: SliderMetrics.tickSize, height: SliderMetrics.tickSize)
                    .offset(
                        x: knob / 2 - SliderMetrics.tickSize / 2 + marks[index] * travel
                    )
            }
        }
    }

    private func drag(id: UInt32, width: Float) -> DragGesture {
        let binding = value

        // The id a drag reports under comes from where it was written, which
        // for every slider is this one line. Naming it after the node keeps
        // two sliders from writing over each other's gesture state.
        return DragGesture(
            axis: .horizontal,
            minimumDistance: 0,
            fileID: "swiftflow.slider.\(id)",
            line: 0,
            column: 0
        )
        .onChanged { drag in
            SliderRegistry.shared.begin(id)
            binding.wrappedValue = self.value(atRowX: drag.location.x, width: width)
        }
        .onEnded { drag in
            SliderRegistry.shared.end(id)
            binding.wrappedValue = self.value(atRowX: drag.location.x, width: width)
        }
    }

    /// Where the knob sits along its travel, 0 to 1.
    private var fraction: Float {
        let span = bounds.upperBound - bounds.lowerBound
        guard span > 0 else { return 0 }
        return min(max((value.wrappedValue - bounds.lowerBound) / span, 0), 1)
    }

    /// The value a finger at `x` is asking for. Measured against the knob's
    /// centre rather than the row's edge, or the last few points of travel
    /// would be unreachable.
    private func value(atRowX x: Float, width: Float) -> Float {
        let held = SliderMetrics.heldKnobWidth
        let travel = max(width - held, 1)
        let fraction = min(max((x - held / 2) / travel, 0), 1)
        let raw = bounds.lowerBound + fraction * (bounds.upperBound - bounds.lowerBound)
        return snapped(raw)
    }

    private func snapped(_ raw: Float) -> Float {
        guard let step, step > 0 else { return raw }
        let steps = ((raw - bounds.lowerBound) / step).rounded()
        return min(max(bounds.lowerBound + steps * step, bounds.lowerBound), bounds.upperBound)
    }

    /// Where the ticks go, as fractions of the travel. `nil` when there is no
    /// step, or when there are so many that they would read as a solid line.
    private var tickFractions: [Float]? {
        guard let step, step > 0 else { return nil }
        let span = bounds.upperBound - bounds.lowerBound
        guard span > 0 else { return nil }

        let count = Int((span / step).rounded())
        guard count > 0, count <= SliderMetrics.maxTicks else { return nil }

        return (0...count).map { Float($0) * step / span }
    }
}

import CSwiftFlow

enum ToggleMetrics {

    static let trackWidth: Float = 64
    static let trackHeight: Float = 28
    static let knobWidth: Float = 38
    static let knobHeight: Float = 24
    static let inset: Float = 2

    /// What the knob widens to while it is held down. It grows on the side it
    /// is about to travel to, which is the whole tell that it is a switch and
    /// not a light.
    static let heldKnobWidth: Float = 38

    /// The knob's travel. Enough damping to land without a wobble, quick
    /// enough that the switch feels flicked rather than driven.
    static let travel: Animation = .spring(response: 0.3, dampingFraction: 0.75)

    /// The track's colour follows on its own, a little behind the knob.
    static let tintDuration: Double = 0.18

    static func trackID(_ id: UInt32) -> UInt32 { fnv1a("swiftflow.toggle.track.\(id)") }
    static func knobID(_ id: UInt32) -> UInt32 { fnv1a("swiftflow.toggle.knob.\(id)") }
}

/// A switch, with an optional label beside it.
///
/// ```swift
/// @State private var wifi = true
///
/// Toggle("Wi-Fi", isOn: $wifi)
/// Toggle(isOn: $wifi)                       // the switch on its own
/// Toggle(isOn: $wifi) { Label() }           // any view as the label
/// ```
///
/// With a label the whole row is the tap target, so the switch does not have
/// to be hit exactly. Holding it stretches the knob toward the side it would
/// travel to, and letting go springs it across.
public struct Toggle<Label: View>: View {
    public typealias Body = Never
    public var body: Never { fatalError() }

    let isOn: Binding<Bool>
    let label: Label
    let hasLabel: Bool

    public init(isOn: Binding<Bool>, @ViewBuilder label: () -> Label) {
        self.isOn = isOn
        self.label = label()
        self.hasLabel = true
    }

    public init(_ title: String, isOn: Binding<Bool>) where Label == Text {
        self.isOn = isOn
        self.label = Text(title)
        self.hasLabel = true
    }

    public init(isOn: Binding<Bool>) where Label == EmptyView {
        self.isOn = isOn
        self.label = EmptyView()
        self.hasLabel = false
    }
}

extension Toggle {
    public func toSFNode() -> SFNode {
        let id = BuildContext.shared.currentID(for: self)
        let held = NodeRegistry.shared.pressedNodes.contains(id)

        var node = row(id: id, held: held).toSFNode()
        node.node_id = id

        let binding = isOn
        NodeRegistry.shared.registerTap(id) { binding.wrappedValue.toggle() }
        return node
    }

    @ViewBuilder
    private func row(id: UInt32, held: Bool) -> some View {
        if hasLabel {
            // `.weight` rather than a `Spacer`, which in a horizontal stack
            // takes the whole offered height and would leave the row as tall
            // as the screen. The row has to be told to fill for a weight to
            // have anything to divide.
            HStack {
                label.weight(1)
                sw(id: id, held: held)
            }
            .expands(.horizontal)
        } else {
            sw(id: id, held: held)
        }
    }

    private func sw(id: UInt32, held: Bool) -> some View {
        let on = isOn.wrappedValue
        let knobWidth = held ? ToggleMetrics.heldKnobWidth : ToggleMetrics.knobWidth
        let travel = ToggleMetrics.trackWidth - ToggleMetrics.inset * 2 - knobWidth

        return ZStack(alignment: .leading) {
            Capsule()
                .fill(on ? Color.accent : Color.overlay)
                .animation(
                    .easeOut(duration: ToggleMetrics.tintDuration),
                    id: ToggleMetrics.trackID(id)
                )

            // Width and offset animate together on the one node, which is what
            // makes the stretch read as the knob leaning into its travel
            // rather than as two separate moves.
            Capsule()
                .fill(.primary)
                .frame(width: knobWidth, height: ToggleMetrics.knobHeight)
                .offset(x: ToggleMetrics.inset + (on ? travel : 0))
                .animation(ToggleMetrics.travel, id: ToggleMetrics.knobID(id))
        }
        // On the stack itself: a hugging stack takes the size of whichever
        // child does not fill it, and here that is the knob.
        .expands()
        .frame(width: ToggleMetrics.trackWidth, height: ToggleMetrics.trackHeight)
    }
}

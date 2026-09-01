import CSwiftFlow

extension Transition {

    public static let sheet = Transition(Transition.Phase(offsetY: 420))

    public static let cover = Transition(Transition.Phase(scale: 1.06, opacity: 0))

    public static func morph(from source: SFRect, to destination: SFRect) -> Transition {

        guard destination.width > 0, destination.height > 0,
              source.width > 0, source.height > 0
        else { return .sheet }

        return Transition(
            Transition.Phase(
                scaleX: source.width / destination.width,
                scaleY: source.height / destination.height,

                cornerRadius: min(source.width, source.height) / 2
                    - PresentationMetrics.sheetCornerRadius,
                opacity: 0,
                offsetX: source.midX - destination.midX,
                offsetY: source.midY - destination.midY
            )
        )
    }
}

extension SFRect {
    var midX: Float { x + width / 2 }
    var midY: Float { y + height / 2 }

    func inPoints() -> SFRect {
        let scale = DeviceScale.current > 0 ? DeviceScale.current : 1
        return SFRect(
            x: x / scale, y: y / scale,
            width: width / scale, height: height / scale
        )
    }
}

enum PresentationMetrics {

    static let sheetTopInset: Float = 40
    static let sheetCornerRadius: Float = 28

    /// The strip along the sheet's top edge that a pull-down starts in, and
    /// the grabber drawn in it.
    static let grabberStrip: Float = 28
    static let grabberWidth: Float = 36
    static let grabberThickness: Float = 5
    static let grabberTopInset: Float = 8

    static let closeInset: Float = 10
    static let closeIconSize: Float = 15

    /// How far down a sheet has to be headed when it is let go to keep
    /// going rather than come back.
    static let dismissDistance: Float = 120

    /// A sheet is at the top of its travel: pulling up only stretches the
    /// gesture, it doesn't move the sheet.
    static let upwardResistance: Float = 0.15

    /// How far down the drag has to get for the scrim to be fully out of the
    /// way, and how much of it goes.
    static let scrimFadeDistance: Float = 400
    static let scrimFade: Float = 0.7

    /// What a sheet that isn't going anywhere comes back on.
    static let settleResponse: Float = 0.34
    static let settleDamping: Float = 0.86
}

/// How far the front sheet has been pulled down.
///
/// Only the front sheet can be dragged — nothing behind it is reachable — so
/// one of these covers the stack.
///
/// The spring back lives here rather than in `.animation(_:value:)` because
/// that modifier ignores a new target while an animation of its own is in
/// flight: the frame the finger arrives would start one, and every frame
/// after it would be dropped. A drag has to be exact under the finger and a
/// spring only after it lets go, which is two behaviours, not one curve.
final class SheetDrag {
    nonisolated(unsafe) static let shared = SheetDrag()

    private(set) var offset: Float = 0
    private var velocity: Float = 0
    private var settling = false

    var isAnimating: Bool { settling }

    func move(_ translation: Float) {
        settling = false
        velocity = 0
        offset = translation >= 0
            ? translation
            : translation * PresentationMetrics.upwardResistance
        NodeRegistry.shared.needsRender = true
    }

    /// Lets go, and says whether the sheet is going with it. The question is
    /// where the drag was *headed*, not where it stopped, so a short flick
    /// counts the same as a long pull.
    ///
    /// On a dismissal the offset is dropped: the departing sheet carries its
    /// own copy, taken from the last build.
    func release(headedFor travel: Float) -> Bool {
        NodeRegistry.shared.needsRender = true

        guard travel <= PresentationMetrics.dismissDistance else {
            offset = 0
            velocity = 0
            settling = false
            return true
        }
        settling = offset != 0
        return false
    }

    func step(dt: Float) {
        guard settling else { return }

        let (x, v) = scalarSpringStep(
            x0: offset, v0: velocity,
            omega: (2 * Float.pi) / PresentationMetrics.settleResponse,
            zeta: PresentationMetrics.settleDamping, dt: dt
        )
        offset = x
        velocity = v

        if abs(x) < 0.5 && abs(v) < 2 {
            offset = 0
            velocity = 0
            settling = false
        }
    }

    /// How far along the scrim is to being out of the way.
    var scrimProgress: Float {
        min(max(offset, 0) / PresentationMetrics.scrimFadeDistance, 1)
    }
}

struct ModalHost: View {
    typealias Body = Never
    var body: Never { fatalError() }

    let base: AnyView
    let stack: NavigationStackModel

    func toSFNode() -> SFNode {
        let host = ZStack {
            base
            NodeListView(nodes: ModalHost.layerNodes(for: stack))
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)

        return host.toSFNode()
    }

    static func layerNodes(for stack: NavigationStackModel) -> [SFNode] {
        let front = stack.modals.count - 1
        let layers = stack.modals.enumerated().map { depth, modal in
            ModalLayer(
                style: modal.style,
                content: AnyView(erasing: modal.view),
                morphSource: modal.morphSource,
                isFront: depth == front,
                showsCloseButton: modal.showsCloseButton,
                dismiss: { [stack] in stack.pop() }
            )
        }

        return buildChildren(
            ForEach(layers.indices, id: \.self) { index in
                layers[index]
            }
        )
    }
}

struct ModalLayer: View {
    let style: NavPresentation
    let content: AnyView

    let morphSource: UInt32

    /// Only the front sheet can be dragged; the ones behind it hold still.
    let isFront: Bool

    let showsCloseButton: Bool
    let dismiss: () -> Void

    private var morph: Transition? {
        guard morphSource != 0, Screen.isKnown else { return nil }
        guard let source = NodeFrames.shared.frame(for: morphSource) else { return nil }

        let top = SafeArea.top + PresentationMetrics.sheetTopInset
        let destination = SFRect(
            x: 0, y: top,
            width: Screen.width,
            height: max(0, Screen.height - top)
        )
        return .morph(from: source.inPoints(), to: destination)
    }

    var body: some View {
        switch style {
        case .push:

            content.transition(.identity)
        case .sheet:
            sheet.transition(morph ?? .sheet)
        case .cover:
            cover.transition(.cover)
        }
    }

    private var sheet: some View {
        ZStack(alignment: .bottom) {
            scrim
            // Trailing, so the close button's box stays the size of the
            // button. A full-width one would sit over the whole top edge and
            // take the grabber's presses with it — a hit test walks the
            // topmost child whose frame contains the point and stops there.
            ZStack(alignment: .topTrailing) {
                content
                    .padding(.bottom, SafeArea.bottom)
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
                grabber
                closeButton
            }
            // On the stack itself, not a box around it: a hugging stack
            // sizes to whatever child *doesn't* fill it, and the close button
            // is exactly that child. `.frame(maxWidth:)` would wrap a filling
            // box around a stack still hugging its own chrome.
            .expands()
            .background(
                RoundedRectangle(cornerRadius: PresentationMetrics.sheetCornerRadius)
                    .fill(.background)
                    .specular()
            )
            .clipShape(RoundedRectangle(cornerRadius: PresentationMetrics.sheetCornerRadius))
            .offset(y: dragOffset)
            .padding(.top, SafeArea.top + PresentationMetrics.sheetTopInset)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    private var cover: some View {
        ZStack(alignment: .topTrailing) {
            content
                .frame(maxWidth: .infinity, maxHeight: .infinity)
            closeButton
                .padding(.top, SafeArea.top)
        }
        .expands()
        .background(RoundedRectangle(cornerRadius: 0).fill(.background))
    }

    private var dragOffset: Float {
        isFront ? SheetDrag.shared.offset : 0
    }

    /// The strip a pull-down starts in, with the grabber drawn in it.
    ///
    /// It is the top of the sheet and not the whole of it on purpose: a drag
    /// gesture anywhere on the sheet would beat every vertical scroll inside
    /// it, since the router hands a press to the first gesture on its path
    /// that claims the axis.
    private var grabber: some View {
        Capsule()
            .fill(.tertiary)
            .frame(
                width: PresentationMetrics.grabberWidth,
                height: PresentationMetrics.grabberThickness
            )
            .padding(.top, PresentationMetrics.grabberTopInset)
            .frame(height: PresentationMetrics.grabberStrip, maxWidth: .infinity, alignment: .top)
            .gesture(pullDown)
    }

    private var pullDown: DragGesture {
        DragGesture(axis: .vertical, minimumDistance: 4)
            .onChanged { value in
                SheetDrag.shared.move(value.translation.y)
            }
            .onEnded { value in
                if SheetDrag.shared.release(headedFor: value.predictedEndTranslation.y) {
                    self.dismiss()
                }
            }
    }

    @ViewBuilder
    private var closeButton: some View {
        if showsCloseButton {
            Button(action: dismiss) {
                Icon.x
                    .size(PresentationMetrics.closeIconSize)
                    .foregroundColor(.primary)
            }
            .buttonStyle(NavBarButtonStyle())
            .padding(.top, PresentationMetrics.closeInset)
            .padding(.trailing, PresentationMetrics.closeInset)
        }
    }

    private var scrim: some View {
        RoundedRectangle(cornerRadius: 0)
            .fill(Color.scrim.opacity(1 - SheetDrag.shared.scrimProgress * PresentationMetrics.scrimFade))
            .frame(maxWidth: .infinity, maxHeight: .infinity)
            .onTap { _ in self.dismiss() }
    }
}

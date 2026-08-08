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
        let layers = stack.modals.map { modal in
            ModalLayer(
                style: modal.style,
                content: AnyView(erasing: modal.view),
                morphSource: modal.morphSource,
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
            content
                .padding(.bottom, SafeArea.bottom)
                .frame(maxWidth: .infinity, maxHeight: .infinity)
                .background(
                    RoundedRectangle(cornerRadius: PresentationMetrics.sheetCornerRadius)
                        .fill(.background)
                        .specular()
                )
                .clipShape(RoundedRectangle(cornerRadius: PresentationMetrics.sheetCornerRadius))
                .padding(.top, SafeArea.top + PresentationMetrics.sheetTopInset)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    private var cover: some View {
        content
            .frame(maxWidth: .infinity, maxHeight: .infinity)
            .background(RoundedRectangle(cornerRadius: 0).fill(.background))
    }

    private var scrim: some View {
        RoundedRectangle(cornerRadius: 0)
            .fill(.scrim)
            .frame(maxWidth: .infinity, maxHeight: .infinity)
            .onTap { _ in self.dismiss() }
    }
}

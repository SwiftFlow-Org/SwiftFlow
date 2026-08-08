import CSwiftFlow

public enum ContentMode: Sendable {

    case fit

    case fill

    case stretch

    var sfValue: SFContentMode {
        switch self {
        case .fit: return SF_CONTENT_FIT
        case .fill: return SF_CONTENT_FILL
        case .stretch: return SF_CONTENT_STRETCH
        }
    }
}

/// A picture from the asset catalogue.
public struct Image: View {
    public typealias Body = Never
    public var body: Never { fatalError() }

    enum Source {
        case asset(String)
        case registered(ImageRegistry.Entry)
    }

    let source: Source
    var isResizable = false
    var contentMode: ContentMode = .fit

    var tint: Color = Color(r: 1, g: 1, b: 1, a: 1)

    public init(_ name: String) {
        self.source = .asset(name)
    }

    init(entry: ImageRegistry.Entry) {
        self.source = .registered(entry)
    }

    /// Lets this image be stretched to fill its frame.
    ///
    /// Without it an image reserves its asset's own size.
    public func resizable() -> Image {
        var copy = self
        copy.isResizable = true
        return copy
    }

    /// How this image fills its frame when the aspect ratios differ.
    public func aspectRatio(contentMode: ContentMode) -> Image {
        var copy = self
        copy.contentMode = contentMode
        return copy
    }

    /// Scales this image to fit its frame, letterboxed.
    public func scaledToFit() -> Image {
        aspectRatio(contentMode: .fit)
    }

    /// Scales this image to cover its frame, cropping the overflow.
    public func scaledToFill() -> Image {
        aspectRatio(contentMode: .fill)
    }

    /// Tints this image.
    public func foregroundColor(_ color: Color) -> Image {
        var copy = self
        copy.tint = color
        return copy
    }
}

extension Image {
    public func toSFNode() -> SFNode {
        var node = SFNode.makeDefault()
        node.kind = SF_NODE_IMAGE
        node.imageContentMode = contentMode.sfValue
        node.color = SFColor(r: tint.r, g: tint.g, b: tint.b, a: tint.a)

        let entry: ImageRegistry.Entry?
        switch source {
        case .asset(let name): entry = ImageRegistry.shared.asset(name)
        case .registered(let e): entry = e
        }

        guard let entry else {

            node.sizing = isResizable ? SF_SIZING_FILL : SF_SIZING_HUG
            return node
        }

        node.imageId = entry.id
        if isResizable {
            node.sizing = SF_SIZING_FILL
        } else {
            node.sizing = SF_SIZING_FIXED
            node.fixedWidth = entry.width * DeviceScale.current
            node.fixedHeight = entry.height * DeviceScale.current
        }
        return node
    }
}

import CSwiftFlow

private enum ContentUnavailableMetrics {
    static let iconSize: Float = 52
    static let titleGap: Float = 16
    static let descriptionGap: Float = 6
    static let actionsGap: Float = 20

    static let measure: Float = 40
}

/// A placeholder for a screen with nothing to show.
public struct ContentUnavailableView<Actions: View>: View {
    let title: String
    let icon: Icon
    let description: String?
    let actions: Actions

    private var hasActions: Bool { Actions.self != EmptyView.self }

    public init(
        _ title: String,
        icon: Icon,
        description: String? = nil,
        @ViewBuilder actions: () -> Actions
    ) {
        self.title = title
        self.icon = icon
        self.description = description
        self.actions = actions()
    }

    public var body: some View {
        VStack(alignment: .center, spacing: 0) {
            icon
                .size(ContentUnavailableMetrics.iconSize)
                .foregroundColor(.secondary)

            Text(title)
                .font(.title3)
                .foregroundColor(.primary)
                .padding(EdgeInsets(top: ContentUnavailableMetrics.titleGap, bottom: 0, leading: 0, trailing: 0))

            if let description {
                Text(description)
                    .font(.subheadline)
                    .foregroundColor(.secondary)
                    .padding(
                        EdgeInsets(
                            top: ContentUnavailableMetrics.descriptionGap, bottom: 0,
                            leading: 0, trailing: 0
                        )
                    )
            }

            if hasActions {
                actions
                    .padding(
                        EdgeInsets(
                            top: ContentUnavailableMetrics.actionsGap, bottom: 0,
                            leading: 0, trailing: 0
                        )
                    )
            }
        }
        .padding(
            EdgeInsets(
                top: 0, bottom: 0,
                leading: ContentUnavailableMetrics.measure,
                trailing: ContentUnavailableMetrics.measure
            )
        )

        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .center)
    }
}

extension ContentUnavailableView where Actions == EmptyView {
    public init(_ title: String, icon: Icon, description: String? = nil) {
        self.init(title, icon: icon, description: description) { EmptyView() }
    }

    public static var search: ContentUnavailableView<EmptyView> {
        ContentUnavailableView(
            "No Results",
            icon: .magnifyingGlass,
            description: "Check the spelling or try a new search."
        )
    }

    public static func search(text: String) -> ContentUnavailableView<EmptyView> {
        ContentUnavailableView(
            "No Results for \u{201C}\(text)\u{201D}",
            icon: .magnifyingGlass,
            description: "Check the spelling or try a new search."
        )
    }
}

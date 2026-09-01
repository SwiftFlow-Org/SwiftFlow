// A local stand-in named `Observable` so `@Observable`'s
// `@attached(extension, conformances: Observable)` type-checks without a
// SwiftFlowCore dependency (which would raise this package's OS floor). At an
// app's use site the generated `extension T: Observable {}` resolves to the
// framework's Observable, reached through `import SwiftFlow`.
protocol Observable: AnyObject {}

@attached(member, names: named(_observationRegistrar))
@attached(memberAttribute)
@attached(extension, conformances: Observable)
public macro Observable() =
    #externalMacro(module: "SwiftFlowMacrosPlugin", type: "ObservableMacro")

@attached(peer)
public macro ObservationIgnored() =
    #externalMacro(module: "SwiftFlowMacrosPlugin", type: "ObservationIgnoredMacro")

@attached(accessor, names: named(init), named(get), named(set))
@attached(peer, names: prefixed(_))
public macro ObservationTracked() =
    #externalMacro(module: "SwiftFlowMacrosPlugin", type: "ObservationTrackedMacro")

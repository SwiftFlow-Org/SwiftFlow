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

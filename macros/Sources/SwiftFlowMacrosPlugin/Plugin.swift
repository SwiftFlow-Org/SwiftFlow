import SwiftCompilerPlugin
import SwiftSyntaxMacros

@main
struct SwiftFlowMacrosPlugin: CompilerPlugin {
    let providingMacros: [Macro.Type] = [
        ObservableMacro.self,
        ObservationTrackedMacro.self,
        ObservationIgnoredMacro.self,
    ]
}

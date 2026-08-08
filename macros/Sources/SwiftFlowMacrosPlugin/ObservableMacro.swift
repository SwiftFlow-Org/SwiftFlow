import SwiftDiagnostics
import SwiftSyntax
import SwiftSyntaxBuilder
import SwiftSyntaxMacros

public struct ObservableMacro {}

extension ObservableMacro: MemberMacro {
    public static func expansion(
        of node: AttributeSyntax,
        providingMembersOf declaration: some DeclGroupSyntax,
        in context: some MacroExpansionContext
    ) throws -> [DeclSyntax] {
        guard declaration.is(ClassDeclSyntax.self) else {
            context.diagnose(
                Diagnostic(node: node, message: MacroError.classesOnly)
            )
            return []
        }

        return [
            "private let _observationRegistrar = ObservationRegistrar()"
        ]
    }
}

extension ObservableMacro: MemberAttributeMacro {
    public static func expansion(
        of node: AttributeSyntax,
        attachedTo declaration: some DeclGroupSyntax,
        providingAttributesFor member: some DeclSyntaxProtocol,
        in context: some MacroExpansionContext
    ) throws -> [AttributeSyntax] {
        guard let property = member.as(VariableDeclSyntax.self),
              property.shouldBeObserved
        else {
            return []
        }

        if property.hasWillSetOrDidSet {
            context.diagnose(
                Diagnostic(node: property, message: MacroError.observerOnStoredProperty)
            )
            return []
        }
        return ["@ObservationTracked"]
    }
}

extension ObservableMacro: ExtensionMacro {
    public static func expansion(
        of node: AttributeSyntax,
        attachedTo declaration: some DeclGroupSyntax,
        providingExtensionsOf type: some TypeSyntaxProtocol,
        conformingTo protocols: [TypeSyntax],
        in context: some MacroExpansionContext
    ) throws -> [ExtensionDeclSyntax] {

        guard !protocols.isEmpty else { return [] }
        return [try ExtensionDeclSyntax("extension \(type.trimmed): Observable {}")]
    }
}

public struct ObservationTrackedMacro {}

extension ObservationTrackedMacro: PeerMacro {

    public static func expansion(
        of node: AttributeSyntax,
        providingPeersOf declaration: some DeclSyntaxProtocol,
        in context: some MacroExpansionContext
    ) throws -> [DeclSyntax] {
        guard let property = declaration.as(VariableDeclSyntax.self),
              let binding = property.bindings.first,
              let name = binding.pattern.as(IdentifierPatternSyntax.self)?.identifier
        else {
            return []
        }

        var storage = "private var _\(name.text)"
        if let type = binding.typeAnnotation?.type {
            storage += ": \(type.trimmed)"
        }
        if let initializer = binding.initializer {
            storage += " \(initializer.trimmed)"
        }
        return [DeclSyntax(stringLiteral: storage)]
    }
}

extension ObservationTrackedMacro: AccessorMacro {
    public static func expansion(
        of node: AttributeSyntax,
        providingAccessorsOf declaration: some DeclSyntaxProtocol,
        in context: some MacroExpansionContext
    ) throws -> [AccessorDeclSyntax] {
        guard let property = declaration.as(VariableDeclSyntax.self),
              let binding = property.bindings.first,
              let name = binding.pattern.as(IdentifierPatternSyntax.self)?.identifier
        else {
            return []
        }
        let storage = "_\(name.text)"
        return [

            """
            @storageRestrictions(initializes: \(raw: storage))
            init(initialValue) {
                \(raw: storage) = initialValue
            }
            """,
            """
            get {
                _observationRegistrar.access()
                return \(raw: storage)
            }
            """,

            """
            set {
                \(raw: storage) = newValue
                _observationRegistrar.didMutate()
            }
            """,
        ]
    }
}

public struct ObservationIgnoredMacro: PeerMacro {

    public static func expansion(
        of node: AttributeSyntax,
        providingPeersOf declaration: some DeclSyntaxProtocol,
        in context: some MacroExpansionContext
    ) throws -> [DeclSyntax] {
        []
    }
}

enum MacroError: String, DiagnosticMessage {
    case classesOnly
    case observerOnStoredProperty

    var message: String {
        switch self {
        case .classesOnly:
            return """
                @Observable can only be applied to a class. Observation is \
                about state that several views share and one mutates; a \
                struct is copied into each of them. Use @State instead.
                """
        case .observerOnStoredProperty:
            return """
                @Observable cannot track a property with willSet or didSet, \
                because making it observable makes it computed and the \
                observer would silently stop running. Mark it \
                @ObservationIgnored, or move the side effect into a method.
                """
        }
    }

    var diagnosticID: MessageID { MessageID(domain: "SwiftFlowMacros", id: rawValue) }
    var severity: DiagnosticSeverity { .error }
}

extension VariableDeclSyntax {

    var shouldBeObserved: Bool {

        guard bindingSpecifier.tokenKind == .keyword(.var) else { return false }

        guard !isStaticOrClass else { return false }

        guard !hasAttribute(named: "ObservationIgnored"),
              !hasAttribute(named: "ObservationTracked")
        else { return false }

        guard bindings.count == 1, let binding = bindings.first else { return false }

        switch binding.accessorBlock?.accessors {
        case .none: return true
        case .accessors: return true
        case .getter: return false
        }
    }

    var hasWillSetOrDidSet: Bool {
        guard let accessors = bindings.first?.accessorBlock?.accessors,
              case .accessors(let list) = accessors
        else { return false }
        return list.contains {
            $0.accessorSpecifier.tokenKind == .keyword(.willSet)
                || $0.accessorSpecifier.tokenKind == .keyword(.didSet)
        }
    }

    var isStaticOrClass: Bool {
        modifiers.contains {
            $0.name.tokenKind == .keyword(.static) || $0.name.tokenKind == .keyword(.class)
        }
    }

    func hasAttribute(named name: String) -> Bool {
        attributes.contains { attribute in
            guard case .attribute(let attribute) = attribute else { return false }
            return attribute.attributeName.trimmedDescription == name
        }
    }
}

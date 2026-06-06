import Foundation
import SwiftTreeSitter
import TreeSitterTSX
import TreeSitterTypeScript

public enum TreeSitterSemanticTokenName: Hashable {
    case comment
    case function
    case keyword
    case number
    case parameter
    case string
    case type
}

public struct TreeSitterSemanticHighlight: Hashable {
    public let name: TreeSitterSemanticTokenName
    public let range: NSRange

    public init(name: TreeSitterSemanticTokenName, range: NSRange) {
        self.name = name
        self.range = range
    }
}

public enum TreeSitterSemanticHighlighter {
    public static func highlightRanges(
        in source: String,
        language: String?
    ) throws -> [TreeSitterSemanticHighlight] {
        guard let configuration = try configuration(for: language) else {
            return []
        }

        let parser = Parser()
        try parser.setLanguage(configuration.language)
        guard let tree = parser.parse(source),
              let query = configuration.queries[.highlights] else {
            return []
        }

        let cursor = query.execute(in: tree)
        let highlights = cursor
            .resolve(with: .init(string: source))
            .highlights()

        let sourceLength = (source as NSString).length
        return highlights.compactMap { namedRange in
            guard let tokenName = tokenName(for: namedRange.nameComponents) else {
                return nil
            }

            let range = namedRange.range
            guard range.location >= 0,
                  range.length > 0,
                  NSMaxRange(range) <= sourceLength else {
                return nil
            }

            return TreeSitterSemanticHighlight(name: tokenName, range: range)
        }
    }

    private static func configuration(for language: String?) throws -> LanguageConfiguration? {
        switch language?.lowercased() {
        case "ts", "typescript":
            try typeScriptConfiguration.get()
        case "tsx":
            try tsxConfiguration.get()
        default:
            nil
        }
    }

    private static let typeScriptConfiguration = Result {
        try makeConfiguration(
            language: Language(language: tree_sitter_typescript()),
            name: "TypeScript"
        )
    }

    private static let tsxConfiguration = Result {
        try makeConfiguration(
            language: Language(language: tree_sitter_tsx()),
            name: "TSX"
        )
    }

    private static func makeConfiguration(
        language: Language,
        name: String
    ) throws -> LanguageConfiguration {
        let query = try Query(
            language: language,
            data: Data(typeScriptHighlightQuery.utf8)
        )
        return LanguageConfiguration(language, name: name, queries: [.highlights: query])
    }

    private static func tokenName(for components: [String]) -> TreeSitterSemanticTokenName? {
        guard let head = components.first else { return nil }

        switch head {
        case "comment":
            return .comment
        case "function", "method":
            return .function
        case "keyword":
            return .keyword
        case "number":
            return .number
        case "string":
            return .string
        case "type":
            return .type
        case "variable" where components.dropFirst().contains("parameter"):
            return .parameter
        default:
            return nil
        }
    }

    private static let typeScriptHighlightQuery = #"""
    ; Comments
    (comment) @comment

    ; Literals
    (string) @string
    (template_string) @string
    (number) @number

    ; Types
    (type_identifier) @type
    (predefined_type) @type.builtin
    ((identifier) @type
      (#match? @type "^[A-Z]"))

    ; Parameters
    (required_parameter (identifier) @variable.parameter)
    (optional_parameter (identifier) @variable.parameter)

    ; Functions and methods
    (function_declaration
      name: (identifier) @function)
    (function_signature
      name: (identifier) @function)
    (method_signature
      name: (property_identifier) @function.method)
    (call_expression
      function: (identifier) @function)
    (call_expression
      function: (member_expression
        property: (property_identifier) @function.method))

    ; Keywords
    [
      (false)
      (null)
      (this)
      (true)
      (undefined)
    ] @keyword

    [
      "abstract"
      "any"
      "as"
      "async"
      "await"
      "boolean"
      "break"
      "case"
      "catch"
      "class"
      "const"
      "continue"
      "declare"
      "default"
      "delete"
      "do"
      "else"
      "enum"
      "export"
      "extends"
      "finally"
      "for"
      "from"
      "function"
      "if"
      "implements"
      "import"
      "in"
      "infer"
      "instanceof"
      "interface"
      "keyof"
      "let"
      "module"
      "namespace"
      "never"
      "new"
      "object"
      "of"
      "private"
      "protected"
      "public"
      "readonly"
      "override"
      "return"
      "satisfies"
      "static"
      "switch"
      "throw"
      "try"
      "type"
      "typeof"
      "unknown"
      "var"
      "void"
      "while"
      "with"
      "yield"
    ] @keyword
    """#
}

import Foundation

public struct MarkdownDocument: Equatable, Sendable {
    public let url: URL
    public let title: String
    public let content: String

    public init(url: URL, title: String, content: String) {
        self.url = url
        self.title = title
        self.content = content
    }
}

public enum MarkdownFileError: LocalizedError, Equatable, Sendable {
    case unsupportedExtension(URL)
    case notFound(URL)
    case unreadable(URL)

    public var errorDescription: String? {
        switch self {
        case .unsupportedExtension(let url):
            "Unsupported file type: \(url.lastPathComponent)"
        case .notFound(let url):
            "File not found: \(url.lastPathComponent)"
        case .unreadable(let url):
            "Could not read file as UTF-8: \(url.lastPathComponent)"
        }
    }
}

public enum MarkdownFile {
    private static let supportedExtensions = Set(["md", "markdown", "mdx"])

    public static func isSupported(_ url: URL) -> Bool {
        supportedExtensions.contains(url.pathExtension.lowercased())
    }

    public static func load(_ url: URL) throws -> MarkdownDocument {
        guard isSupported(url) else {
            throw MarkdownFileError.unsupportedExtension(url)
        }

        guard FileManager.default.fileExists(atPath: url.path) else {
            throw MarkdownFileError.notFound(url)
        }

        guard let content = try? String(contentsOf: url, encoding: .utf8) else {
            throw MarkdownFileError.unreadable(url)
        }

        return MarkdownDocument(
            url: url,
            title: frontmatterTitle(in: content) ?? url.lastPathComponent,
            content: content
        )
    }

    private static func frontmatterTitle(in content: String) -> String? {
        let lines = content.components(separatedBy: .newlines)
        guard let firstLine = lines.first,
              firstLine.trimmingCharacters(in: .whitespaces) == "---" else {
            return nil
        }

        for line in lines.dropFirst() {
            let trimmed = line.trimmingCharacters(in: .whitespaces)
            if trimmed == "---" {
                return nil
            }
            guard trimmed.hasPrefix("title:") else {
                continue
            }

            let valueStart = trimmed.index(trimmed.startIndex, offsetBy: "title:".count)
            let value = unquotedFrontmatterValue(
                String(trimmed[valueStart...]).trimmingCharacters(in: .whitespaces)
            )
            return value.isEmpty ? nil : value
        }

        return nil
    }

    private static func unquotedFrontmatterValue(_ source: String) -> String {
        guard source.count >= 2,
              let first = source.first,
              let last = source.last,
              (first == "\"" && last == "\"") || (first == "'" && last == "'") else {
            return source
        }

        return String(source.dropFirst().dropLast())
    }
}

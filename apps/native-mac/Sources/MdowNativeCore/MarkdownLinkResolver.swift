import Foundation

public enum MarkdownLinkTarget: Equatable, Sendable {
    case anchor(String)
    case markdownFile(URL, anchor: String?)
    case localFile(URL)
    case external(URL)
    case ignored
}

public enum MarkdownLinkResolver {
    public static func resolve(href: String, documentURL: URL) -> MarkdownLinkTarget {
        let trimmedHref = href.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmedHref.isEmpty else { return .ignored }

        if trimmedHref.hasPrefix("#") {
            return .anchor(decodedFragment(String(trimmedHref.dropFirst())))
        }

        if let url = URL(string: trimmedHref),
           let scheme = url.scheme?.lowercased(),
           scheme != "file",
           !scheme.isEmpty {
            return .external(url)
        }

        let parts = splitFragment(trimmedHref)
        let path = decodedPath(parts.path)
        let resolvedURL = resolvedLocalURL(path: path, documentURL: documentURL)

        if isMarkdownPath(path) {
            return .markdownFile(resolvedURL.standardizedFileURL, anchor: parts.fragment.map(decodedFragment))
        }

        if resolvedURL.pathExtension.isEmpty,
           let probedURL = probedMarkdownURL(for: resolvedURL) {
            return .markdownFile(probedURL.standardizedFileURL, anchor: parts.fragment.map(decodedFragment))
        }

        if resolvedURL.pathExtension.isEmpty {
            return .markdownFile(
                resolvedURL.appendingPathExtension("md").standardizedFileURL,
                anchor: parts.fragment.map(decodedFragment)
            )
        }

        return .localFile(resolvedURL.standardizedFileURL)
    }

    public static func slug(_ title: String) -> String {
        let allowed = CharacterSet.alphanumerics.union(CharacterSet(charactersIn: " -_"))
        return title.lowercased()
            .unicodeScalars
            .filter { allowed.contains($0) }
            .map { Character($0) == " " ? "-" : String($0) }
            .joined()
            .replacingOccurrences(of: "--", with: "-")
    }

    private static func splitFragment(_ href: String) -> (path: String, fragment: String?) {
        guard let hashIndex = href.firstIndex(of: "#") else {
            return (href, nil)
        }

        let path = String(href[..<hashIndex])
        let fragment = String(href[href.index(after: hashIndex)...])
        return (path, fragment.isEmpty ? nil : fragment)
    }

    private static func decodedPath(_ path: String) -> String {
        path.removingPercentEncoding ?? path
    }

    private static func decodedFragment(_ fragment: String) -> String {
        fragment.removingPercentEncoding ?? fragment
    }

    private static func resolvedLocalURL(path: String, documentURL: URL) -> URL {
        if path.hasPrefix("/") {
            return URL(fileURLWithPath: path)
        }
        if let fileURL = URL(string: path), fileURL.isFileURL {
            return fileURL
        }
        return documentURL.deletingLastPathComponent().appendingPathComponent(path)
    }

    private static func probedMarkdownURL(for url: URL) -> URL? {
        for pathExtension in ["md", "markdown", "mdx"] {
            let candidate = url.appendingPathExtension(pathExtension)
            if FileManager.default.fileExists(atPath: candidate.path) {
                return candidate
            }
        }
        return nil
    }

    private static func isMarkdownPath(_ path: String) -> Bool {
        let lowercasedPath = path.lowercased()
        return lowercasedPath.hasSuffix(".md")
            || lowercasedPath.hasSuffix(".markdown")
            || lowercasedPath.hasSuffix(".mdx")
    }
}

import Foundation

public enum MarkdownOpenURLTarget: Equatable, Sendable {
    case folder(URL)
    case markdownFile(URL)
    case unsupportedFile(URL)
    case ignored
}

public enum MarkdownOpenURLRouting {
    public static func target(for url: URL) -> MarkdownOpenURLTarget {
        if isDirectory(url) {
            return .folder(url)
        }

        if MarkdownFile.isSupported(url) {
            return .markdownFile(url)
        }

        if isExistingFile(url) {
            return .unsupportedFile(url)
        }

        return .ignored
    }

    private static func isDirectory(_ url: URL) -> Bool {
        (try? url.resourceValues(forKeys: [.isDirectoryKey]).isDirectory) == true
    }

    private static func isExistingFile(_ url: URL) -> Bool {
        (try? url.resourceValues(forKeys: [.isRegularFileKey]).isRegularFile) == true
    }
}

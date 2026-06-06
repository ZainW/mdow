import Foundation

public struct DocumentBreadcrumbSegment: Equatable, Identifiable, Sendable {
    public let name: String
    public let url: URL

    public var id: String { url.path }

    public init(name: String, url: URL) {
        self.name = name
        self.url = url
    }
}

public enum DocumentBreadcrumbSegments {
    public static func parentSegments(for fileURL: URL, rootURL: URL?) -> [DocumentBreadcrumbSegment] {
        let directoryURL = fileURL.deletingLastPathComponent().standardizedFileURL
        let directoryComponents = directoryURL.pathComponents
        guard !directoryComponents.isEmpty else { return [] }

        var startIndex = max(0, directoryComponents.count - 3)
        if let rootURL {
            let rootComponents = rootURL.standardizedFileURL.pathComponents
            if !rootComponents.isEmpty,
               rootComponents.count <= directoryComponents.count,
               Array(directoryComponents.prefix(rootComponents.count)) == rootComponents {
                startIndex = max(0, rootComponents.count - 1)
            }
        }

        return directoryComponents.indices.compactMap { index in
            guard index >= startIndex else { return nil }
            let name = directoryComponents[index]
            guard name != "/" else { return nil }
            return DocumentBreadcrumbSegment(
                name: name,
                url: URL(fileURLWithPath: path(from: directoryComponents, through: index))
            )
        }
    }

    private static func path(from components: [String], through index: Int) -> String {
        guard components.first == "/" else {
            return components.prefix(index + 1).joined(separator: "/")
        }

        let tail = components.dropFirst().prefix(index)
        return "/" + tail.joined(separator: "/")
    }
}

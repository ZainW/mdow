import Foundation

public struct MarkdownSearchMatch: Equatable, Sendable {
    public let offset: Int
    public let length: Int

    public init(offset: Int, length: Int) {
        self.offset = offset
        self.length = length
    }

    public func range(in source: String) -> Range<String.Index>? {
        guard let lowerBound = source.index(
            source.startIndex,
            offsetBy: offset,
            limitedBy: source.endIndex
        ),
              let upperBound = source.index(
                  lowerBound,
                  offsetBy: length,
                  limitedBy: source.endIndex
              ) else {
            return nil
        }

        return lowerBound..<upperBound
    }

    public func nsRange(in source: String) -> NSRange? {
        guard let range = range(in: source) else { return nil }
        return NSRange(range, in: source)
    }
}

public enum MarkdownSearch {
    public static func matches(in source: String, query: String) -> [MarkdownSearchMatch] {
        guard !query.isEmpty else { return [] }

        var matches: [MarkdownSearchMatch] = []
        var searchRange = source.startIndex..<source.endIndex

        while let range = source.range(
            of: query,
            options: [.caseInsensitive, .diacriticInsensitive],
            range: searchRange
        ) {
            let offset = source.distance(from: source.startIndex, to: range.lowerBound)
            let length = source.distance(from: range.lowerBound, to: range.upperBound)
            matches.append(.init(offset: offset, length: length))
            searchRange = range.upperBound..<source.endIndex
        }

        return matches
    }
}

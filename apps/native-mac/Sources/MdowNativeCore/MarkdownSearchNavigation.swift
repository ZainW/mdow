import Foundation

public struct MarkdownSearchNavigationTarget: Equatable, Sendable {
    public let blockIndex: Int
    public let matchIndex: Int

    public init(blockIndex: Int, matchIndex: Int) {
        self.blockIndex = blockIndex
        self.matchIndex = matchIndex
    }
}

public enum MarkdownSearchNavigation {
    public static func targets(in blockTexts: [String], query: String) -> [MarkdownSearchNavigationTarget] {
        guard !query.isEmpty else { return [] }

        return blockTexts.enumerated().flatMap { blockIndex, text in
            MarkdownSearch.matches(in: text, query: query).indices.map { matchIndex in
                MarkdownSearchNavigationTarget(blockIndex: blockIndex, matchIndex: matchIndex)
            }
        }
    }

    public static func currentIndex(position: Int, targetCount: Int) -> Int? {
        guard targetCount > 0 else { return nil }
        guard position >= 0 else { return 0 }
        return min(position, targetCount - 1)
    }

    public static func counterText(query: String, targetCount: Int, currentIndex: Int?) -> String {
        guard !query.isEmpty else { return "" }
        guard targetCount > 0 else { return "No results" }

        let boundedIndex = min(max(currentIndex ?? 0, 0), targetCount - 1)
        return "\(boundedIndex + 1) of \(targetCount)"
    }

    public static func advancedPosition(
        currentPosition: Int,
        targetCount: Int,
        offset: Int,
        queryChanged: Bool
    ) -> Int? {
        guard targetCount > 0 else { return nil }

        if queryChanged {
            return offset < 0 ? targetCount - 1 : 0
        }

        let visiblePosition = min(max(currentPosition, 0), targetCount - 1)
        return (visiblePosition + offset + targetCount) % targetCount
    }
}

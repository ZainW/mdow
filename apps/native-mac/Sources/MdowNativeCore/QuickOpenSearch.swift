import Foundation

public struct QuickOpenSearchResult: Equatable, Sendable {
    public let file: MarkdownFileSummary
    public let score: Double

    public init(file: MarkdownFileSummary, score: Double) {
        self.file = file
        self.score = score
    }
}

public enum QuickOpenSearch {
    public static func deduped(_ files: [MarkdownFileSummary]) -> [MarkdownFileSummary] {
        var seen = Set<String>()
        return files.filter { file in
            let key = canonicalKey(file.url)
            guard !seen.contains(key) else { return false }
            seen.insert(key)
            return true
        }
    }

    public static func results(
        query: String,
        files: [MarkdownFileSummary],
        maxResults: Int = 50
    ) -> [QuickOpenSearchResult] {
        let files = deduped(files)
        let trimmedQuery = query.trimmingCharacters(in: .whitespacesAndNewlines)

        guard !trimmedQuery.isEmpty else {
            return files.prefix(maxResults).map { QuickOpenSearchResult(file: $0, score: 0) }
        }

        return files.compactMap { file in
            let nameScore = fuzzyScore(query: trimmedQuery, target: file.title)
            let pathScore = fuzzyScore(query: trimmedQuery, target: file.url.path)
            let score = max(Double(nameScore) * 1.5, Double(pathScore))
            guard score > 0 else { return nil }
            return QuickOpenSearchResult(file: file, score: score)
        }
        .sorted { lhs, rhs in
            if lhs.score != rhs.score { return lhs.score > rhs.score }
            return lhs.file.url.path.localizedStandardCompare(rhs.file.url.path) == .orderedAscending
        }
        .prefix(maxResults)
        .map { $0 }
    }

    private static func fuzzyScore(query: String, target: String) -> Int {
        let queryCharacters = Array(query.lowercased())
        let targetCharacters = Array(target.lowercased())
        guard !queryCharacters.isEmpty else { return 0 }

        var score = 0
        var queryIndex = 0
        var consecutive = 0

        for targetIndex in targetCharacters.indices where queryIndex < queryCharacters.count {
            if targetCharacters[targetIndex] == queryCharacters[queryIndex] {
                score += 1

                if consecutive > 0 {
                    score += consecutive * 2
                }
                consecutive += 1

                if isBoundary(targetCharacters, at: targetIndex) {
                    score += 5
                }

                if queryIndex == 0 && targetIndex == 0 {
                    score += 3
                }

                queryIndex += 1
            } else {
                consecutive = 0
            }
        }

        return queryIndex < queryCharacters.count ? 0 : score
    }

    private static func isBoundary(_ characters: [Character], at index: Int) -> Bool {
        guard index > 0 else { return true }
        return ["/", "-", "_", "."].contains(characters[index - 1])
    }

    private static func canonicalKey(_ url: URL) -> String {
        let path = url.standardizedFileURL.resolvingSymlinksInPath().path
        guard path.hasPrefix("/private/tmp/") else { return path }
        return "/tmp/" + path.dropFirst("/private/tmp/".count)
    }
}

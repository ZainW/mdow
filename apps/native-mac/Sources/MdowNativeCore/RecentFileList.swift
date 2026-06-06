import Foundation

public enum RecentFileList {
    public static func deduped(_ files: [MarkdownFileSummary]) -> [MarkdownFileSummary] {
        var seen = Set<String>()
        return files.filter { file in
            let key = canonicalKey(file.url)
            guard !seen.contains(key) else { return false }
            seen.insert(key)
            return true
        }
    }

    public static func removing(
        _ url: URL,
        from files: [MarkdownFileSummary]
    ) -> [MarkdownFileSummary] {
        let key = canonicalKey(url)
        return files.filter { canonicalKey($0.url) != key }
    }

    public static func prepending(
        _ url: URL,
        to files: [MarkdownFileSummary],
        limit: Int
    ) -> [MarkdownFileSummary] {
        let newFile = MarkdownFileSummary(url: url)
        return Array(deduped([newFile] + files).prefix(limit))
    }

    private static func canonicalKey(_ url: URL) -> String {
        let path = url.standardizedFileURL.resolvingSymlinksInPath().path
        guard path.hasPrefix("/private/tmp/") else { return path }
        return "/tmp/" + path.dropFirst("/private/tmp/".count)
    }
}

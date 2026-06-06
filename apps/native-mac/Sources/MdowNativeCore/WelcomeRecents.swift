import Foundation

public enum WelcomeRecents {
    public static func displayed(_ files: [MarkdownFileSummary]) -> [MarkdownFileSummary] {
        Array(files.prefix(6))
    }
}

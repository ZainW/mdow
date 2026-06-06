import Foundation

public enum MarkdownAnchorResolver {
    public static func blockIndex(for anchor: String, in blocks: [MarkdownBlock]) -> Int? {
        let normalizedAnchor = anchor.lowercased()

        return blocks.enumerated().compactMap { index, block -> Int? in
            switch block {
            case .heading(_, let title):
                title == anchor || MarkdownLinkResolver.slug(title) == normalizedAnchor ? index : nil
            case .footnoteDefinition(let label, _):
                "fn-\(label.lowercased())" == normalizedAnchor ? index : nil
            default:
                nil
            }
        }.first
    }
}

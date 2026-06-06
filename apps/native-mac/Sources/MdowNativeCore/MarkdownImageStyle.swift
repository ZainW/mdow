import Foundation

public struct MarkdownImageStyle: Equatable, Sendable {
    public let cornerRadius: Int
    public let captionFontSize: Int

    public static let standard = MarkdownImageStyle(
        cornerRadius: 8,
        captionFontSize: 12
    )
}

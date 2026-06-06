import Foundation

public struct MarkdownInlineCodeStyle: Equatable, Sendable {
    public let fontScale: Double
    public let horizontalPadding: Int
    public let cornerRadius: Int

    public static let standard = MarkdownInlineCodeStyle(
        fontScale: 0.875,
        horizontalPadding: 5,
        cornerRadius: 4
    )
}

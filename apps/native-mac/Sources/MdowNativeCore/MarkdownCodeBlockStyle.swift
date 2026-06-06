import Foundation

public struct MarkdownCodeBlockStyle: Equatable, Sendable {
    public let cornerRadius: Int
    public let horizontalPadding: Int
    public let verticalPadding: Int
    public let languageFontSize: Int
    public let codeFontScale: Double
    public let shadowOffsetY: Int

    public static let standard = MarkdownCodeBlockStyle(
        cornerRadius: 10,
        horizontalPadding: 18,
        verticalPadding: 14,
        languageFontSize: 11,
        codeFontScale: 0.875,
        shadowOffsetY: 1
    )
}

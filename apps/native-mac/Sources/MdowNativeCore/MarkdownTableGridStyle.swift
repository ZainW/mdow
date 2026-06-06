import Foundation

public struct MarkdownTableGridStyle: Equatable, Sendable {
    public let cornerRadius: Int
    public let horizontalPadding: Int
    public let verticalPadding: Int
    public let headerFontScale: Double
    public let bodyFontScale: Double

    public static let standard = MarkdownTableGridStyle(
        cornerRadius: 8,
        horizontalPadding: 14,
        verticalPadding: 10,
        headerFontScale: 0.8,
        bodyFontScale: 0.925
    )
}

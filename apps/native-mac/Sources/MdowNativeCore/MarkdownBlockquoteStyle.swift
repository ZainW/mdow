import Foundation

public struct MarkdownBlockquoteStyle: Equatable, Sendable {
    public let borderWidth: Int
    public let horizontalPadding: Int
    public let verticalPadding: Int
    public let textIsMuted: Bool

    public static let standard = MarkdownBlockquoteStyle(
        borderWidth: 3,
        horizontalPadding: 16,
        verticalPadding: 6,
        textIsMuted: true
    )
}

import Foundation

public enum MarkdownHeadingEmphasis: Equatable, Sendable {
    case semibold
    case bold
}

public struct MarkdownHeadingPresentation: Equatable, Sendable {
    public let fontScale: Double
    public let topPadding: Int
    public let emphasis: MarkdownHeadingEmphasis
    public let isMuted: Bool
    public let isUppercase: Bool

    public init(level: Int) {
        switch level {
        case 1:
            self.fontScale = 1.875
            self.topPadding = 22
            self.emphasis = .bold
            self.isMuted = false
            self.isUppercase = false
        case 2:
            self.fontScale = 1.5
            self.topPadding = 18
            self.emphasis = .semibold
            self.isMuted = false
            self.isUppercase = false
        case 3:
            self.fontScale = 1.15
            self.topPadding = 14
            self.emphasis = .semibold
            self.isMuted = false
            self.isUppercase = false
        case 4:
            self.fontScale = 1.0
            self.topPadding = 12
            self.emphasis = .semibold
            self.isMuted = true
            self.isUppercase = false
        case 5:
            self.fontScale = 0.95
            self.topPadding = 10
            self.emphasis = .semibold
            self.isMuted = true
            self.isUppercase = false
        default:
            self.fontScale = 0.875
            self.topPadding = 8
            self.emphasis = .semibold
            self.isMuted = true
            self.isUppercase = true
        }
    }
}

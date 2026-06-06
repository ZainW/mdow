import Foundation

public enum DocumentReadingAlignment: Equatable, Sendable {
    case leading
    case center
}

public struct DocumentReadingLayout: Equatable, Sendable {
    public let maxContentWidth: Int?
    public let horizontalPadding: Int
    public let alignment: DocumentReadingAlignment

    public init(readingWidth: ReadingWidth, isWide: Bool) {
        self.maxContentWidth = isWide ? nil : readingWidth.documentMaxWidth
        self.horizontalPadding = 48
        self.alignment = isWide ? .leading : .center
    }
}

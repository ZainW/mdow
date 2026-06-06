import Foundation

public enum MarkdownFileErrorKind: String, Sendable {
    case notFound
    case permissionDenied
    case readError
}

public struct MarkdownFileErrorModel: Equatable, Sendable {
    public let url: URL
    public let kind: MarkdownFileErrorKind
    public let title: String
    public let body: String
    public let canRevealInFinder: Bool

    public init(url: URL, error: Error) {
        self.url = url

        if let markdownError = error as? MarkdownFileError {
            switch markdownError {
            case .notFound:
                self.kind = .notFound
                self.title = "File not found"
                self.body = "This file may have been moved or renamed."
                self.canRevealInFinder = true
            case .unsupportedExtension, .unreadable:
                self.kind = .readError
                self.title = "Couldn't read file"
                self.body = "Something went wrong trying to read this file. It might be corrupted or locked by another process."
                self.canRevealInFinder = true
            }
            return
        }

        let nsError = error as NSError
        if nsError.domain == NSCocoaErrorDomain,
           nsError.code == NSFileReadNoPermissionError {
            self.kind = .permissionDenied
            self.title = "Access denied"
            self.body = "You don't have permission to read this file. Check the file permissions and try again."
            self.canRevealInFinder = false
            return
        }

        self.kind = .readError
        self.title = "Couldn't read file"
        self.body = "Something went wrong trying to read this file. It might be corrupted or locked by another process."
        self.canRevealInFinder = true
    }
}

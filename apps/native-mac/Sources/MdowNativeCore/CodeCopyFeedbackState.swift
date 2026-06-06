import Foundation

public struct CodeCopyFeedbackState: Equatable, Sendable {
    public let copiedCodeIDs: Set<String>

    public init(copiedCodeIDs: Set<String> = []) {
        self.copiedCodeIDs = copiedCodeIDs
    }

    public func copying(_ id: String) -> CodeCopyFeedbackState {
        var next = copiedCodeIDs
        next.insert(id)
        return CodeCopyFeedbackState(copiedCodeIDs: next)
    }

    public func resetting(_ id: String) -> CodeCopyFeedbackState {
        var next = copiedCodeIDs
        next.remove(id)
        return CodeCopyFeedbackState(copiedCodeIDs: next)
    }

    public func isCopied(_ id: String) -> Bool {
        copiedCodeIDs.contains(id)
    }
}

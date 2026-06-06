import Foundation

public enum DocumentDisplayTitle {
    public static func secondaryFilename(documentTitle: String, fileURL: URL) -> String? {
        let filename = fileURL.lastPathComponent
        return documentTitle == filename ? nil : filename
    }
}

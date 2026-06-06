import Foundation

public enum DocumentTabOperations {
    public static func closeOthers(in urls: [URL], keeping target: URL) -> [URL] {
        urls.filter { $0 == target }
    }

    public static func closeToRight(in urls: [URL], after target: URL) -> [URL] {
        guard let index = urls.firstIndex(of: target) else { return urls }
        return Array(urls.prefix(index + 1))
    }
}

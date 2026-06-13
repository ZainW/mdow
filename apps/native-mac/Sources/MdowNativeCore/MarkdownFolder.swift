import Darwin
import Foundation

public struct MarkdownFileSummary: Equatable, Identifiable, Sendable {
    public let id: URL
    public let url: URL
    public let title: String

    public init(url: URL) {
        self.id = url
        self.url = url
        self.title = url.lastPathComponent
    }
}

public struct MarkdownFolderScanResult: Equatable, Sendable {
    public let files: [MarkdownFileSummary]
    public let didReachLimit: Bool
    public let maxScannedEntries: Int

    public init(files: [MarkdownFileSummary], didReachLimit: Bool, maxScannedEntries: Int) {
        self.files = files
        self.didReachLimit = didReachLimit
        self.maxScannedEntries = maxScannedEntries
    }
}

public enum MarkdownFolder {
    private static let maxScannedEntries = 5_000

    public static func scan(_ root: URL) throws -> [MarkdownFileSummary] {
        try scanWithMetadata(root).files
    }

    public static func scanWithMetadata(_ root: URL) throws -> MarkdownFolderScanResult {
        var files: [MarkdownFileSummary] = []
        var scannedEntries = 0
        var didReachLimit = false

        func scanDirectory(_ directoryPath: String) {
            guard scannedEntries < Self.maxScannedEntries else {
                didReachLimit = true
                return
            }
            guard let directory = opendir(directoryPath) else { return }
            defer { closedir(directory) }

            var childNames: [String] = []
            while let entry = readdir(directory) {
                let name = withUnsafePointer(to: entry.pointee.d_name) {
                    $0.withMemoryRebound(to: CChar.self, capacity: 1) {
                        String(cString: $0)
                    }
                }

                guard name != ".", name != "..", !name.hasPrefix(".") else { continue }
                childNames.append(name)
            }

            for childName in childNames.sorted(by: { lhs, rhs in
                lhs.localizedStandardCompare(rhs) == .orderedAscending
            }) {
                guard scannedEntries < Self.maxScannedEntries else {
                    didReachLimit = true
                    return
                }
                scannedEntries += 1

                let childPath = NSString(string: directoryPath).appendingPathComponent(childName)
                var statBuffer = stat()
                guard lstat(childPath, &statBuffer) == 0 else { continue }

                let mode = statBuffer.st_mode
                if (mode & S_IFMT) == S_IFLNK {
                    continue
                } else if (mode & S_IFMT) == S_IFDIR {
                    guard !childName.hasSuffix(".app"),
                          !childName.hasSuffix(".bundle"),
                          !childName.hasSuffix(".framework") else {
                        continue
                    }
                    scanDirectory(childPath)
                } else if (mode & S_IFMT) == S_IFREG {
                    let fileURL = URL(fileURLWithPath: childPath)
                    if MarkdownFile.isSupported(fileURL) {
                        files.append(MarkdownFileSummary(url: fileURL))
                    }
                }
            }
        }

        scanDirectory(root.path)

        let sortedFiles = files.sorted {
            $0.url.path.localizedStandardCompare($1.url.path) == .orderedAscending
        }
        return MarkdownFolderScanResult(
            files: sortedFiles,
            didReachLimit: didReachLimit,
            maxScannedEntries: Self.maxScannedEntries
        )
    }
}

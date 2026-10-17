import Foundation

/// A node in the sidebar's folder tree: folders that contain documents, and the documents.
public final class FolderNode: @unchecked Sendable {
  public let url: URL
  public let isFolder: Bool
  public private(set) var children: [FolderNode]

  public var name: String { url.lastPathComponent }

  init(url: URL, isFolder: Bool, children: [FolderNode] = []) {
    self.url = url
    self.isFolder = isFolder
    self.children = children
  }

  static let skipped: Set<String> = [
    "node_modules", ".git", ".build", "build", "dist", "target", "DerivedData", "Pods", ".next",
    ".turbo", ".venv", "venv", "__pycache__", ".cache",
  ]

  /// Scans a folder for markdown and HTML files, keeping only folders that lead to one.
  public static func scan(_ root: URL, limit: Int = 20_000) -> FolderNode {
    var budget = limit
    return scanFolder(root, depth: 0, budget: &budget)
      ?? FolderNode(url: root, isFolder: true)
  }

  private static func scanFolder(_ url: URL, depth: Int, budget: inout Int) -> FolderNode? {
    guard depth < 12, budget > 0 else { return nil }
    let keys: [URLResourceKey] = [.isDirectoryKey, .isHiddenKey, .isPackageKey]
    guard
      let entries = try? FileManager.default.contentsOfDirectory(
        at: url, includingPropertiesForKeys: keys, options: [.skipsHiddenFiles, .skipsPackageDescendants])
    else { return nil }
    var folders: [FolderNode] = []
    var files: [FolderNode] = []
    for found in entries {
      // Keep the caller's spelling of the path (e.g. /tmp, not /private/tmp).
      let entry = url.appendingPathComponent(found.lastPathComponent, isDirectory: false)
      budget -= 1
      if budget <= 0 { break }
      let values = try? found.resourceValues(forKeys: Set(keys))
      if values?.isDirectory == true, values?.isPackage != true {
        if skipped.contains(entry.lastPathComponent) { continue }
        if let folder = scanFolder(url.appendingPathComponent(found.lastPathComponent, isDirectory: true), depth: depth + 1, budget: &budget), !folder.children.isEmpty {
          folders.append(folder)
        }
      } else if FileKind.isOpenable(entry) {
        files.append(FolderNode(url: entry, isFolder: false))
      }
    }
    let order: (FolderNode, FolderNode) -> Bool = {
      $0.name.localizedStandardCompare($1.name) == .orderedAscending
    }
    return FolderNode(url: url, isFolder: true, children: folders.sorted(by: order) + files.sorted(by: order))
  }

  /// Every document under this node, depth first.
  public var allFiles: [URL] {
    isFolder ? children.flatMap(\.allFiles) : [url]
  }

  /// A copy keeping only documents whose name or path matches `query`.
  public func filtered(_ query: String) -> FolderNode? {
    let query = query.trimmingCharacters(in: .whitespaces)
    if query.isEmpty { return self }
    if !isFolder {
      return name.localizedCaseInsensitiveContains(query) ? self : nil
    }
    let kept = children.compactMap { $0.filtered(query) }
    return kept.isEmpty ? nil : FolderNode(url: url, isFolder: true, children: kept)
  }
}

/// Calls back when a file changes on disk, including when an editor replaces it by renaming
/// a new file over it.
public final class FileWatcher: @unchecked Sendable {
  private let url: URL
  private let queue = DispatchQueue(label: "mdow.file-watcher")
  private let onChange: @Sendable () -> Void
  private var source: DispatchSourceFileSystemObject?
  private var pending: DispatchWorkItem?
  private var cancelled = false

  public init(url: URL, onChange: @escaping @Sendable () -> Void) {
    self.url = url
    self.onChange = onChange
    queue.async { self.start() }
  }

  deinit {
    source?.cancel()
  }

  public func cancel() {
    queue.async {
      self.cancelled = true
      self.source?.cancel()
      self.source = nil
    }
  }

  private func start() {
    guard !cancelled else { return }
    let descriptor = open(url.path, O_EVTONLY)
    guard descriptor >= 0 else {
      // The file is mid-replace; look again shortly.
      queue.asyncAfter(deadline: .now() + 0.5) { self.start() }
      return
    }
    let source = DispatchSource.makeFileSystemObjectSource(
      fileDescriptor: descriptor, eventMask: [.write, .extend, .delete, .rename, .attrib],
      queue: queue)
    source.setEventHandler { [weak self] in
      guard let self else { return }
      let events = source.data
      if events.contains(.delete) || events.contains(.rename) {
        source.cancel()
        self.source = nil
        self.queue.asyncAfter(deadline: .now() + 0.1) { self.start() }
      }
      self.schedule()
    }
    source.setCancelHandler { close(descriptor) }
    self.source = source
    source.resume()
  }

  private func schedule() {
    pending?.cancel()
    let item = DispatchWorkItem { [weak self] in
      guard let self, !self.cancelled else { return }
      self.onChange()
    }
    pending = item
    queue.asyncAfter(deadline: .now() + 0.12, execute: item)
  }
}

/// Subsequence matching with a score that favours word starts and consecutive runs, for the
/// command palette.
public enum FuzzyMatch {
  public static func score(_ query: String, in candidate: String) -> Int? {
    let query = Array(query.lowercased().filter { !$0.isWhitespace })
    if query.isEmpty { return 0 }
    let text = Array(candidate.lowercased())
    let original = Array(candidate)
    var score = 0
    var queryIndex = 0
    var previous = -2
    for (index, character) in text.enumerated() where queryIndex < query.count {
      guard character == query[queryIndex] else { continue }
      var points = 1
      if index == previous + 1 { points += 5 }
      if index == 0 {
        points += 8
      } else {
        let before = original[index - 1]
        if before == "/" || before == " " || before == "-" || before == "_" || before == "." {
          points += 6
        } else if original[index].isUppercase, before.isLowercase {
          points += 4
        }
      }
      score += points
      previous = index
      queryIndex += 1
    }
    guard queryIndex == query.count else { return nil }
    return score * 100 / max(text.count, 1) + score * 10
  }
}

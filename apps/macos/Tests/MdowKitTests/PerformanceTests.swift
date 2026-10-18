import AppKit
import Testing

@testable import MdowKit

/// Guards for things that made the reader slow: a reload loop, slow string searches, and
/// replacing the whole text when only images or diagrams changed.
@Suite struct PerformanceTests {
  /// Reading a file updates its access time. The watcher used to react to that, so every reload
  /// set off the next one and the document re-rendered several times a second.
  @Test func readingAWatchedFileDoesNotReloadIt() async throws {
    let url = FileManager.default.temporaryDirectory
      .appendingPathComponent("mdow-watch-\(UUID().uuidString).md")
    try "# One\n".write(to: url, atomically: false, encoding: .utf8)
    defer { try? FileManager.default.removeItem(at: url) }
    let changes = Counter()
    let watcher = FileWatcher(url: url) { changes.increment() }
    defer { watcher.cancel() }
    try await Task.sleep(for: .milliseconds(200))

    for _ in 0..<3 { _ = try Data(contentsOf: url) }
    try await Task.sleep(for: .milliseconds(400))
    #expect(changes.value == 0)

    let handle = try FileHandle(forWritingTo: url)
    try handle.seekToEnd()
    try handle.write(contentsOf: Data("More\n".utf8))
    try handle.close()
    try await Task.sleep(for: .milliseconds(400))
    #expect(changes.value == 1)
  }

  @Test func byteSearchMatchesFoundation() {
    let text = "Some text with a footnote[^1] and money $5, café [^x]"
    for needle in ["[^", "$", "café", "[^x]", "missing", "t"] {
      #expect(text.containsBytes(needle) == text.contains(needle))
      #expect(text.rangeOfBytes(needle) == text.range(of: needle))
    }
    #expect(text.dropFirst(30).rangeOfBytes("[^") == text.dropFirst(30).range(of: "[^"))
  }

  /// Images and diagrams carry slot numbers, so a render with them loaded can replace just their
  /// placeholders. Both renders must number them the same way.
  @Test func assetSlotsLineUpAcrossRenders() {
    let markdown = """
      # Doc

      ![Logo](logo.png) and ![](badge.svg)

      ```mermaid
      graph LR; A-->B
      ```

      Text after, with $x^2$ math.
      """
    let source = MarkdownSource(markdown)
    let style = ReaderStyle()
    let pending = MarkdownRenderer.render(source, style: style, baseURL: nil, assets: .none)
    let image = NSImage(size: NSSize(width: 40, height: 20), flipped: false) { _ in true }
    let ready = RenderAssets(image: { _ in .ready(image) }, diagram: { _ in .ready(image) })
    let loaded = MarkdownRenderer.render(source, style: style, baseURL: nil, assets: ready)

    func slots(_ text: NSAttributedString) -> [Int: NSRange] {
      var slots: [Int: NSRange] = [:]
      text.enumerateAttribute(.mdowAssetSlot, in: NSRange(location: 0, length: text.length)) {
        value, range, _ in
        if let slot = value as? Int { slots[slot] = slots[slot].map { NSUnionRange($0, range) } ?? range }
      }
      return slots
    }
    func outside(_ text: NSAttributedString, _ slots: [Int: NSRange]) -> String {
      let rest = NSMutableString(string: text.string)
      for range in slots.values.sorted(by: { $0.location > $1.location }) {
        rest.deleteCharacters(in: range)
      }
      return rest as String
    }
    let before = slots(pending.text)
    let after = slots(loaded.text)
    // Two images, the diagram and the formula.
    #expect(Set(before.keys) == [1, 2, 3, 4])
    #expect(Set(after.keys) == Set(before.keys))
    #expect(pending.text.attributedSubstring(from: before[1]!).string.hasPrefix("Logo"))
    #expect(loaded.text.attributedSubstring(from: after[1]!).string.hasPrefix("\u{FFFC}"))
    // Everything outside the slots is the same text.
    let outsideBefore = outside(pending.text, before)
    let outsideAfter = outside(loaded.text, after)
    #expect(outsideBefore == outsideAfter)
  }
}

private final class Counter: @unchecked Sendable {
  private let lock = NSLock()
  private var count = 0

  var value: Int { lock.withLock { count } }
  func increment() { lock.withLock { count += 1 } }
}

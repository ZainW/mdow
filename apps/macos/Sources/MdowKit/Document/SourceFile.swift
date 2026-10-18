import Foundation

/// The kinds of file Mdow opens.
public enum FileKind: Sendable, Equatable {
  case markdown
  case html

  public static let markdownExtensions: Set<String> = ["md", "markdown", "mdx", "mdown", "mkd"]
  public static let htmlExtensions: Set<String> = ["html", "htm"]

  public init?(url: URL) {
    let ext = url.pathExtension.lowercased()
    if Self.markdownExtensions.contains(ext) {
      self = .markdown
    } else if Self.htmlExtensions.contains(ext) {
      self = .html
    } else {
      return nil
    }
  }

  public static func isOpenable(_ url: URL) -> Bool { FileKind(url: url) != nil }
}

/// A markdown file split into its frontmatter and body.
public struct MarkdownSource: Sendable, Equatable {
  public var body: String
  public var frontmatter: String?
  /// `title:` from the frontmatter, if there is one.
  public var title: String?

  public init(_ text: String) {
    var text = text
    if text.hasPrefix("\u{FEFF}") { text.removeFirst() }
    text = text.replacingOccurrences(of: "\r\n", with: "\n")
    guard let (frontmatter, body) = Self.splitFrontmatter(text) else {
      self.body = text
      return
    }
    self.body = body
    self.frontmatter = frontmatter
    self.title = Self.title(in: frontmatter)
  }

  static func splitFrontmatter(_ text: String) -> (String, String)? {
    guard text.hasPrefix("---\n") else { return nil }
    let rest = text.dropFirst(4)
    var offset = rest.startIndex
    while offset < rest.endIndex {
      let lineEnd = rest[offset...].firstIndex(of: "\n") ?? rest.endIndex
      let line = rest[offset..<lineEnd]
      if line == "---" || line == "..." {
        let body = lineEnd < rest.endIndex ? rest[rest.index(after: lineEnd)...] : ""
        return (String(rest[rest.startIndex..<offset]), String(body))
      }
      offset = lineEnd < rest.endIndex ? rest.index(after: lineEnd) : rest.endIndex
    }
    return nil
  }

  static func title(in frontmatter: String) -> String? {
    for line in frontmatter.split(separator: "\n") where line.hasPrefix("title:") {
      var value = line.dropFirst(6).trimmingCharacters(in: .whitespaces)
      if value.count >= 2, let first = value.first, first == value.last, first == "\"" || first == "'" {
        value = String(value.dropFirst().dropLast())
      }
      return value.isEmpty ? nil : value
    }
    return nil
  }
}

/// Footnote definitions (`[^id]: text`) pulled out of the body before parsing, since
/// swift-markdown doesn't parse footnotes.
public struct Footnotes: Sendable, Equatable {
  public struct Note: Sendable, Equatable {
    public var id: String
    public var text: String
    public var number: Int
  }

  public var notes: [Note] = []

  public func number(for id: String) -> Int? { notes.first { $0.id == id }?.number }

  /// Returns the body without definitions, plus the definitions numbered by first reference.
  public static func extract(from body: String) -> (String, Footnotes) {
    guard body.containsBytes("[^") else { return (body, Footnotes()) }
    var kept: [Substring] = []
    var definitions: [(String, String)] = []
    var inFence = false
    var current: (String, [Substring])?

    func finish() {
      if let (id, lines) = current {
        definitions.append((id, lines.joined(separator: "\n")))
      }
      current = nil
    }

    for line in body.split(separator: "\n", omittingEmptySubsequences: false) {
      let trimmed = line.drop { $0 == " " }
      if trimmed.hasPrefix("```") || trimmed.hasPrefix("~~~") { inFence.toggle() }
      if !inFence, line.hasPrefix("[^"), let close = line.firstIndex(of: "]"),
        line.index(after: close) < line.endIndex, line[line.index(after: close)] == ":"
      {
        finish()
        let id = String(line[line.index(line.startIndex, offsetBy: 2)..<close])
        let text = line[line.index(close, offsetBy: 2)...].drop { $0 == " " }
        current = (id, [text])
        continue
      }
      if current != nil {
        if line.hasPrefix("    ") || line.hasPrefix("\t") {
          current?.1.append(line.drop { $0 == " " || $0 == "\t" })
          continue
        }
        if line.isEmpty {
          current?.1.append("")
          continue
        }
        finish()
      }
      kept.append(line)
    }
    finish()
    guard !definitions.isEmpty else { return (body, Footnotes()) }

    let stripped = kept.joined(separator: "\n")
    // Number notes in the order they're first referenced, like GitHub.
    var order: [String] = []
    var search = stripped[...]
    while let start = search.rangeOfBytes("[^") {
      guard let close = search[start.upperBound...].firstIndex(of: "]") else { break }
      let id = String(search[start.upperBound..<close])
      if !order.contains(id), definitions.contains(where: { $0.0 == id }) { order.append(id) }
      search = search[close...]
    }
    for (id, _) in definitions where !order.contains(id) { order.append(id) }
    var notes: [Note] = []
    for (index, id) in order.enumerated() {
      guard let text = definitions.first(where: { $0.0 == id })?.1 else { continue }
      notes.append(Note(id: id, text: text.trimmingCharacters(in: .newlines), number: index + 1))
    }
    return (stripped, Footnotes(notes: notes))
  }
}

/// GitHub-style heading anchors: lowercase, punctuation dropped, spaces to hyphens, and
/// `-1`, `-2`... for repeats.
public struct Slugger: Sendable {
  private var seen: [String: Int] = [:]

  public init() {}

  public static func slug(_ text: String) -> String {
    var out = ""
    for scalar in text.lowercased().unicodeScalars {
      if CharacterSet.alphanumerics.contains(scalar) || scalar == "-" || scalar == "_" {
        out.unicodeScalars.append(scalar)
      } else if scalar == " " {
        out.append("-")
      }
    }
    return out
  }

  public mutating func next(_ text: String) -> String {
    let base = Self.slug(text)
    if let count = seen[base] {
      seen[base] = count + 1
      let candidate = "\(base)-\(count)"
      seen[candidate] = 1
      return candidate
    }
    seen[base] = 1
    return base
  }
}

extension StringProtocol {
  /// Whether the text contains `needle`, compared byte for byte. Much faster than Foundation's
  /// `contains(_:)` on long strings, which bridges the whole string first.
  public func containsBytes(_ needle: String) -> Bool {
    rangeOfBytes(needle) != nil
  }

  /// The first range of `needle`, compared byte for byte (see `containsBytes`).
  public func rangeOfBytes(_ needle: String) -> Range<Index>? {
    let bytes = Array(needle.utf8)
    guard let first = bytes.first else { return nil }
    let haystack = utf8
    var start = haystack.startIndex
    while let index = haystack[start...].firstIndex(of: first) {
      if haystack[index...].starts(with: bytes) {
        return index..<haystack.index(index, offsetBy: bytes.count)
      }
      start = haystack.index(after: index)
    }
    return nil
  }
}

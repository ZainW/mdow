import Foundation

/// The pure parts of the companion service, mirroring the desktop's `service.ts`,
/// `opencode-binary.ts` and `opencode-server.ts`.
public enum CompanionLogic {
  public static let instructions = """
    You are the writing companion inside Mdow, a calm reader for Markdown and HTML documents. The user is reading the documents in this folder and wants help understanding and improving them.

    - Answer questions about the documents. Read them with your tools instead of guessing, and mention documents by their path relative to the folder.
    - When the user asks for a change, make it with your edit or write tool right away. Mdow shows the user every change as a diff to approve or reject before it is written, so do not ask for confirmation first.
    - Keep edits focused. Preserve each document's voice, structure and formatting, whether it is Markdown or HTML.
    - Do not run shell commands, reach outside this folder, or create new files unless the user asks for one.
    - Reply concisely in Markdown.
    """

  /// Shell commands and other folders are off limits; edits always come back for review.
  public static let sessionPermissions: [OpencodePermissionRule] = [
    .init(action: "shell", resource: "*", effect: .deny),
    .init(action: "bash", resource: "*", effect: .deny),
    .init(action: "edit", resource: "*", effect: .ask),
  ]

  /// Mdow talks to the v2 HTTP API (`opencode serve`), which shipped in OpenCode 2.0.
  public static let minimumMajorVersion = 2

  /// The open folder, else the document's folder, else home.
  public static func sessionDirectory(document: URL?, folder: URL?, home: String = NSHomeDirectory())
    -> String
  {
    if let folder { return folder.path }
    if let document { return document.deletingLastPathComponent().path }
    return home
  }

  static func displayPath(_ path: String, in directory: String) -> String {
    let base = directory.hasSuffix("/") ? directory : directory + "/"
    if path.hasPrefix(base), path.count > base.count { return String(path.dropFirst(base.count)) }
    return path
  }

  /// Prefixes the user's message with what they are looking at in Mdow.
  public static func composePrompt(
    text: String, activePath: String?, tags: [CompanionContextTag], directory: String
  ) -> String {
    var lines: [String] = []
    if let activePath { lines.append("Viewing: \(displayPath(activePath, in: directory))") }
    let attached = tags.map { displayPath($0.path, in: directory) }
    if !attached.isEmpty { lines.append("Attached: \(attached.joined(separator: ", "))") }
    if lines.isEmpty { return text }
    return "<mdow-context>\n\(lines.joined(separator: "\n"))\n</mdow-context>\n\n\(text)"
  }

  /// Model options without deprecated models, sorted by provider then name.
  public static func modelOptions(models: [OpencodeModelInfo], providers: [OpencodeProviderInfo])
    -> [CompanionModelOption]
  {
    var names: [String: String] = [:]
    for provider in providers { names[provider.id] = provider.name ?? provider.id }
    return models.filter { $0.status != "deprecated" }
      .map {
        CompanionModelOption(
          value: "\($0.providerID)/\($0.id)", name: $0.name ?? $0.id, providerId: $0.providerID,
          providerName: names[$0.providerID] ?? $0.providerID)
      }
      .sorted {
        let provider = $0.providerName.localizedCompare($1.providerName)
        if provider != .orderedSame { return provider == .orderedAscending }
        return $0.name.localizedCompare($1.name) == .orderedAscending
      }
  }

  // MARK: Binary and server

  /// Apps launched from Finder get a minimal PATH, so the usual install locations are searched
  /// as well as whatever PATH the app inherited (and the login shell's, when known).
  public static func searchDirectories(path: String?, home: String, loginShellPath: String? = nil)
    -> [String]
  {
    let inherited = (path ?? "").split(separator: ":").map(String.init)
    let shell = (loginShellPath ?? "").split(separator: ":").map(String.init)
    let wellKnown = [
      "\(home)/.opencode/bin", "\(home)/.local/bin", "\(home)/.bun/bin", "\(home)/.npm-global/bin",
      "/opt/homebrew/bin", "/usr/local/bin", "/usr/bin",
    ]
    var seen = Set<String>()
    return (inherited + shell + wellKnown).filter { !$0.isEmpty && seen.insert($0).inserted }
  }

  public static func parseVersion(_ output: String) -> String? {
    guard let match = output.firstMatch(of: /(\d+)\.(\d+)\.(\d+)/) else { return nil }
    return String(match.output.0)
  }

  public static func isSupportedVersion(_ version: String) -> Bool {
    guard let major = Int(version.split(separator: ".").first ?? "") else { return false }
    return major >= minimumMajorVersion
  }

  public static func parseListeningURL(_ output: String) -> URL? {
    guard let match = output.firstMatch(of: /(?i)listening on (https?:\/\/[^\s]+)/) else { return nil }
    return URL(string: String(match.output.1))
  }

  /// A login shell prints its PATH between these markers, so rc-file output can't confuse it.
  public static let pathMarker = "__MDOW_PATH__"

  public static func parseShellPath(_ output: String) -> String? {
    let parts = output.components(separatedBy: pathMarker)
    guard parts.count >= 3 else { return nil }
    let path = parts[1].trimmingCharacters(in: .whitespacesAndNewlines)
    return path.isEmpty ? nil : path
  }

  /// A fresh per-launch password for the private server, never written to disk.
  public static func makePassword() -> String {
    var generator = SystemRandomNumberGenerator()
    return (0..<24).map { _ in String(format: "%02x", UInt8.random(in: .min ... .max, using: &generator)) }
      .joined()
  }
}

// MARK: Unified diff

public struct DiffLine: Sendable, Equatable {
  public enum Kind: Sendable { case context, add, remove }
  public var kind: Kind
  public var text: String
  public var oldLine: Int?
  public var newLine: Int?

  public init(kind: Kind, text: String, oldLine: Int?, newLine: Int?) {
    self.kind = kind
    self.text = text
    self.oldLine = oldLine
    self.newLine = newLine
  }
}

public struct DiffHunk: Sendable, Equatable {
  public var header: String
  public var lines: [DiffLine]

  public init(header: String, lines: [DiffLine]) {
    self.header = header
    self.lines = lines
  }
}

public enum UnifiedDiff {
  /// Parses the hunks of a unified diff, skipping file headers and "no newline" markers.
  public static func parse(_ patch: String) -> [DiffHunk] {
    var hunks: [DiffHunk] = []
    var oldLine = 0
    var newLine = 0
    let header = /^@@ -(\d+)(?:,\d+)? \+(\d+)(?:,\d+)? @@(.*)$/
    for raw in patch.replacingOccurrences(of: "\r\n", with: "\n").split(
      separator: "\n", omittingEmptySubsequences: false)
    {
      if let match = raw.wholeMatch(of: header) {
        oldLine = Int(match.output.1) ?? 0
        newLine = Int(match.output.2) ?? 0
        hunks.append(DiffHunk(header: match.output.3.trimmingCharacters(in: .whitespaces), lines: []))
        continue
      }
      guard !hunks.isEmpty, !raw.hasPrefix("\\"), let marker = raw.first else { continue }
      let text = String(raw.dropFirst())
      switch marker {
      case "+":
        hunks[hunks.count - 1].lines.append(.init(kind: .add, text: text, oldLine: nil, newLine: newLine))
        newLine += 1
      case "-":
        hunks[hunks.count - 1].lines.append(.init(kind: .remove, text: text, oldLine: oldLine, newLine: nil))
        oldLine += 1
      case " ":
        hunks[hunks.count - 1].lines.append(
          .init(kind: .context, text: text, oldLine: oldLine, newLine: newLine))
        oldLine += 1
        newLine += 1
      default:
        break
      }
    }
    return hunks.filter { !$0.lines.isEmpty }
  }
}

// MARK: Assistant markdown

/// A block of an assistant reply, for simple native rendering.
public enum ReplyBlock: Sendable, Equatable {
  case paragraph(String)
  case heading(level: Int, text: String)
  case code(language: String, text: String)
}

public enum ReplyBlocks {
  /// Splits markdown into paragraphs (blank-line separated, line breaks kept), ATX headings and
  /// fenced code. Inline markdown is left for `AttributedString` to style.
  public static func split(_ markdown: String) -> [ReplyBlock] {
    var blocks: [ReplyBlock] = []
    var paragraph: [Substring] = []
    var fence: (marker: String, language: String, lines: [Substring])?

    func flush() {
      let text = paragraph.joined(separator: "\n").trimmingCharacters(in: .newlines)
      if !text.isEmpty { blocks.append(.paragraph(text)) }
      paragraph.removeAll()
    }

    for line in markdown.split(separator: "\n", omittingEmptySubsequences: false) {
      let trimmed = line.drop { $0 == " " }
      if var open = fence {
        if trimmed.hasPrefix(open.marker), trimmed.trimmingCharacters(in: .whitespaces).allSatisfy({
          String($0) == String(open.marker.first!)
        }) {
          blocks.append(.code(language: open.language, text: open.lines.joined(separator: "\n")))
          fence = nil
        } else {
          open.lines.append(line)
          fence = open
        }
        continue
      }
      if trimmed.hasPrefix("```") || trimmed.hasPrefix("~~~") {
        flush()
        let marker = String(trimmed.prefix { $0 == trimmed.first })
        let language = trimmed.dropFirst(marker.count).trimmingCharacters(in: .whitespaces)
        fence = (marker, language, [])
        continue
      }
      if trimmed.isEmpty {
        flush()
        continue
      }
      if trimmed.first == "#" {
        let hashes = trimmed.prefix { $0 == "#" }.count
        let rest = trimmed.dropFirst(hashes)
        if hashes <= 6, rest.isEmpty || rest.first == " " {
          flush()
          blocks.append(.heading(level: hashes, text: rest.trimmingCharacters(in: .whitespaces)))
          continue
        }
      }
      paragraph.append(line)
    }
    if let open = fence {
      // Still streaming: show the unfinished block as code.
      blocks.append(.code(language: open.language, text: open.lines.joined(separator: "\n")))
    }
    flush()
    return blocks
  }
}

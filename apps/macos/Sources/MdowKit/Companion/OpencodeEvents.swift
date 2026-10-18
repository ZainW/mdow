import Foundation

// Event-stream parsing and translation, ported line for line from the desktop app's
// `opencode-client.ts` (SSE) and `opencode-events.ts` (turn translator).

/// One server event: `{ id, type, data }`.
public struct OpencodeEvent: Sendable, Equatable {
  public var id: String?
  public var type: String
  public var data: [String: JSONValue]

  public init(id: String? = nil, type: String, data: [String: JSONValue] = [:]) {
    self.id = id
    self.type = type
    self.data = data
  }

  public static func parse(_ text: String) -> OpencodeEvent? {
    guard let json = JSONValue.parse(text), let type = json["type"]?.string else { return nil }
    return OpencodeEvent(id: json["id"]?.string, type: type, data: json["data"]?.object ?? [:])
  }
}

/// Splits a server-sent event stream into `data:` payloads.
public struct SSEParser: Sendable {
  private var buffer = ""
  private var dataLines: [String] = []

  public init() {}

  /// Feeds raw text, which may end mid-line. Returns the payloads it completed.
  public mutating func feed(_ chunk: String) -> [String] {
    buffer += chunk
    var payloads: [String] = []
    // "\r\n" is a single Character in Swift, so look for both.
    while let newline = buffer.firstIndex(where: { $0 == "\n" || $0 == "\r\n" }) {
      let line = String(buffer[buffer.startIndex..<newline])
      buffer = String(buffer[buffer.index(after: newline)...])
      if let payload = feed(line: line) { payloads.append(payload) }
    }
    return payloads
  }

  /// Feeds one line without its newline. Returns a payload when the line ends an event.
  public mutating func feed(line raw: String) -> String? {
    let line = raw.hasSuffix("\r") ? String(raw.dropLast()) : raw
    if line.isEmpty {
      defer { dataLines = [] }
      return dataLines.isEmpty ? nil : dataLines.joined(separator: "\n")
    }
    if line.hasPrefix("data:") {
      var value = line.dropFirst(5)
      if value.first == " " { value = value.dropFirst() }
      dataLines.append(String(value))
    }
    return nil
  }
}

// MARK: Translation

public struct TurnFinish: Sendable, Equatable {
  public enum Outcome: String, Sendable { case succeeded, failed, interrupted }
  public var outcome: Outcome
  public var error: String?

  public init(_ outcome: Outcome, error: String? = nil) {
    self.outcome = outcome
    self.error = error
  }
}

public struct TranslateResult: Sendable, Equatable {
  public var updates: [CompanionUpdate] = []
  /// A permission Mdow declines on the user's behalf, such as running shell commands.
  public var autoReject: (permissionId: String, reason: String)?
  public var finish: TurnFinish?
  /// OpenCode began a run; the run's events and its finish belong to the newest prompt.
  public var started = false

  public init(
    updates: [CompanionUpdate] = [], autoReject: (permissionId: String, reason: String)? = nil,
    finish: TurnFinish? = nil, started: Bool = false
  ) {
    self.updates = updates
    self.autoReject = autoReject
    self.finish = finish
    self.started = started
  }

  public static func == (a: TranslateResult, b: TranslateResult) -> Bool {
    a.updates == b.updates && a.autoReject?.permissionId == b.autoReject?.permissionId
      && a.autoReject?.reason == b.autoReject?.reason && a.finish == b.finish && a.started == b.started
  }
}

enum CompanionPath {
  static func basename(_ path: String) -> String { (path as NSString).lastPathComponent }

  static func resolve(_ path: String, in directory: String) -> String {
    if path.hasPrefix("/") { return path }
    let base = URL(fileURLWithPath: directory, isDirectory: true)
    return URL(fileURLWithPath: path, relativeTo: base).standardizedFileURL.path
  }

  static func isDocument(_ path: String) -> Bool {
    FileKind.isOpenable(URL(fileURLWithPath: path))
  }
}

private let maxDetailChars = 4_000

/// Tools that change files. Mdow shows these as reviewable change cards, not tool rows.
private let writeTools: Set<String> = ["edit", "write", "patch", "apply_patch", "multiedit"]

private func clip(_ text: String) -> String {
  text.count > maxDetailChars ? String(text.prefix(maxDetailChars)) + "\n…" : text
}

private func formatInput(_ input: JSONValue?) -> String? {
  guard let input else { return nil }
  if case .string(let text) = input { return clip(text) }
  return clip(input.prettyPrinted)
}

private func textFromContent(_ content: JSONValue?) -> String {
  guard let blocks = content?.array else { return content?.string ?? "" }
  return blocks.compactMap { $0["text"]?.string }.joined(separator: "\n")
}

private func inputPath(_ input: JSONValue?) -> String? {
  guard let input, input.isObject else { return nil }
  return input["path"]?.string ?? input["filePath"]?.string ?? input["file"]?.string
}

/// A plain-words title for a tool call, such as "Read notes.md".
public func opencodeToolTitle(_ name: String, input: JSONValue? = nil) -> String {
  let path = inputPath(input)
  switch name {
  case "read":
    return path.map { "Read \(CompanionPath.basename($0))" } ?? "Read a document"
  case "grep", "search":
    if let pattern = input?["pattern"]?.string ?? input?["query"]?.string {
      return "Searched for “\(pattern)”"
    }
    return "Searched the folder"
  case "glob", "list", "ls", "find":
    return "Looked through the folder"
  case "webfetch":
    return input?["url"]?.string.map { "Fetched \($0)" } ?? "Fetched a page"
  case "websearch":
    return input?["query"]?.string.map { "Searched the web for “\($0)”" } ?? "Searched the web"
  case "todowrite", "todoread":
    return "Updated its plan"
  default:
    return name.prefix(1).uppercased() + name.dropFirst()
  }
}

/// The files in an edit permission's or tool result's `metadata.files`.
public func opencodeFileChanges(_ metadata: JSONValue?, directory: String) -> [CompanionFileChange] {
  guard let files = metadata?["files"]?.array else { return [] }
  return files.compactMap { entry in
    guard entry.isObject, let file = entry["file"]?.string ?? entry["path"]?.string else { return nil }
    let status: CompanionFileStatus =
      switch entry["status"]?.string {
      case "added": .added
      case "deleted": .deleted
      default: .modified
      }
    return CompanionFileChange(
      path: CompanionPath.resolve(file, in: directory), displayPath: file,
      patch: entry["patch"]?.string ?? "", additions: Int(entry["additions"]?.number ?? 0),
      deletions: Int(entry["deletions"]?.number ?? 0), status: status)
  }
}

/// Translates the OpenCode event stream for one session into companion updates. One instance
/// lives as long as the session, so tool names and pending permissions carry across turns.
public final class OpencodeTurnTranslator {
  public let sessionId: String
  public let directory: String
  private var toolNames: [String: String] = [:]
  private var toolInputs: [String: JSONValue] = [:]
  private var permissionTools: [String: String] = [:]
  private var rejectedPermissions: Set<String> = []
  private var cited: Set<String> = []
  private var reasoningOpen = false

  public init(sessionId: String, directory: String) {
    self.sessionId = sessionId
    self.directory = directory
  }

  /// Clears per-turn state; call before each prompt.
  public func beginTurn() {
    cited.removeAll()
    reasoningOpen = false
  }

  public func translate(_ event: OpencodeEvent) -> TranslateResult {
    let data = event.data
    guard data["sessionID"]?.string == sessionId else { return TranslateResult() }

    switch event.type {
    case "session.text.delta":
      guard let delta = data["delta"]?.string, !delta.isEmpty else { return TranslateResult() }
      return TranslateResult(updates: closeReasoning() + [.delta(delta)])
    case "session.reasoning.delta":
      guard let delta = data["delta"]?.string, !delta.isEmpty else { return TranslateResult() }
      reasoningOpen = true
      return TranslateResult(updates: [.thinking(delta)])
    case "session.reasoning.ended":
      return TranslateResult(updates: closeReasoning())
    case "session.tool.input.started":
      return toolStarted(data["id"]?.string, data["name"]?.string)
    case "session.tool.called":
      return toolCalled(data["id"]?.string, data["input"])
    case "session.tool.success":
      return toolSucceeded(data["id"]?.string, data["content"], data["metadata"])
    case "session.tool.failed":
      return toolFailed(data["id"]?.string, data["error"])
    case "permission.asked":
      return permissionAsked(data)
    case "permission.replied":
      return permissionReplied(data["requestID"]?.string, data["reply"]?.string)
    case "session.execution.started":
      return TranslateResult(started: true)
    case "session.execution.succeeded":
      return TranslateResult(updates: closeReasoning(), finish: TurnFinish(.succeeded))
    case "session.execution.failed":
      let error = data["error"]?["message"]?.string
      return TranslateResult(
        updates: closeReasoning(),
        finish: TurnFinish(.failed, error: error ?? "OpenCode could not finish the response."))
    default:
      if event.type.hasPrefix("session.execution.") {
        return TranslateResult(updates: closeReasoning(), finish: TurnFinish(.interrupted))
      }
      return TranslateResult()
    }
  }

  private func closeReasoning() -> [CompanionUpdate] {
    guard reasoningOpen else { return [] }
    reasoningOpen = false
    return [.thinkingDone]
  }

  private func toolStarted(_ id: String?, _ name: String?) -> TranslateResult {
    guard let id, let name, !id.isEmpty, !name.isEmpty else { return TranslateResult() }
    toolNames[id] = name
    if writeTools.contains(name) { return TranslateResult(updates: closeReasoning()) }
    return TranslateResult(
      updates: closeReasoning() + [
        .tool(CompanionToolPart(toolCallId: id, name: opencodeToolTitle(name), state: .pending))
      ])
  }

  private func toolCalled(_ id: String?, _ input: JSONValue?) -> TranslateResult {
    guard let id, !id.isEmpty else { return TranslateResult() }
    let name = toolNames[id] ?? "tool"
    if writeTools.contains(name) { return TranslateResult() }
    if let input { toolInputs[id] = input }
    var updates: [CompanionUpdate] = [
      .tool(
        CompanionToolPart(
          toolCallId: id, name: opencodeToolTitle(name, input: input), state: .running,
          input: formatInput(input)))
    ]
    if name == "read", let path = inputPath(input), CompanionPath.isDocument(path) {
      let absolute = CompanionPath.resolve(path, in: directory)
      if cited.insert(absolute).inserted {
        updates.append(
          .citation(
            CompanionCitation(
              sourceId: "read:\(absolute)", path: absolute, label: CompanionPath.basename(absolute))))
      }
    }
    return TranslateResult(updates: updates)
  }

  private func toolSucceeded(_ id: String?, _ content: JSONValue?, _ metadata: JSONValue?)
    -> TranslateResult
  {
    guard let id, !id.isEmpty else { return TranslateResult() }
    let name = toolNames[id] ?? "tool"
    let files = opencodeFileChanges(metadata, directory: directory)
    if writeTools.contains(name) || !files.isEmpty {
      return TranslateResult(updates: [
        .change(toolCallId: id, status: .applied, files: files.isEmpty ? nil : files)
      ])
    }
    return TranslateResult(updates: [
      .tool(
        CompanionToolPart(
          toolCallId: id, name: opencodeToolTitle(name, input: toolInputs[id]), state: .completed,
          output: clip(textFromContent(content))))
    ])
  }

  private func toolFailed(_ id: String?, _ error: JSONValue?) -> TranslateResult {
    guard let id, !id.isEmpty else { return TranslateResult() }
    let name = toolNames[id] ?? "tool"
    let message = (error?["message"]?.string ?? error?.string) ?? "Tool failed"
    if writeTools.contains(name) {
      let rejected = permissionTools.contains { permissionId, toolId in
        toolId == id && rejectedPermissions.contains(permissionId)
      }
      return TranslateResult(updates: [
        rejected
          ? .change(toolCallId: id, status: .rejected)
          : .change(toolCallId: id, status: .failed, error: message)
      ])
    }
    return TranslateResult(updates: [
      .tool(
        CompanionToolPart(
          toolCallId: id, name: opencodeToolTitle(name, input: toolInputs[id]), state: .error,
          error: message))
    ])
  }

  private func permissionAsked(_ data: [String: JSONValue]) -> TranslateResult {
    guard let permissionId = data["id"]?.string, !permissionId.isEmpty else { return TranslateResult() }
    let action = data["action"]?.string ?? "unknown"
    let toolCallId = data["source"]?["id"]?.string ?? permissionId
    permissionTools[permissionId] = toolCallId

    if action == "edit" || action == "write" {
      let files = opencodeFileChanges(data["metadata"], directory: directory)
      return TranslateResult(
        updates: closeReasoning() + [
          .change(toolCallId: toolCallId, permissionId: permissionId, status: .pending, files: files)
        ])
    }

    // Mdow is a document companion: anything beyond reading and editing documents in the
    // open folder (shell commands, other folders, the network) is declined automatically.
    let resources = data["resources"]?.array?.compactMap(\.string) ?? []
    let target = resources.isEmpty ? "" : " (\(resources.joined(separator: ", ")))"
    return TranslateResult(
      autoReject: (
        permissionId,
        "Mdow does not allow the \(action) permission\(target). Work only with the documents in this folder."
      ))
  }

  private func permissionReplied(_ permissionId: String?, _ reply: String?) -> TranslateResult {
    guard let permissionId, let toolCallId = permissionTools[permissionId], reply == "reject" else {
      return TranslateResult()
    }
    rejectedPermissions.insert(permissionId)
    if let name = toolNames[toolCallId], !writeTools.contains(name) { return TranslateResult() }
    return TranslateResult(updates: [.change(toolCallId: toolCallId, status: .rejected)])
  }
}

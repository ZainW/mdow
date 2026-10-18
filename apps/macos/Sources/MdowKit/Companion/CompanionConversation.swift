import Foundation

/// The conversation shown in the companion pane: messages and the parts they stream in.
/// Ported from the desktop app's `companion-slice.ts`.
public struct CompanionConversation: Sendable, Equatable {
  public private(set) var messages: [CompanionMessage] = []
  public private(set) var streaming = false
  public var error: String?
  public private(set) var warnings: [String] = []
  public private(set) var tags: [CompanionContextTag] = []
  private var streamingAssistantId: String?

  public init() {}

  // MARK: Tags

  public mutating func addTag(_ tag: CompanionContextTag) {
    guard !tags.contains(where: { $0.sourceId == tag.sourceId }) else { return }
    tags.append(tag)
  }

  public mutating func removeTag(_ sourceId: String) {
    tags.removeAll { $0.sourceId == sourceId }
  }

  // MARK: Turns

  public mutating func appendUserMessage(_ text: String) {
    messages.append(CompanionMessage(role: .user, content: text, status: .complete))
    error = nil
  }

  public mutating func beginRequest() {
    guard !streaming else { return }
    let id = ensureAssistant()
    streaming = true
    error = nil
    mapAssistant(id) { $0.status = .streaming }
  }

  public mutating func cancelRequest() {
    streamingAssistantId = nil
    streaming = false
    endTurn(.cancelled)
  }

  public mutating func reset() {
    streamingAssistantId = nil
    messages = []
    streaming = false
    warnings = []
    error = nil
  }

  public mutating func dismissWarnings() { warnings = [] }

  /// The change card waiting on `permissionId`, if any.
  public func change(permissionId: String) -> CompanionChangePart? {
    for message in messages {
      for case .change(let change) in message.parts where change.permissionId == permissionId {
        return change
      }
    }
    return nil
  }

  /// Hides the review buttons while a decision is sent (`keepId` false), or restores them when
  /// sending fails. OpenCode confirms with applied or rejected events.
  public mutating func markReviewed(
    toolCallId: String, status: CompanionChangeStatus, permissionId: String?
  ) {
    for messageIndex in messages.indices {
      for partIndex in messages[messageIndex].parts.indices {
        guard case .change(var change) = messages[messageIndex].parts[partIndex],
          change.toolCallId == toolCallId
        else { continue }
        change.status = status
        change.permissionId = permissionId
        messages[messageIndex].parts[partIndex] = .change(change)
      }
    }
  }

  /// Documents with an edit waiting for a decision in the latest response.
  public var pendingReviewPaths: [String] {
    guard let last = messages.last, last.role == .assistant else { return [] }
    var paths: [String] = []
    for case .change(let change) in last.parts where change.awaitingReview {
      for file in change.files where !paths.contains(file.path) { paths.append(file.path) }
    }
    return paths
  }

  /// The change in the latest response that is waiting for a decision.
  public var pendingChange: CompanionChangePart? {
    guard let last = messages.last, last.role == .assistant else { return nil }
    for case .change(let change) in last.parts.reversed() where change.awaitingReview {
      return change
    }
    return nil
  }

  public mutating func apply(_ update: CompanionUpdate) {
    switch update {
    case .delta(let text):
      let id = ensureAssistant()
      streaming = true
      mapAssistant(id) { message in
        message.parts = Self.upsert(message.parts, .text(text))
        message.content = Self.text(of: message.parts)
        message.status = .streaming
      }
    case .thinking(let text):
      let id = ensureAssistant()
      streaming = true
      mapAssistant(id) { message in
        message.parts = Self.upsert(message.parts, .thinking(text: text, done: false))
        message.status = .streaming
      }
    case .thinkingDone:
      guard let id = streamingAssistantId else { return }
      mapAssistant(id) { message in
        message.parts = message.parts.map {
          if case .thinking(let text, _) = $0 { return .thinking(text: text, done: true) }
          return $0
        }
      }
    case .tool(let tool):
      let id = ensureAssistant()
      streaming = true
      mapAssistant(id) { message in
        message.parts = Self.upsert(message.parts, .tool(tool))
        message.status = .streaming
      }
    case .change(let toolCallId, let permissionId, let status, let files, let error):
      let id = ensureAssistant()
      streaming = true
      let part = CompanionChangePart(
        toolCallId: toolCallId, permissionId: permissionId, status: status, files: files ?? [],
        error: error)
      mapAssistant(id) { message in
        message.parts = Self.upsert(message.parts, .change(part))
        message.status = .streaming
      }
    case .status(let text):
      let id = ensureAssistant()
      mapAssistant(id) { message in message.parts = Self.upsert(message.parts, .status(text)) }
    case .citation(let citation):
      guard let id = streamingAssistantId else { return }
      mapAssistant(id) { message in
        if !message.citations.contains(where: { $0.sourceId == citation.sourceId }) {
          message.citations.append(citation)
        }
      }
    case .warning(let text):
      warnings.append(text)
    case .error(let text):
      streamingAssistantId = nil
      streaming = false
      error = text
      endTurn(.error)
    case .done(let messageId):
      streamingAssistantId = nil
      streaming = false
      endTurn(.complete, messageId: messageId)
    case .cancelled(let messageId):
      streamingAssistantId = nil
      streaming = false
      endTurn(.cancelled, messageId: messageId)
    }
  }

  // MARK: Helpers

  private mutating func ensureAssistant() -> String {
    if let streamingAssistantId { return streamingAssistantId }
    let message = CompanionMessage(role: .assistant, content: "", parts: [], status: .streaming)
    streamingAssistantId = message.id
    messages.append(message)
    return message.id
  }

  private mutating func mapAssistant(_ id: String, _ change: (inout CompanionMessage) -> Void) {
    guard let index = messages.firstIndex(where: { $0.id == id }) else { return }
    change(&messages[index])
  }

  static func text(of parts: [CompanionPart]) -> String {
    parts.compactMap { if case .text(let text) = $0 { return text } else { return nil } }.joined()
  }

  static func upsert(_ parts: [CompanionPart], _ part: CompanionPart) -> [CompanionPart] {
    var parts = parts
    switch part {
    case .text(let text):
      if case .text(let previous) = parts.last {
        parts[parts.count - 1] = .text(previous + text)
      } else {
        parts.append(part)
      }
    case .thinking(let text, let done):
      // Reasoning that resumes right after earlier reasoning reads as one block.
      if case .thinking(let previous, true) = parts.last {
        parts[parts.count - 1] = .thinking(text: "\(previous)\n\n\(text)", done: done)
        return parts
      }
      let index = parts.lastIndex { if case .thinking = $0 { true } else { false } }
      if let index, case .thinking(let existing, false) = parts[index] {
        parts[index] = .thinking(text: existing + text, done: done)
      } else {
        parts.append(part)
      }
    case .tool(let tool):
      let index = parts.firstIndex {
        if case .tool(let existing) = $0 { existing.toolCallId == tool.toolCallId } else { false }
      }
      if let index, case .tool(let existing) = parts[index] {
        var merged = tool
        merged.input = tool.input ?? existing.input
        merged.output = tool.output ?? existing.output
        merged.error = tool.error ?? existing.error
        parts[index] = .tool(merged)
      } else {
        parts.append(part)
      }
    case .change(let change):
      let index = parts.firstIndex {
        if case .change(let existing) = $0 { existing.toolCallId == change.toolCallId } else { false }
      }
      if let index, case .change(var existing) = parts[index] {
        existing.status = change.status
        // The review id only means something while the change waits for a decision.
        existing.permissionId =
          change.status == .pending ? (change.permissionId ?? existing.permissionId) : nil
        if !change.files.isEmpty { existing.files = change.files }
        existing.error = change.error
        parts[index] = .change(existing)
      } else {
        parts.append(part)
      }
    case .status:
      parts.append(part)
    }
    return parts
  }

  /// Settles parts left open when a turn ends early: reasoning closes, reviews expire.
  static func settle(_ parts: [CompanionPart]) -> [CompanionPart] {
    parts.map { part in
      switch part {
      case .thinking(let text, _):
        return .thinking(text: text, done: true)
      case .change(var change) where change.status == .pending:
        change.status = .rejected
        change.permissionId = nil
        return .change(change)
      case .tool(var tool) where tool.state == .pending || tool.state == .running:
        tool.state = .cancelled
        return .tool(tool)
      default:
        return part
      }
    }
  }

  private mutating func endTurn(_ status: CompanionMessageStatus, messageId: String? = nil) {
    for index in messages.indices
    where messages[index].id == messageId || messages[index].status == .streaming {
      messages[index].status = status
      messages[index].parts = Self.settle(messages[index].parts)
    }
  }
}

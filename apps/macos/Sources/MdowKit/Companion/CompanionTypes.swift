import Foundation

// The companion's data model, ported from the desktop app's `shared/types.ts`.

public enum CompanionRuntimeAvailability: String, Sendable {
  case available, missing, outdated, failed
}

public struct CompanionRuntimeStatus: Sendable, Equatable {
  public var availability: CompanionRuntimeAvailability
  /// `opencode --version`, when the binary was found.
  public var version: String?
  public var detail: String?

  public init(availability: CompanionRuntimeAvailability, version: String? = nil, detail: String? = nil) {
    self.availability = availability
    self.version = version
    self.detail = detail
  }
}

public struct CompanionModelOption: Sendable, Equatable, Identifiable, Hashable {
  /// `providerID/modelID`, the form OpenCode uses everywhere.
  public var value: String
  public var name: String
  public var providerId: String
  public var providerName: String
  public var id: String { value }

  public init(value: String, name: String, providerId: String, providerName: String) {
    self.value = value
    self.name = name
    self.providerId = providerId
    self.providerName = providerName
  }
}

public struct CompanionModelState: Sendable, Equatable {
  public var options: [CompanionModelOption]
  public var currentValue: String?
  public var stale: Bool
  public var unavailableReason: String?

  public init(
    options: [CompanionModelOption], currentValue: String?, stale: Bool,
    unavailableReason: String? = nil
  ) {
    self.options = options
    self.currentValue = currentValue
    self.stale = stale
    self.unavailableReason = unavailableReason
  }

  public static let loading = CompanionModelState(
    options: [], currentValue: nil, stale: true, unavailableReason: "Starting OpenCode…")

  public var current: CompanionModelOption? {
    currentValue.flatMap { value in options.first { $0.value == value } }
  }
}

public enum CompanionContextTagKind: String, Sendable { case file, folder }

public struct CompanionContextTag: Sendable, Equatable, Identifiable {
  public var kind: CompanionContextTagKind
  public var path: String
  public var sourceId: String
  public var id: String { sourceId }

  public init(kind: CompanionContextTagKind = .file, path: String, sourceId: String? = nil) {
    self.kind = kind
    self.path = path
    self.sourceId = sourceId ?? "tag:\(path)"
  }
}

public struct CompanionCitation: Sendable, Equatable {
  public var sourceId: String
  public var path: String
  public var label: String

  public init(sourceId: String, path: String, label: String) {
    self.sourceId = sourceId
    self.path = path
    self.label = label
  }
}

public enum CompanionToolState: String, Sendable {
  case pending, running, completed, error, cancelled
}

public enum CompanionFileStatus: String, Sendable { case added, modified, deleted }

public struct CompanionFileChange: Sendable, Equatable {
  /// Absolute path of the changed document.
  public var path: String
  /// Path relative to the session folder, for display.
  public var displayPath: String
  /// Unified diff of the change.
  public var patch: String
  public var additions: Int
  public var deletions: Int
  public var status: CompanionFileStatus

  public init(
    path: String, displayPath: String, patch: String, additions: Int, deletions: Int,
    status: CompanionFileStatus
  ) {
    self.path = path
    self.displayPath = displayPath
    self.patch = patch
    self.additions = additions
    self.deletions = deletions
    self.status = status
  }
}

public enum CompanionChangeStatus: String, Sendable { case pending, applied, rejected, failed }

public struct CompanionToolPart: Sendable, Equatable {
  public var toolCallId: String
  public var name: String
  public var state: CompanionToolState
  public var input: String?
  public var output: String?
  public var error: String?

  public init(
    toolCallId: String, name: String, state: CompanionToolState, input: String? = nil,
    output: String? = nil, error: String? = nil
  ) {
    self.toolCallId = toolCallId
    self.name = name
    self.state = state
    self.input = input
    self.output = output
    self.error = error
  }
}

public struct CompanionChangePart: Sendable, Equatable {
  public var toolCallId: String
  /// Set while OpenCode waits for the user's decision.
  public var permissionId: String?
  public var status: CompanionChangeStatus
  public var files: [CompanionFileChange]
  public var error: String?

  public init(
    toolCallId: String, permissionId: String? = nil, status: CompanionChangeStatus,
    files: [CompanionFileChange] = [], error: String? = nil
  ) {
    self.toolCallId = toolCallId
    self.permissionId = permissionId
    self.status = status
    self.files = files
    self.error = error
  }

  /// OpenCode is waiting for the user to accept or decline.
  public var awaitingReview: Bool { status == .pending && permissionId != nil }
  /// Accepted, and OpenCode has not confirmed the write yet.
  public var applying: Bool { status == .pending && permissionId == nil }
}

public enum CompanionPart: Sendable, Equatable {
  case text(String)
  case thinking(text: String, done: Bool)
  case tool(CompanionToolPart)
  case change(CompanionChangePart)
  case status(String)
}

public enum CompanionMessageRole: String, Sendable { case user, assistant, system }

public enum CompanionMessageStatus: String, Sendable { case streaming, complete, error, cancelled }

public struct CompanionMessage: Sendable, Equatable, Identifiable {
  public var id: String
  public var role: CompanionMessageRole
  public var content: String
  public var parts: [CompanionPart]
  public var citations: [CompanionCitation]
  public var status: CompanionMessageStatus?

  public init(
    id: String = UUID().uuidString, role: CompanionMessageRole, content: String,
    parts: [CompanionPart]? = nil, citations: [CompanionCitation] = [],
    status: CompanionMessageStatus? = nil
  ) {
    self.id = id
    self.role = role
    self.content = content
    self.parts = parts ?? (content.isEmpty ? [] : [.text(content)])
    self.citations = citations
    self.status = status
  }
}

/// What the service tells the conversation as a turn streams in.
public enum CompanionUpdate: Sendable, Equatable {
  case delta(String)
  case thinking(String)
  case thinkingDone
  case tool(CompanionToolPart)
  case change(toolCallId: String, permissionId: String? = nil, status: CompanionChangeStatus,
    files: [CompanionFileChange]? = nil, error: String? = nil)
  case status(String)
  case citation(CompanionCitation)
  case warning(String)
  case error(String)
  case done(messageId: String)
  case cancelled(messageId: String)
}

public enum CompanionPermissionDecision: Sendable { case approve, reject }

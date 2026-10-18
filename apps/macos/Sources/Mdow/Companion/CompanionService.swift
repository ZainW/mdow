import AppKit
import MdowKit
import Observation

/// The private OpenCode server, shared by every window: started when a companion pane first
/// opens, stopped when the app quits or no pane has been used for a while.
@MainActor @Observable
final class CompanionRuntime {
  static let shared = CompanionRuntime()

  /// How long the server keeps running with every pane closed and nothing in flight.
  static let idleTimeout: Duration = .seconds(10 * 60)
  private static let modelWarmupAttempts = 8

  /// nil until the first check finishes (or after the server was stopped).
  private(set) var status: CompanionRuntimeStatus?
  private(set) var modelState: CompanionModelState = .loading

  @ObservationIgnored private var server: OpencodeServer?
  @ObservationIgnored private var client: OpencodeClient?
  @ObservationIgnored private var starting: Task<OpencodeClient, Error>?
  @ObservationIgnored private var sessions: [WeakSession] = []
  @ObservationIgnored private var visiblePanes = 0
  @ObservationIgnored private var activeTurns = 0
  @ObservationIgnored private var idleTask: Task<Void, Never>?
  @ObservationIgnored private var stopping = false

  private struct WeakSession {
    weak var session: CompanionSession?
  }

  private init() {
    NotificationCenter.default.addObserver(
      forName: NSApplication.willTerminateNotification, object: nil, queue: .main
    ) { _ in
      MainActor.assumeIsolated { CompanionRuntime.shared.stopNow() }
    }
  }

  var isAvailable: Bool { status?.availability == .available }

  // MARK: Lifecycle

  /// Checks for OpenCode, starts its server and loads the live model list.
  func connect(directory: String?) async {
    if status != nil, !isAvailable { status = nil }
    do {
      _ = try await ensureClient()
    } catch {
      let detected = await OpencodeBinary.detect().status
      status =
        detected.availability == .available
        ? CompanionRuntimeStatus(
          availability: .failed, version: detected.version, detail: error.localizedDescription)
        : detected
      modelState = CompanionModelState(
        options: [], currentValue: nil, stale: true, unavailableReason: status?.detail)
      return
    }
    await loadModels(directory: directory ?? NSHomeDirectory())
  }

  func ensureClient() async throws -> OpencodeClient {
    if let client, server?.isRunning == true { return client }
    if let starting { return try await starting.value }
    let task = Task<OpencodeClient, Error> {
      let detected = await OpencodeBinary.detect()
      guard let binary = detected.binary else {
        throw CompanionError(detected.status.detail ?? "OpenCode is not available.")
      }
      OpencodeServerCleanup.installSignalHandlers { CompanionRuntime.shared.stopNow() }
      let server = try await OpencodeServer.start(binary: binary.path)
      let client = OpencodeClient(baseURL: server.url, password: server.password)
      self.server = server
      self.client = client
      self.status = CompanionRuntimeStatus(availability: .available, version: binary.version)
      server.onExit { [weak server] in
        Task { @MainActor in
          guard let server else { return }
          CompanionRuntime.shared.serverExited(server)
        }
      }
      return client
    }
    starting = task
    defer { starting = nil }
    return try await task.value
  }

  private func serverExited(_ exited: OpencodeServer) {
    guard server === exited else { return }
    let unexpected = !stopping
    server = nil
    client = nil
    stopping = false
    status =
      unexpected && visiblePanes > 0
      ? CompanionRuntimeStatus(availability: .failed, detail: "OpenCode stopped unexpectedly.")
      : nil
    modelState = .loading
    for entry in sessions { entry.session?.serverStopped(unexpectedly: unexpected) }
    sessions.removeAll { $0.session == nil }
  }

  /// Stops the server right away, for quitting and idle time.
  func stopNow() {
    idleTask?.cancel()
    guard let server else { return }
    stopping = true
    for entry in sessions { entry.session?.serverStopped(unexpectedly: false) }
    server.stop()
  }

  func register(_ session: CompanionSession) {
    sessions.removeAll { $0.session == nil }
    sessions.append(WeakSession(session: session))
  }

  func paneVisibilityChanged(_ visible: Bool) {
    visiblePanes = max(0, visiblePanes + (visible ? 1 : -1))
    scheduleIdleStop()
  }

  func turnCountChanged(_ delta: Int) {
    activeTurns = max(0, activeTurns + delta)
    scheduleIdleStop()
  }

  private func scheduleIdleStop() {
    idleTask?.cancel()
    idleTask = nil
    guard visiblePanes == 0, activeTurns == 0, server != nil else { return }
    idleTask = Task {
      try? await Task.sleep(for: Self.idleTimeout)
      guard !Task.isCancelled else { return }
      self.stopNow()
    }
  }

  // MARK: Models

  @discardableResult
  func loadModels(directory: String) async -> CompanionModelState {
    let client: OpencodeClient
    do {
      client = try await ensureClient()
    } catch {
      modelState = CompanionModelState(
        options: [], currentValue: nil, stale: true, unavailableReason: error.localizedDescription)
      return modelState
    }
    do {
      // A fresh server loads the model catalog in the background, so the first answers can be
      // empty.
      var models = try await client.listModels(directory: directory)
      var attempt = 1
      while models.isEmpty, attempt < Self.modelWarmupAttempts {
        try? await Task.sleep(for: .milliseconds(500))
        models = try await client.listModels(directory: directory)
        attempt += 1
      }
      let providers = (try? await client.listProviders(directory: directory)) ?? []
      let options = CompanionLogic.modelOptions(models: models, providers: providers)
      var current = Preferences.shared.companionModel.flatMap { saved in
        options.contains { $0.value == saved } ? saved : nil
      }
      if current == nil, let fallback = try? await client.defaultModel(directory: directory),
        options.contains(where: { $0.value == fallback.value })
      {
        current = fallback.value
      }
      modelState = CompanionModelState(
        options: options, currentValue: current, stale: false,
        unavailableReason: options.isEmpty
          ? "No models yet. Run `opencode auth login` to connect a provider." : nil)
    } catch {
      modelState = CompanionModelState(
        options: [], currentValue: nil, stale: true, unavailableReason: error.localizedDescription)
    }
    return modelState
  }

  func hasModel(_ value: String) -> Bool { modelState.options.contains { $0.value == value } }

  /// Remembers the choice; sessions switch their own OpenCode session.
  func selectModel(_ value: String) throws {
    guard OpencodeModelRef(value: value) != nil, hasModel(value) else {
      throw CompanionError("Model is not available: \(value)")
    }
    Preferences.shared.companionModel = value
    modelState.currentValue = value
  }

  /// The saved model when OpenCode still offers it, otherwise OpenCode's own default.
  func sessionModel(directory: String) async -> OpencodeModelRef? {
    guard let saved = Preferences.shared.companionModel else { return nil }
    if modelState.options.isEmpty { await loadModels(directory: directory) }
    return hasModel(saved) ? OpencodeModelRef(value: saved) : nil
  }
}

struct CompanionError: LocalizedError {
  var message: String
  init(_ message: String) { self.message = message }
  var errorDescription: String? { message }
}

/// One window's conversation with OpenCode, ported from the desktop's `CompanionService` and
/// the renderer's companion store.
@MainActor @Observable
final class CompanionSession {
  private(set) var conversation = CompanionConversation()
  /// While the message field has focus, ⌘⌫ edits text instead of declining a suggestion.
  var composerFocused = false

  @ObservationIgnored weak var context: CompanionDocumentContext?
  @ObservationIgnored let runtime: CompanionRuntime
  @ObservationIgnored private var active: ActiveSession?
  @ObservationIgnored private var pendingPermissions: Set<String> = []
  @ObservationIgnored private var turn: ActiveTurn? {
    didSet {
      if (oldValue == nil) != (turn == nil) { runtime.turnCountChanged(turn == nil ? -1 : 1) }
    }
  }

  private static let eventStreamTimeout: Duration = .seconds(10)

  final class ActiveSession {
    let id: String
    let directory: String
    let translator: OpencodeTurnTranslator
    var events: Task<Void, Never>?
    var openWaiter: CheckedContinuation<Void, Error>?
    var isOpen = false

    init(id: String, directory: String) {
      self.id = id
      self.directory = directory
      translator = OpencodeTurnTranslator(sessionId: id, directory: directory)
    }

    func resolveOpen(_ error: Error?) {
      guard let waiter = openWaiter else { return }
      openWaiter = nil
      if let error { waiter.resume(throwing: error) } else { waiter.resume() }
    }
  }

  final class ActiveTurn {
    let messageId = UUID().uuidString
    /// When the prompt was sent, to tell this turn's finish from an earlier run's.
    var promptedAt = Date()
    /// Set once OpenCode starts the run for this prompt. Until then, events still arriving
    /// from a run the user stopped are ignored, so a late "interrupted" can't end this turn.
    var started = false
  }

  init(runtime: CompanionRuntime = .shared) {
    self.runtime = runtime
    runtime.register(self)
  }

  var directory: String {
    CompanionLogic.sessionDirectory(document: context?.documentURL, folder: context?.folderURL)
  }

  // MARK: Composer actions

  /// Sends `text` as the user's message, with what they are looking at as context.
  func send(_ text: String) {
    let trimmed = text.trimmingCharacters(in: .whitespacesAndNewlines)
    guard !trimmed.isEmpty, !conversation.streaming else { return }
    conversation.appendUserMessage(trimmed)
    conversation.beginRequest()
    let activePath = context?.documentURL?.path
    let tags = conversation.tags
    Task { await self.sendToOpencode(trimmed, activePath: activePath, tags: tags) }
  }

  func stop() {
    Task {
      await cancel()
      conversation.cancelRequest()
    }
  }

  func newChat() {
    conversation.reset()
    Task {
      if turn != nil { await cancel() }
      closeSession()
    }
  }

  func addTag(_ url: URL) { conversation.addTag(CompanionContextTag(path: url.path)) }
  func removeTag(_ sourceId: String) { conversation.removeTag(sourceId) }
  func clearError() { conversation.error = nil }
  func dismissWarnings() { conversation.dismissWarnings() }

  func selectModel(_ value: String) {
    do {
      try runtime.selectModel(value)
      conversation.error = nil
    } catch {
      conversation.error = error.localizedDescription
      return
    }
    guard let active, let ref = OpencodeModelRef(value: value) else { return }
    Task {
      do {
        try await runtime.ensureClient().switchModel(sessionId: active.id, model: ref)
      } catch {
        conversation.error = error.localizedDescription
      }
    }
  }

  /// Accepts (`once`) or declines a suggested edit.
  func review(_ permissionId: String, _ decision: CompanionPermissionDecision) {
    guard let change = conversation.change(permissionId: permissionId) else { return }
    // Hide the buttons right away; OpenCode confirms with applied or rejected events.
    conversation.markReviewed(
      toolCallId: change.toolCallId, status: decision == .reject ? .rejected : .pending,
      permissionId: nil)
    Task {
      do {
        try await replyPermission(permissionId, decision)
      } catch {
        conversation.markReviewed(
          toolCallId: change.toolCallId, status: .pending, permissionId: permissionId)
        conversation.error = error.localizedDescription
      }
    }
  }

  /// The window closed: stop whatever is running.
  func shutdown() {
    Task {
      if turn != nil { await cancel() }
      closeSession()
    }
  }

  // MARK: Service

  private func emit(_ update: CompanionUpdate) {
    conversation.apply(update)
  }

  private func sendToOpencode(_ text: String, activePath: String?, tags: [CompanionContextTag]) async {
    if turn != nil {
      emit(.warning("Wait for the current response or stop it first."))
      return
    }
    let turn = ActiveTurn()
    self.turn = turn
    do {
      let client = try await runtime.ensureClient()
      guard self.turn === turn else { return }
      let directory = self.directory
      let session = try await ensureSession(client, directory: directory)
      // Stopped while OpenCode was starting: nothing is waiting for a response any more.
      guard self.turn === turn else { return }
      session.translator.beginTurn()
      turn.promptedAt = Date()
      try await client.prompt(
        sessionId: session.id,
        text: CompanionLogic.composePrompt(
          text: text, activePath: activePath, tags: tags, directory: directory))
    } catch {
      guard self.turn === turn else { return }
      emit(.error(error.localizedDescription))
      self.turn = nil
    }
  }

  private func ensureSession(_ client: OpencodeClient, directory: String) async throws -> ActiveSession {
    if let active, active.directory == directory { return active }
    closeSession()

    let id = try await client.createSession(
      directory: directory, model: await runtime.sessionModel(directory: directory),
      permissions: CompanionLogic.sessionPermissions)
    // Older 2.x servers may lack session instructions; the permission rules still apply.
    try? await client.setInstructions(sessionId: id, key: "mdow", value: CompanionLogic.instructions)

    let session = ActiveSession(id: id, directory: directory)
    active = session
    session.events = Task { [weak self] in
      do {
        for try await item in client.events(directory: directory) {
          guard let self else { return }
          switch item {
          case .opened(let reconnected):
            session.isOpen = true
            session.resolveOpen(nil)
            if reconnected { await self.reconcileTurn(session) }
          case .event(let event):
            self.handleEvent(session, event)
          }
        }
      } catch {
        session.resolveOpen(error)
        guard let self, self.active === session, self.turn != nil else { return }
        self.emit(.error(error.localizedDescription))
        self.turn = nil
      }
    }

    // Prompting before the stream is open would lose the start of the response, so wait.
    do {
      if !session.isOpen {
        try await withCheckedThrowingContinuation { (continuation: CheckedContinuation<Void, Error>) in
          session.openWaiter = continuation
          Task {
            try? await Task.sleep(for: Self.eventStreamTimeout)
            session.resolveOpen(CompanionError("OpenCode did not open its event stream."))
          }
        }
      }
    } catch {
      if active === session { closeSession() }
      throw error
    }
    return session
  }

  /// Events sent while the stream was reconnecting are not replayed. If the turn finished in
  /// that gap, read how it ended from the session instead of waiting forever.
  private func reconcileTurn(_ session: ActiveSession) async {
    guard let turn, active === session,
      let info = try? await runtime.ensureClient().getSession(session.id)
    else { return }
    guard self.turn === turn, let outcome = info.outcome, let idle = info.time.idle,
      idle >= turn.promptedAt.timeIntervalSince1970 * 1000
    else { return }
    turn.started = true
    finishTurn(TurnFinish(TurnFinish.Outcome(rawValue: outcome) ?? .interrupted))
  }

  private func closeSession() {
    active?.events?.cancel()
    active?.resolveOpen(CancellationError())
    active = nil
    pendingPermissions.removeAll()
  }

  func handleEvent(_ session: ActiveSession, _ event: OpencodeEvent) {
    guard active === session else { return }
    let result = session.translator.translate(event)
    if result.started, let turn { turn.started = true }
    let live = turn?.started == true

    if live {
      for update in result.updates {
        if case .change(_, let permissionId?, .pending, _, _) = update {
          pendingPermissions.insert(permissionId)
        }
        emit(update)
      }
    }

    if let reject = result.autoReject {
      Task {
        try? await runtime.ensureClient().replyPermission(
          sessionId: session.id, requestId: reject.permissionId, decision: .reject,
          message: reject.reason)
      }
    }

    if let finish = result.finish, live { finishTurn(finish) }
  }

  private func finishTurn(_ finish: TurnFinish) {
    guard let turn else { return }
    switch finish.outcome {
    case .failed: emit(.error(finish.error ?? "OpenCode failed."))
    case .interrupted: emit(.cancelled(messageId: turn.messageId))
    case .succeeded: emit(.done(messageId: turn.messageId))
    }
    self.turn = nil
    pendingPermissions.removeAll()
  }

  private func replyPermission(_ permissionId: String, _ decision: CompanionPermissionDecision)
    async throws
  {
    guard let active, runtime.isAvailable, pendingPermissions.contains(permissionId) else {
      throw CompanionError("This change is no longer waiting for review.")
    }
    pendingPermissions.remove(permissionId)
    // `always` would save the rule in OpenCode's global config, so Mdow only answers once.
    try await runtime.ensureClient().replyPermission(
      sessionId: active.id, requestId: permissionId, decision: decision == .approve ? .once : .reject,
      message: decision == .reject ? "The user rejected this change." : nil)
  }

  private func cancel() async {
    let turn = self.turn
    let session = active
    self.turn = nil
    if let session, runtime.isAvailable, let client = try? await runtime.ensureClient() {
      let pending = pendingPermissions
      let sessionId = session.id
      pendingPermissions.removeAll()
      await withTaskGroup(of: Void.self) { group in
        group.addTask { try? await client.interrupt(sessionId: sessionId) }
        for id in pending {
          group.addTask {
            try? await client.replyPermission(
              sessionId: sessionId, requestId: id, decision: .reject,
              message: "The user stopped the response.")
          }
        }
      }
    }
    if let turn { emit(.cancelled(messageId: turn.messageId)) }
  }

  /// The server went away; the next message starts a new one and a new session.
  func serverStopped(unexpectedly: Bool) {
    closeSession()
    if turn != nil {
      emit(.error("OpenCode stopped unexpectedly. Send again to restart it."))
      turn = nil
    }
  }
}

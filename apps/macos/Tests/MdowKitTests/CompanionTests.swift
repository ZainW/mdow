import Foundation
import Synchronization
import Testing

@testable import MdowKit

// Mirrors the desktop app's companion tests (`apps/desktop/src/main/companion/*.test.ts`).
// Event shapes are the ones `opencode serve` 2.0.x sends.

private let session = "ses_test"
private let dir = "/docs"
private let patch = "--- notes.md\n+++ notes.md\n@@ -1 +1 @@\n-teh\n+the\n"

private func event(_ type: String, _ data: [String: JSONValue] = [:]) -> OpencodeEvent {
  var data = data
  data["sessionID"] = .string(session)
  return OpencodeEvent(type: type, data: data)
}

@Suite struct OpencodeTurnTranslatorTests {
  @Test func ignoresEventsFromOtherSessions() {
    let translator = OpencodeTurnTranslator(sessionId: session, directory: dir)
    let result = translator.translate(
      OpencodeEvent(type: "session.text.delta", data: ["sessionID": "ses_other", "delta": "hi"]))
    #expect(result.updates.isEmpty)
  }

  @Test func streamsReasoningThenClosesItWhenTextStarts() {
    let translator = OpencodeTurnTranslator(sessionId: session, directory: dir)
    #expect(translator.translate(event("session.reasoning.delta", ["delta": "Hmm"])).updates == [.thinking("Hmm")])
    #expect(
      translator.translate(event("session.text.delta", ["delta": "Fixed."])).updates == [
        .thinkingDone, .delta("Fixed."),
      ])
  }

  @Test func showsReadsAsTitledToolRowsAndCitesTheDocument() {
    let translator = OpencodeTurnTranslator(sessionId: session, directory: dir)
    _ = translator.translate(event("session.tool.input.started", ["id": "call_1", "name": "read"]))
    let called = translator.translate(
      event("session.tool.called", ["id": "call_1", "input": ["path": "/docs/notes.md"]]))
    guard case .tool(let tool) = called.updates.first else {
      Issue.record("expected a tool row")
      return
    }
    #expect(tool.toolCallId == "call_1")
    #expect(tool.name == "Read notes.md")
    #expect(tool.state == .running)
    #expect(
      called.updates.last
        == .citation(CompanionCitation(sourceId: "read:/docs/notes.md", path: "/docs/notes.md", label: "notes.md")))

    let done = translator.translate(
      event("session.tool.success", ["id": "call_1", "content": [["type": "text", "text": "1: # Notes"]]]))
    #expect(
      done.updates == [
        .tool(CompanionToolPart(toolCallId: "call_1", name: "Read notes.md", state: .completed, output: "1: # Notes"))
      ])
  }

  @Test func relativeReadsAreResolvedAndCitedOnce() {
    let translator = OpencodeTurnTranslator(sessionId: session, directory: dir)
    _ = translator.translate(event("session.tool.input.started", ["id": "a", "name": "read"]))
    let first = translator.translate(event("session.tool.called", ["id": "a", "input": ["path": "guide/intro.md"]]))
    #expect(first.updates.count == 2)
    _ = translator.translate(event("session.tool.input.started", ["id": "b", "name": "read"]))
    let second = translator.translate(event("session.tool.called", ["id": "b", "input": ["path": "guide/intro.md"]]))
    #expect(second.updates.count == 1)
  }

  @Test func turnsAnEditPermissionIntoAPendingChangeCardThenAppliesIt() {
    let translator = OpencodeTurnTranslator(sessionId: session, directory: dir)
    #expect(translator.translate(event("session.tool.input.started", ["id": "call_2", "name": "edit"])).updates.isEmpty)

    let asked = translator.translate(
      event(
        "permission.asked",
        [
          "id": "per_1", "action": "edit", "resources": ["notes.md"],
          "metadata": [
            "files": [
              ["file": "notes.md", "patch": .string(patch), "status": "modified", "additions": 1, "deletions": 1]
            ]
          ],
          "source": ["type": "tool", "id": "call_2"],
        ]))
    #expect(
      asked.updates == [
        .change(
          toolCallId: "call_2", permissionId: "per_1", status: .pending,
          files: [
            CompanionFileChange(
              path: "/docs/notes.md", displayPath: "notes.md", patch: patch, additions: 1, deletions: 1,
              status: .modified)
          ])
      ])

    let applied = translator.translate(
      event(
        "session.tool.success",
        [
          "id": "call_2", "content": [["type": "text", "text": "Edited notes.md"]],
          "metadata": ["files": [["file": "notes.md", "patch": .string(patch), "additions": 1, "deletions": 1]]],
        ]))
    guard case .change(let toolCallId, _, let status, _, _) = applied.updates.first else {
      Issue.record("expected a change")
      return
    }
    #expect(toolCallId == "call_2")
    #expect(status == .applied)
  }

  @Test func marksARejectedEditAsRejectedRatherThanFailed() {
    let translator = OpencodeTurnTranslator(sessionId: session, directory: dir)
    _ = translator.translate(event("session.tool.input.started", ["id": "call_3", "name": "edit"]))
    _ = translator.translate(event("permission.asked", ["id": "per_2", "action": "edit", "source": ["id": "call_3"]]))
    #expect(
      translator.translate(
        OpencodeEvent(type: "permission.replied", data: ["sessionID": .string(session), "requestID": "per_2", "reply": "reject"])
      ).updates == [.change(toolCallId: "call_3", status: .rejected)])
    #expect(
      translator.translate(event("session.tool.failed", ["id": "call_3", "error": ["message": "Rejected"]])).updates
        == [.change(toolCallId: "call_3", status: .rejected)])
  }

  @Test func reportsFailedEditsWithTheirError() {
    let translator = OpencodeTurnTranslator(sessionId: session, directory: dir)
    _ = translator.translate(event("session.tool.input.started", ["id": "c", "name": "patch"]))
    #expect(
      translator.translate(event("session.tool.failed", ["id": "c", "error": ["message": "No match"]])).updates
        == [.change(toolCallId: "c", status: .failed, error: "No match")])
  }

  @Test func declinesShellPermissionsAutomatically() {
    let translator = OpencodeTurnTranslator(sessionId: session, directory: dir)
    let result = translator.translate(
      event("permission.asked", ["id": "per_3", "action": "shell", "resources": ["ls"], "source": ["id": "call_4"]]))
    #expect(result.updates.isEmpty)
    #expect(result.autoReject?.permissionId == "per_3")
    #expect(result.autoReject?.reason.contains("shell") == true)
    #expect(result.autoReject?.reason.contains("(ls)") == true)
  }

  @Test func reportsHowTheTurnEnded() {
    let translator = OpencodeTurnTranslator(sessionId: session, directory: dir)
    #expect(translator.translate(event("session.execution.succeeded")).finish == TurnFinish(.succeeded))
    #expect(
      translator.translate(
        event("session.execution.failed", ["error": ["type": "provider.auth", "message": "Sign in again"]])
      ).finish == TurnFinish(.failed, error: "Sign in again"))
    #expect(translator.translate(event("session.execution.interrupted")).finish == TurnFinish(.interrupted))
    let started = translator.translate(event("session.execution.started"))
    #expect(started.finish == nil)
    #expect(started.started)
  }

  @Test func toolTitlesUsePlainWords() {
    #expect(opencodeToolTitle("grep", input: ["pattern": "roadmap"]) == "Searched for “roadmap”")
    #expect(opencodeToolTitle("glob", input: ["pattern": "**/*.md"]) == "Looked through the folder")
    #expect(opencodeToolTitle("websearch", input: ["query": "swift"]) == "Searched the web for “swift”")
    #expect(opencodeToolTitle("mystery") == "Mystery")
  }

  @Test func fileChangesResolveRelativePathsAndSkipMalformedEntries() {
    #expect(
      opencodeFileChanges(["files": [["file": "a.md", "status": "added"], ["nope": true], "x"]], directory: dir) == [
        CompanionFileChange(path: "/docs/a.md", displayPath: "a.md", patch: "", additions: 0, deletions: 0, status: .added)
      ])
    #expect(opencodeFileChanges(nil, directory: dir).isEmpty)
  }
}

@Suite struct OpencodeStreamTests {
  @Test func sseParserJoinsDataLinesAndHandlesSplitChunks() {
    var parser = SSEParser()
    var received = parser.feed("event: message\ndata: {\"type\":\"a\",")
    received += parser.feed("\"data\":{}}\n\ndata: one\r\ndata: two\n\n: heartbeat\n\n")
    #expect(received == ["{\"type\":\"a\",\"data\":{}}", "one\ntwo"])
  }

  @Test func parsesTypedEventsAndDropsMalformedPayloads() {
    #expect(
      OpencodeEvent.parse("{\"id\":\"evt_1\",\"type\":\"session.idle\",\"data\":{\"sessionID\":\"s\"}}")
        == OpencodeEvent(id: "evt_1", type: "session.idle", data: ["sessionID": "s"]))
    #expect(OpencodeEvent.parse("{\"data\":{}}") == nil)
    #expect(OpencodeEvent.parse("not json") == nil)
  }
}

/// Records requests and answers with a canned response.
private final class FakeTransport: Sendable {
  let requests = Mutex<[URLRequest]>([])
  let status: Int
  let body: Data

  init(status: Int = 200, body: String = "{\"data\":[]}") {
    self.status = status
    self.body = Data(body.utf8)
  }

  var transport: OpencodeClient.Transport {
    { request in
      self.requests.withLock { $0.append(request) }
      let response = HTTPURLResponse(url: request.url!, statusCode: self.status, httpVersion: nil, headerFields: nil)!
      return (self.body, response)
    }
  }

  var last: URLRequest? { requests.withLock { $0.last } }
}

private func json(_ request: URLRequest?) -> JSONValue? {
  request?.httpBody.flatMap { JSONValue.parse($0) }
}

@Suite struct OpencodeClientTests {
  @Test func sendsBasicAuthAndScopesRequestsToAFolder() async throws {
    let fake = FakeTransport()
    let client = OpencodeClient(baseURL: URL(string: "http://127.0.0.1:9")!, password: "secret", transport: fake.transport)
    _ = try await client.listModels(directory: "/my docs")
    #expect(fake.last?.url?.absoluteString == "http://127.0.0.1:9/api/model?location%5Bdirectory%5D=%2Fmy%20docs")
    #expect(
      fake.last?.value(forHTTPHeaderField: "authorization")
        == "Basic " + Data("opencode:secret".utf8).base64EncodedString())
  }

  @Test func surfacesTheServerErrorMessage() async {
    let fake = FakeTransport(status: 400, body: "{\"error\":{\"message\":\"Unknown model\"}}")
    let client = OpencodeClient(baseURL: URL(string: "http://x")!, password: "p", transport: fake.transport)
    await #expect(throws: OpencodeHTTPError(status: 400, message: "Unknown model")) {
      try await client.prompt(sessionId: "ses_1", text: "hi")
    }
    #expect(json(fake.last) == ["text": "hi"])
    #expect(fake.last?.value(forHTTPHeaderField: "content-type") == "application/json")
  }

  @Test func repliesToPermissionsWithTheDecisionAndAnOptionalMessage() async throws {
    let fake = FakeTransport(status: 204, body: "")
    let client = OpencodeClient(baseURL: URL(string: "http://x")!, password: "p", transport: fake.transport)
    try await client.replyPermission(sessionId: "ses_1", requestId: "per_1", decision: .reject, message: "No thanks")
    #expect(fake.last?.url?.absoluteString == "http://x/api/session/ses_1/permission/per_1/reply")
    #expect(fake.last?.httpMethod == "POST")
    #expect(json(fake.last) == ["decision": "reject", "message": "No thanks"])

    try await client.replyPermission(sessionId: "ses_1", requestId: "per_2", decision: .once)
    #expect(json(fake.last) == ["decision": "once"])
  }

  @Test func createsALockedDownSessionInTheFolder() async throws {
    let fake = FakeTransport(body: "{\"data\":{\"id\":\"ses_9\"}}")
    let client = OpencodeClient(baseURL: URL(string: "http://x")!, password: "p", transport: fake.transport)
    let id = try await client.createSession(
      directory: "/docs", model: OpencodeModelRef(value: "openrouter/inclusionai/ling"),
      permissions: CompanionLogic.sessionPermissions)
    #expect(id == "ses_9")
    let body = json(fake.last)
    #expect(body?["location"] == ["directory": "/docs"])
    #expect(body?["model"] == ["providerID": "openrouter", "id": "inclusionai/ling"])
    #expect(body?["permissions"]?.array?.first == ["action": "shell", "resource": "*", "effect": "deny"])
    #expect(body?["permissions"]?.array?.contains(["action": "edit", "resource": "*", "effect": "ask"]) == true)

    _ = try await client.createSession(directory: "/docs", model: nil, permissions: [])
    #expect(json(fake.last)?["model"] == nil)
  }

  @Test func buildsTheOtherEndpoints() async throws {
    let fake = FakeTransport(status: 204, body: "")
    let client = OpencodeClient(baseURL: URL(string: "http://x")!, password: "p", transport: fake.transport)
    try await client.setInstructions(sessionId: "ses_1", key: "mdow", value: "Be brief.")
    #expect(fake.last?.httpMethod == "PUT")
    #expect(fake.last?.url?.path == "/api/experimental/session/ses_1/instructions/entries/mdow")
    try await client.interrupt(sessionId: "ses_1")
    #expect(fake.last?.url?.path == "/api/session/ses_1/interrupt")
    try await client.switchModel(sessionId: "ses_1", model: OpencodeModelRef(providerID: "opencode", id: "m"))
    #expect(json(fake.last) == ["model": ["providerID": "opencode", "id": "m"]])
    let events = client.eventRequest(directory: "/docs")
    #expect(events.url?.absoluteString == "http://x/api/event?location%5Bdirectory%5D=%2Fdocs")
    #expect(events.value(forHTTPHeaderField: "accept") == "text/event-stream")
  }

  @Test func decodesModelsAndTheDefault() async throws {
    let models = FakeTransport(
      body: "{\"data\":[{\"id\":\"a\",\"providerID\":\"p\",\"name\":\"A\",\"status\":\"active\",\"cost\":[]}]}")
    let client = OpencodeClient(baseURL: URL(string: "http://x")!, password: "p", transport: models.transport)
    let listed = try await client.listModels(directory: "/")
    #expect(listed == [OpencodeModelInfo(id: "a", providerID: "p", name: "A", status: "active")])
    let empty = FakeTransport(body: "{\"data\":null}")
    let other = OpencodeClient(baseURL: URL(string: "http://x")!, password: "p", transport: empty.transport)
    let fallback = try await other.defaultModel(directory: "/")
    #expect(fallback == nil)
  }
}

@Suite struct CompanionLogicTests {
  @Test func splitsModelValuesOnTheFirstSlash() {
    #expect(OpencodeModelRef(value: "openrouter/inclusionai/ling") == OpencodeModelRef(providerID: "openrouter", id: "inclusionai/ling"))
    #expect(OpencodeModelRef(value: "no-slash") == nil)
    #expect(OpencodeModelRef(value: "/x") == nil)
    #expect(OpencodeModelRef(value: "x/") == nil)
  }

  @Test func usesTheOpenFolderElseTheDocumentFolder() {
    let doc = URL(fileURLWithPath: "/docs/guide/intro.md")
    #expect(CompanionLogic.sessionDirectory(document: doc, folder: URL(fileURLWithPath: "/docs")) == "/docs")
    #expect(CompanionLogic.sessionDirectory(document: doc, folder: nil) == "/docs/guide")
    #expect(CompanionLogic.sessionDirectory(document: nil, folder: nil, home: "/Users/me") == "/Users/me")
  }

  @Test func prefixesThePromptWithWhatTheUserIsViewing() {
    #expect(
      CompanionLogic.composePrompt(
        text: "Tighten the intro", activePath: "/docs/guide/intro.md",
        tags: [CompanionContextTag(path: "/docs/notes.md")], directory: "/docs")
        == "<mdow-context>\nViewing: guide/intro.md\nAttached: notes.md\n</mdow-context>\n\nTighten the intro")
    #expect(CompanionLogic.composePrompt(text: "Hi", activePath: nil, tags: [], directory: "/docs") == "Hi")
    #expect(
      CompanionLogic.composePrompt(text: "Hi", activePath: "/elsewhere/a.md", tags: [], directory: "/docs")
        .contains("Viewing: /elsewhere/a.md"))
  }

  @Test func listsModelsGroupedByProviderWithoutDeprecatedOnes() {
    let options = CompanionLogic.modelOptions(
      models: [
        OpencodeModelInfo(id: "inclusionai/ling", providerID: "openrouter", name: "Ling"),
        OpencodeModelInfo(id: "claude-sonnet-5-5", providerID: "opencode", name: "Claude Sonnet 5.5"),
        OpencodeModelInfo(id: "old", providerID: "opencode", name: "Old", status: "deprecated"),
      ],
      providers: [OpencodeProviderInfo(id: "opencode", name: "OpenCode Zen"), OpencodeProviderInfo(id: "openrouter", name: "OpenRouter")])
    #expect(options.map(\.value) == ["opencode/claude-sonnet-5-5", "openrouter/inclusionai/ling"])
    #expect(options.first?.providerName == "OpenCode Zen")
  }

  @Test func requiresOpencode2OrNewer() {
    #expect(CompanionLogic.parseVersion("opencode v2.0.16\n") == "2.0.16")
    #expect(CompanionLogic.isSupportedVersion("2.0.16"))
    #expect(!CompanionLogic.isSupportedVersion("1.18.34"))
  }

  @Test func searchesTheInstallFolderEvenWhenPathIsMinimal() {
    let dirs = CompanionLogic.searchDirectories(path: "/usr/bin", home: "/Users/me", loginShellPath: "/opt/x/bin:/usr/bin")
    #expect(dirs.first == "/usr/bin")
    #expect(dirs[1] == "/opt/x/bin")
    #expect(dirs.contains("/Users/me/.opencode/bin"))
    #expect(dirs.contains("/opt/homebrew/bin"))
    #expect(Set(dirs).count == dirs.count)
  }

  @Test func findsTheListeningURLAndTheShellPath() {
    #expect(
      CompanionLogic.parseListeningURL("boot\nServer listening on http://127.0.0.1:4096\n")
        == URL(string: "http://127.0.0.1:4096"))
    #expect(CompanionLogic.parseListeningURL("starting…") == nil)
    #expect(CompanionLogic.parseShellPath("motd\n__MDOW_PATH__/a:/b__MDOW_PATH__") == "/a:/b")
    #expect(CompanionLogic.parseShellPath("nothing") == nil)
  }

  @Test func passwordsAreLongAndFresh() {
    let password = CompanionLogic.makePassword()
    #expect(password.count == 48)
    let isHex = password.allSatisfy(\.isHexDigit)
    #expect(isHex)
    #expect(password != CompanionLogic.makePassword())
  }

  @Test func parsesUnifiedDiffs() {
    let hunks = UnifiedDiff.parse(
      "Index: x\n===\n--- a\n+++ b\n@@ -1,3 +1,3 @@ Title\n # Notes\n \n-Teh fox.\n+The fox.\n\\ No newline at end of file\n")
    #expect(hunks.count == 1)
    #expect(hunks[0].header == "Title")
    #expect(hunks[0].lines.map(\.kind) == [.context, .context, .remove, .add])
    #expect(hunks[0].lines[2] == DiffLine(kind: .remove, text: "Teh fox.", oldLine: 3, newLine: nil))
    #expect(hunks[0].lines[3] == DiffLine(kind: .add, text: "The fox.", oldLine: nil, newLine: 3))
    #expect(UnifiedDiff.parse("").isEmpty)
  }

  @Test func splitsRepliesIntoBlocks() {
    #expect(
      ReplyBlocks.split("## Title\nSome *text*\nmore\n\n```swift\nlet a = 1\n\nlet b = 2\n```\n- one\n- two") == [
        .heading(level: 2, text: "Title"), .paragraph("Some *text*\nmore"),
        .code(language: "swift", text: "let a = 1\n\nlet b = 2"), .paragraph("- one\n- two"),
      ])
    #expect(ReplyBlocks.split("```\nunfinished") == [.code(language: "", text: "unfinished")])
    #expect(ReplyBlocks.split("#hashtag") == [.paragraph("#hashtag")])
  }
}

@Suite struct CompanionConversationTests {
  @Test func streamsTextIntoOneAssistantMessage() {
    var conversation = CompanionConversation()
    conversation.appendUserMessage("Hi")
    conversation.beginRequest()
    conversation.apply(.delta("Hello"))
    conversation.apply(.delta(", there"))
    #expect(conversation.messages.count == 2)
    #expect(conversation.messages[1].content == "Hello, there")
    #expect(conversation.streaming)
    conversation.apply(.done(messageId: "x"))
    #expect(!conversation.streaming)
    #expect(conversation.messages[1].status == .complete)
  }

  @Test func mergesReasoningAndToolUpdates() {
    var conversation = CompanionConversation()
    conversation.apply(.thinking("A"))
    conversation.apply(.thinking("B"))
    conversation.apply(.thinkingDone)
    conversation.apply(.thinking("C"))
    conversation.apply(.tool(CompanionToolPart(toolCallId: "t", name: "Read", state: .running, input: "{}")))
    conversation.apply(.tool(CompanionToolPart(toolCallId: "t", name: "Read a.md", state: .completed, output: "ok")))
    let parts = conversation.messages[0].parts
    #expect(parts.count == 2)
    #expect(parts[0] == .thinking(text: "AB\n\nC", done: false))
    #expect(parts[1] == .tool(CompanionToolPart(toolCallId: "t", name: "Read a.md", state: .completed, input: "{}", output: "ok")))
  }

  @Test func tracksPendingReviewsAndSettlesThemWhenTheTurnEnds() {
    var conversation = CompanionConversation()
    let file = CompanionFileChange(path: "/docs/a.md", displayPath: "a.md", patch: patch, additions: 1, deletions: 1, status: .modified)
    conversation.apply(.change(toolCallId: "c", permissionId: "per_1", status: .pending, files: [file]))
    #expect(conversation.pendingReviewPaths == ["/docs/a.md"])
    #expect(conversation.change(permissionId: "per_1")?.toolCallId == "c")

    // Accepting hides the buttons until OpenCode confirms.
    conversation.markReviewed(toolCallId: "c", status: .pending, permissionId: nil)
    #expect(conversation.pendingReviewPaths.isEmpty)
    #expect(conversation.pendingChange == nil)
    conversation.apply(.change(toolCallId: "c", status: .applied))
    guard case .change(let applied) = conversation.messages[0].parts[0] else {
      Issue.record("expected a change")
      return
    }
    #expect(applied.status == .applied)
    #expect(applied.files == [file])

    conversation.apply(.change(toolCallId: "d", permissionId: "per_2", status: .pending, files: [file]))
    conversation.apply(.tool(CompanionToolPart(toolCallId: "t", name: "Read", state: .running)))
    conversation.cancelRequest()
    let parts = conversation.messages[0].parts
    #expect(parts[1] == .change(CompanionChangePart(toolCallId: "d", status: .rejected, files: [file])))
    #expect(parts[2] == .tool(CompanionToolPart(toolCallId: "t", name: "Read", state: .cancelled)))
    #expect(conversation.messages[0].status == .cancelled)
  }

  @Test func errorsEndTheTurnAndCitationsAreDeduplicated() {
    var conversation = CompanionConversation()
    conversation.apply(.delta("x"))
    let citation = CompanionCitation(sourceId: "read:/a.md", path: "/a.md", label: "a.md")
    conversation.apply(.citation(citation))
    conversation.apply(.citation(citation))
    #expect(conversation.messages[0].citations.count == 1)
    conversation.apply(.error("Sign in"))
    #expect(conversation.error == "Sign in")
    #expect(conversation.messages[0].status == .error)
    // The next turn starts a fresh assistant message.
    conversation.apply(.delta("y"))
    #expect(conversation.messages.count == 2)
  }

  @Test func tagsAreUniqueAndRemovable() {
    var conversation = CompanionConversation()
    conversation.addTag(CompanionContextTag(path: "/a.md"))
    conversation.addTag(CompanionContextTag(path: "/a.md"))
    #expect(conversation.tags.count == 1)
    conversation.removeTag("tag:/a.md")
    #expect(conversation.tags.isEmpty)
  }
}

import Foundation

// A small client for the OpenCode 2 HTTP API (`opencode serve`), mirroring the desktop app's
// `opencode-client.ts`. The published SDK still targets the pre-2.0 event names, so Mdow
// speaks to the endpoints it needs directly.

public struct OpencodeModelRef: Codable, Sendable, Equatable {
  public var providerID: String
  public var id: String

  public init(providerID: String, id: String) {
    self.providerID = providerID
    self.id = id
  }

  /// `providerID/modelID`, split on the first slash (model ids may contain slashes).
  public init?(value: String) {
    guard let slash = value.firstIndex(of: "/"), slash != value.startIndex,
      value.index(after: slash) != value.endIndex
    else { return nil }
    providerID = String(value[..<slash])
    id = String(value[value.index(after: slash)...])
  }

  public var value: String { "\(providerID)/\(id)" }
}

public struct OpencodeModelInfo: Decodable, Sendable, Equatable {
  public var id: String
  public var providerID: String
  public var name: String?
  public var status: String?

  public init(id: String, providerID: String, name: String? = nil, status: String? = nil) {
    self.id = id
    self.providerID = providerID
    self.name = name
    self.status = status
  }
}

public struct OpencodeProviderInfo: Decodable, Sendable, Equatable {
  public var id: String
  public var name: String?
}

public struct OpencodePermissionRule: Codable, Sendable, Equatable {
  public enum Effect: String, Codable, Sendable { case allow, deny, ask }
  public var action: String
  public var resource: String
  public var effect: Effect

  public init(action: String, resource: String, effect: Effect) {
    self.action = action
    self.resource = resource
    self.effect = effect
  }
}

public struct OpencodeSessionInfo: Decodable, Sendable, Equatable {
  public struct Time: Decodable, Sendable, Equatable {
    public var created: Double
    public var updated: Double
    public var idle: Double?
  }
  public var id: String
  public var outcome: String?
  public var time: Time
}

public enum OpencodePermissionReply: String, Sendable { case once, always, reject }

public struct OpencodeHTTPError: Error, LocalizedError, Equatable {
  public var status: Int
  public var message: String
  public var errorDescription: String? { message }

  public init(status: Int, message: String) {
    self.status = status
    self.message = message
  }

  /// The server's own message when the body is a JSON error, else the start of the body.
  public static func message(status: Int, body: Data) -> String {
    if let json = JSONValue.parse(body),
      let message = json["error"]?["message"]?.string ?? json["message"]?.string, !message.isEmpty
    {
      return message
    }
    let text = String(decoding: body, as: UTF8.self).trimmingCharacters(in: .whitespacesAndNewlines)
    return text.isEmpty ? "OpenCode request failed (\(status))" : String(text.prefix(200))
  }
}

/// Item from the event stream: it (re)connected, or an event arrived.
public enum OpencodeStreamItem: Sendable {
  /// `reconnected` is false the first time.
  case opened(reconnected: Bool)
  case event(OpencodeEvent)
}

public struct OpencodeClient: Sendable {
  public typealias Transport = @Sendable (URLRequest) async throws -> (Data, URLResponse)

  public let baseURL: URL
  public let authorization: String
  private let transport: Transport

  public init(baseURL: URL, password: String, transport: Transport? = nil) {
    self.baseURL = baseURL
    authorization = "Basic " + Data("opencode:\(password)".utf8).base64EncodedString()
    self.transport = transport ?? { request in try await URLSession.shared.data(for: request) }
  }

  // MARK: Requests

  static func locationQuery(_ directory: String) -> String {
    var allowed = CharacterSet.alphanumerics
    allowed.insert(charactersIn: "-_.!~*'()")
    let encoded = directory.addingPercentEncoding(withAllowedCharacters: allowed) ?? directory
    return "location%5Bdirectory%5D=\(encoded)"
  }

  /// Escapes one path segment, like `encodeURIComponent`.
  static func segment(_ value: String) -> String {
    var allowed = CharacterSet.alphanumerics
    allowed.insert(charactersIn: "-_.!~*'()")
    return value.addingPercentEncoding(withAllowedCharacters: allowed) ?? value
  }

  public func makeRequest(_ method: String, _ path: String, body: JSONValue? = nil) -> URLRequest {
    var request = URLRequest(url: URL(string: baseURL.absoluteString + path)!)
    request.httpMethod = method
    request.setValue(authorization, forHTTPHeaderField: "authorization")
    if let body {
      request.setValue("application/json", forHTTPHeaderField: "content-type")
      request.httpBody = try? JSONEncoder().encode(body)
    }
    return request
  }

  @discardableResult
  private func send(_ method: String, _ path: String, body: JSONValue? = nil) async throws -> Data {
    let (data, response) = try await transport(makeRequest(method, path, body: body))
    let status = (response as? HTTPURLResponse)?.statusCode ?? 0
    guard (200..<300).contains(status) else {
      throw OpencodeHTTPError(status: status, message: OpencodeHTTPError.message(status: status, body: data))
    }
    return data
  }

  private struct Envelope<T: Decodable>: Decodable { var data: T }

  private func get<T: Decodable>(_ path: String, as type: T.Type) async throws -> T {
    try JSONDecoder().decode(Envelope<T>.self, from: try await send("GET", path)).data
  }

  public func listModels(directory: String) async throws -> [OpencodeModelInfo] {
    try await get("/api/model?\(Self.locationQuery(directory))", as: [OpencodeModelInfo].self)
  }

  public func listProviders(directory: String) async throws -> [OpencodeProviderInfo] {
    try await get("/api/provider?\(Self.locationQuery(directory))", as: [OpencodeProviderInfo].self)
  }

  public func defaultModel(directory: String) async throws -> OpencodeModelRef? {
    try await get("/api/model/default?\(Self.locationQuery(directory))", as: OpencodeModelRef?.self)
  }

  public static func sessionBody(
    directory: String, model: OpencodeModelRef?, permissions: [OpencodePermissionRule]
  ) -> JSONValue {
    var body: [String: JSONValue] = [
      "location": ["directory": .string(directory)],
      "permissions": .array(
        permissions.map {
          ["action": .string($0.action), "resource": .string($0.resource), "effect": .string($0.effect.rawValue)]
        }),
    ]
    if let model { body["model"] = ["providerID": .string(model.providerID), "id": .string(model.id)] }
    return .object(body)
  }

  public func createSession(
    directory: String, model: OpencodeModelRef?, permissions: [OpencodePermissionRule]
  ) async throws -> String {
    struct Created: Decodable { var id: String }
    let data = try await send(
      "POST", "/api/session",
      body: Self.sessionBody(directory: directory, model: model, permissions: permissions))
    return try JSONDecoder().decode(Envelope<Created>.self, from: data).data.id
  }

  public func setInstructions(sessionId: String, key: String, value: String) async throws {
    try await send(
      "PUT", "/api/experimental/session/\(sessionId)/instructions/entries/\(Self.segment(key))",
      body: ["value": .string(value)])
  }

  /// Whether the session has finished its latest run, and how.
  public func getSession(_ sessionId: String) async throws -> OpencodeSessionInfo {
    try await get("/api/session/\(sessionId)", as: OpencodeSessionInfo.self)
  }

  public func switchModel(sessionId: String, model: OpencodeModelRef) async throws {
    try await send(
      "POST", "/api/session/\(sessionId)/model",
      body: ["model": ["providerID": .string(model.providerID), "id": .string(model.id)]])
  }

  public func prompt(sessionId: String, text: String) async throws {
    try await send("POST", "/api/session/\(sessionId)/prompt", body: ["text": .string(text)])
  }

  public func interrupt(sessionId: String) async throws {
    try await send("POST", "/api/session/\(sessionId)/interrupt")
  }

  public func replyPermission(
    sessionId: String, requestId: String, decision: OpencodePermissionReply, message: String? = nil
  ) async throws {
    var body: [String: JSONValue] = ["decision": .string(decision.rawValue)]
    if let message, !message.isEmpty { body["message"] = .string(message) }
    try await send(
      "POST", "/api/session/\(sessionId)/permission/\(requestId)/reply", body: .object(body))
  }

  // MARK: Events

  public func eventRequest(directory: String) -> URLRequest {
    var request = makeRequest("GET", "/api/event?\(Self.locationQuery(directory))")
    request.setValue("text/event-stream", forHTTPHeaderField: "accept")
    request.timeoutInterval = 24 * 60 * 60
    return request
  }

  private static let streamSession: URLSession = {
    let configuration = URLSessionConfiguration.ephemeral
    configuration.timeoutIntervalForRequest = 24 * 60 * 60
    configuration.timeoutIntervalForResource = 7 * 24 * 60 * 60
    configuration.requestCachePolicy = .reloadIgnoringLocalCacheData
    return URLSession(configuration: configuration)
  }()

  /// Streams server events for a folder until the consumer stops iterating, reconnecting with
  /// backoff when the connection drops. Ends with an error only for a 401.
  public func events(directory: String) -> AsyncThrowingStream<OpencodeStreamItem, Error> {
    let request = eventRequest(directory: directory)
    return AsyncThrowingStream { continuation in
      let task = Task {
        var attempt = 0
        var opened = false
        while !Task.isCancelled {
          do {
            let (bytes, response) = try await Self.streamSession.bytes(for: request)
            let status = (response as? HTTPURLResponse)?.statusCode ?? 0
            guard status == 200 else {
              throw OpencodeHTTPError(status: status, message: "OpenCode event stream unavailable")
            }
            attempt = 0
            continuation.yield(.opened(reconnected: opened))
            opened = true
            var parser = SSEParser()
            var line: [UInt8] = []
            // `AsyncBytes.lines` drops blank lines, which end SSE events, so split by hand.
            for try await byte in bytes {
              if byte == UInt8(ascii: "\n") {
                if let payload = parser.feed(line: String(decoding: line, as: UTF8.self)),
                  let event = OpencodeEvent.parse(payload)
                {
                  continuation.yield(.event(event))
                }
                line.removeAll(keepingCapacity: true)
              } else {
                line.append(byte)
              }
            }
          } catch let error as OpencodeHTTPError where error.status == 401 {
            continuation.finish(throwing: error)
            return
          } catch {
            if Task.isCancelled { break }
          }
          attempt += 1
          let delay = min(250 * (1 << min(attempt, 5)), 5_000)
          try? await Task.sleep(for: .milliseconds(delay))
        }
        continuation.finish()
      }
      continuation.onTermination = { _ in task.cancel() }
    }
  }
}

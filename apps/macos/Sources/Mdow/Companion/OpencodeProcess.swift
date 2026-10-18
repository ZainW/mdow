import Foundation
import MdowKit
import Synchronization

/// Finds the `opencode` binary the way the desktop app does: the inherited PATH plus the usual
/// install folders. A Mac app launched from Finder gets a minimal PATH, so the login shell's
/// PATH is asked for too.
enum OpencodeBinary {
  struct Resolved: Sendable {
    var path: String
    var version: String
  }

  private static let loginPath = Mutex<String??>(nil)

  /// The user's login shell PATH, read once (in a few hundred milliseconds) and cached.
  static func loginShellPath() async -> String? {
    if let cached = loginPath.withLock({ $0 }) { return cached }
    let shell = ProcessInfo.processInfo.environment["SHELL"] ?? "/bin/zsh"
    let marker = CompanionLogic.pathMarker
    let output = await run(
      shell, ["-l", "-i", "-c", "printf '\(marker)%s\(marker)' \"$PATH\""], timeout: .seconds(4))
    let path = output.flatMap(CompanionLogic.parseShellPath)
    loginPath.withLock { $0 = .some(path) }
    return path
  }

  static func searchDirectories() async -> [String] {
    CompanionLogic.searchDirectories(
      path: ProcessInfo.processInfo.environment["PATH"], home: NSHomeDirectory(),
      loginShellPath: await loginShellPath())
  }

  /// PATH for the server process, so OpenCode can find git and its own helpers.
  static func environmentPath() async -> String {
    await searchDirectories().joined(separator: ":")
  }

  static func find() async -> String? {
    for directory in await searchDirectories() {
      let candidate = (directory as NSString).appendingPathComponent("opencode")
      if FileManager.default.isExecutableFile(atPath: candidate) { return candidate }
    }
    return nil
  }

  static func detect() async -> (status: CompanionRuntimeStatus, binary: Resolved?) {
    guard let path = await find() else {
      return (
        CompanionRuntimeStatus(
          availability: .missing,
          detail: "Install OpenCode 2 from opencode.ai, sign in to a provider, then try again."),
        nil
      )
    }
    let output = await run(path, ["--version"], timeout: .seconds(5), path: await environmentPath())
    guard let version = output.flatMap(CompanionLogic.parseVersion) else {
      return (CompanionRuntimeStatus(availability: .failed, detail: "Could not run \(path) --version."), nil)
    }
    guard CompanionLogic.isSupportedVersion(version) else {
      return (
        CompanionRuntimeStatus(
          availability: .outdated, version: version,
          detail:
            "Mdow needs OpenCode \(CompanionLogic.minimumMajorVersion) or newer. Run `opencode upgrade`."),
        nil
      )
    }
    return (CompanionRuntimeStatus(availability: .available, version: version), Resolved(path: path, version: version))
  }

  /// Runs a short command and returns its standard output, or nil if it failed or timed out.
  static func run(_ executable: String, _ arguments: [String], timeout: Duration, path: String? = nil)
    async -> String?
  {
    let process = Process()
    process.executableURL = URL(fileURLWithPath: executable)
    process.arguments = arguments
    var environment = ProcessInfo.processInfo.environment
    if let path { environment["PATH"] = path }
    process.environment = environment
    let stdout = Pipe()
    process.standardOutput = stdout
    process.standardError = FileHandle.nullDevice
    process.standardInput = FileHandle.nullDevice
    let box = ProcessBox(process)
    return await withCheckedContinuation { continuation in
      let once = Mutex(false)
      @Sendable func finish(_ value: String?) {
        let first = once.withLock { done in
          defer { done = true }
          return !done
        }
        if first { continuation.resume(returning: value) }
      }
      process.terminationHandler = { process in
        let data = stdout.fileHandleForReading.readDataToEndOfFile()
        finish(process.terminationStatus == 0 ? String(decoding: data, as: UTF8.self) : nil)
      }
      do {
        try process.run()
      } catch {
        finish(nil)
        return
      }
      Task.detached {
        try? await Task.sleep(for: timeout)
        if box.process.isRunning { box.process.terminate() }
        finish(nil)
      }
    }
  }
}

/// A private `opencode serve` bound to loopback on a free port. Mdow owns this server instead
/// of borrowing the user's background service, so it controls the credentials and the
/// server's lifetime matches the app's.
/// `Process` is thread-safe for what Mdow does with it (run, terminate, isRunning).
final class ProcessBox: @unchecked Sendable {
  let process: Process
  init(_ process: Process) { self.process = process }
}

final class OpencodeServer: Sendable {
  enum StartError: LocalizedError {
    case timedOut
    case exited(code: Int32, detail: String)

    var errorDescription: String? {
      switch self {
      case .timedOut: "OpenCode did not start within 20 seconds."
      case .exited(let code, let detail): "OpenCode exited (\(code))\(detail.isEmpty ? "" : ": \(detail)")"
      }
    }
  }

  let url: URL
  /// Basic auth password, generated per launch and never written to disk.
  let password: String
  private let box: ProcessBox
  private var process: Process { box.process }
  private let exitHandlers = Mutex<[@Sendable () -> Void]>([])
  private let hasExited = Mutex(false)

  private init(url: URL, password: String, box: ProcessBox) {
    self.url = url
    self.password = password
    self.box = box
  }

  var isRunning: Bool { process.isRunning }

  /// Calls `handler` once when the process exits, whether stopped or crashed.
  func onExit(_ handler: @escaping @Sendable () -> Void) {
    let already = hasExited.withLock { exited in
      if !exited { exitHandlers.withLock { $0.append(handler) } }
      return exited
    }
    if already { handler() }
  }

  fileprivate func markExited() {
    hasExited.withLock { $0 = true }
    for handler in exitHandlers.withLock({ handlers in defer { handlers = [] }; return handlers }) {
      handler()
    }
  }

  func stop() {
    if process.isRunning { process.terminate() }
  }

  static func start(binary: String) async throws -> OpencodeServer {
    OpencodeServerCleanup.stopOrphan()
    let password = CompanionLogic.makePassword()
    let process = Process()
    process.executableURL = URL(fileURLWithPath: binary)
    process.arguments = ["serve", "--hostname=127.0.0.1", "--port=0"]
    var environment = ProcessInfo.processInfo.environment
    environment["PATH"] = await OpencodeBinary.environmentPath()
    environment["OPENCODE_SERVER_PASSWORD"] = password
    process.environment = environment
    process.currentDirectoryURL = URL(fileURLWithPath: NSHomeDirectory())
    process.standardInput = FileHandle.nullDevice
    let output = Pipe()
    process.standardOutput = output
    process.standardError = output

    final class State: Sendable {
      let output = Mutex("")
      let settled = Mutex(false)
      let server = Mutex<OpencodeServer?>(nil)
    }
    let state = State()
    let box = ProcessBox(process)

    return try await withCheckedThrowingContinuation { continuation in
      @Sendable func settle(_ result: Result<OpencodeServer, Error>) {
        let first = state.settled.withLock { settled in
          defer { settled = true }
          return !settled
        }
        guard first else { return }
        continuation.resume(with: result)
      }

      // Keep draining the pipe after start-up so the server never blocks on a full buffer.
      output.fileHandleForReading.readabilityHandler = { handle in
        let data = handle.availableData
        if data.isEmpty {
          handle.readabilityHandler = nil
          return
        }
        let text = state.output.withLock { buffer in
          buffer = String((buffer + String(decoding: data, as: UTF8.self)).suffix(8_000))
          return buffer
        }
        guard let url = CompanionLogic.parseListeningURL(text) else { return }
        let server = OpencodeServer(url: url, password: password, box: box)
        let isNew = state.server.withLock { current in
          if current != nil { return false }
          current = server
          return true
        }
        if isNew { settle(.success(server)) }
      }
      process.terminationHandler = { process in
        output.fileHandleForReading.readabilityHandler = nil
        OpencodeServerCleanup.clear(process.processIdentifier)
        state.server.withLock { $0 }?.markExited()
        let detail = state.output.withLock { $0 }
          .trimmingCharacters(in: .whitespacesAndNewlines)
          .split(separator: "\n").suffix(3).joined(separator: "\n")
        settle(.failure(StartError.exited(code: process.terminationStatus, detail: detail)))
      }
      do {
        try process.run()
        OpencodeServerCleanup.record(process.processIdentifier)
      } catch {
        settle(.failure(error))
        return
      }
      Task.detached {
        try? await Task.sleep(for: .seconds(20))
        if state.server.withLock({ $0 }) == nil {
          if box.process.isRunning { box.process.terminate() }
          settle(.failure(StartError.timedOut))
        }
      }
    }
  }
}

/// Keeps Mdow from leaving its server running when it doesn't quit normally. Signals stop the
/// server before Mdow exits; after a crash or SIGKILL, the next start stops the orphan.
enum OpencodeServerCleanup {
  private static var pidFile: URL {
    let caches = FileManager.default.urls(for: .cachesDirectory, in: .userDomainMask)[0]
    return caches.appendingPathComponent(
      Bundle.main.bundleIdentifier ?? "com.zain.mdow.Mac", isDirectory: true
    )
    .appendingPathComponent("opencode-server.pid")
  }

  private static func recordedPid() -> Int32? {
    guard let text = try? String(contentsOf: pidFile, encoding: .utf8) else { return nil }
    return Int32(text.trimmingCharacters(in: .whitespacesAndNewlines))
  }

  static func record(_ pid: Int32) {
    try? FileManager.default.createDirectory(
      at: pidFile.deletingLastPathComponent(), withIntermediateDirectories: true)
    try? String(pid).write(to: pidFile, atomically: true, encoding: .utf8)
  }

  static func clear(_ pid: Int32) {
    if recordedPid() == pid { try? FileManager.default.removeItem(at: pidFile) }
  }

  /// Stops a server recorded by an earlier Mdow that is still running without it: its parent
  /// is now launchd and it is still `opencode`, so it can't be anything else.
  static func stopOrphan() {
    guard let pid = recordedPid(), pid > 1 else { return }
    try? FileManager.default.removeItem(at: pidFile)
    var info = proc_bsdinfo()
    let size = Int32(MemoryLayout<proc_bsdinfo>.size)
    guard proc_pidinfo(pid, PROC_PIDTBSDINFO, 0, &info, size) == size, info.pbi_ppid == 1 else {
      return
    }
    var name = [CChar](repeating: 0, count: 256)
    guard proc_name(pid, &name, UInt32(name.count)) > 0,
      String(decoding: name.prefix { $0 != 0 }.map { UInt8(bitPattern: $0) }, as: UTF8.self)
        == "opencode"
    else { return }
    kill(pid, SIGTERM)
  }

  @MainActor private static var signalSources: [DispatchSourceSignal] = []

  /// On SIGTERM, SIGINT or SIGHUP, stops the server, then lets the signal end Mdow as before.
  @MainActor static func installSignalHandlers(stop: @escaping @MainActor () -> Void) {
    guard signalSources.isEmpty else { return }
    for number in [SIGTERM, SIGINT, SIGHUP] {
      signal(number, SIG_IGN)
      let source = DispatchSource.makeSignalSource(signal: number, queue: .main)
      source.setEventHandler {
        MainActor.assumeIsolated { stop() }
        signal(number, SIG_DFL)
        raise(number)
      }
      source.resume()
      signalSources.append(source)
    }
  }
}

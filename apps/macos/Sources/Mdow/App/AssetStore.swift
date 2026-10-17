import AppKit
import MdowKit

/// Images and diagrams for the renderer. Local images load on demand; remote images and
/// diagrams are fetched after the first paint and the document is rendered again.
final class AssetStore: @unchecked Sendable {
  static let shared = AssetStore()

  private let lock = NSLock()
  private var remote: [URL: AssetState] = [:]
  private var diagrams: [String: AssetState] = [:]

  private let session: URLSession = {
    let configuration = URLSessionConfiguration.default
    configuration.timeoutIntervalForRequest = 15
    configuration.urlCache = URLCache(memoryCapacity: 32 << 20, diskCapacity: 128 << 20)
    return URLSession(configuration: configuration)
  }()

  func assets(dark: Bool) -> RenderAssets {
    RenderAssets(
      image: { [self] url in image(for: url) },
      diagram: { [self] source in diagram(for: source, dark: dark) })
  }

  private func image(for url: URL) -> AssetState {
    if url.isFileURL {
      guard let image = NSImage(contentsOf: url), image.isValid else { return .failed }
      return .ready(image)
    }
    guard url.scheme == "http" || url.scheme == "https" else { return .failed }
    lock.lock()
    defer { lock.unlock() }
    return remote[url] ?? .pending
  }

  private func diagram(for source: String, dark: Bool) -> AssetState {
    lock.lock()
    defer { lock.unlock() }
    return diagrams[key(source, dark)] ?? .pending
  }

  private func key(_ source: String, _ dark: Bool) -> String { (dark ? "d:" : "l:") + source }

  /// Fetches what a render was missing. Returns true if anything new arrived.
  func fetch(images: [URL], diagrams sources: [String], dark: Bool) async -> Bool {
    await withTaskGroup(of: Bool.self) { group in
      for url in Set(images) {
        group.addTask { [self] in
          var state = AssetState.failed
          if let (data, response) = try? await session.data(from: url),
            (response as? HTTPURLResponse).map({ $0.statusCode < 400 }) ?? true,
            let image = NSImage(data: data), image.isValid
          {
            state = .ready(image)
          }
          store { remote[url] = state }
          return true
        }
      }
      for source in Set(sources) {
        group.addTask { [self] in
          var state = AssetState.failed
          if let image = try? await MermaidRenderer.shared.render(source, dark: dark) {
            state = .ready(image)
          }
          store { diagrams[key(source, dark)] = state }
          return true
        }
      }
      var changed = false
      for await result in group { changed = changed || result }
      return changed
    }
  }

  private func store(_ body: () -> Void) {
    lock.lock()
    body()
    lock.unlock()
  }
}

import AppKit
import MdowKit

@MainActor protocol DocumentPaneDelegate: AnyObject {
  func paneDidChangeDocument(_ pane: DocumentPane)
  func paneDidRender(_ pane: DocumentPane)
  func pane(_ pane: DocumentPane, didScrollToHeading index: Int?)
  func pane(_ pane: DocumentPane, openInNewTab url: URL)
  func pane(_ pane: DocumentPane, openFolder url: URL)
  func pane(_ pane: DocumentPane, openFiles urls: [URL])
}

/// One document view: a reader, the file it shows, its live reload, its rendering, and its
/// back/forward history. A window has one, or two side by side in split view.
final class DocumentPane: NSObject, ReaderDelegate {
  private struct Place {
    var url: URL
    var location: Int
  }

  weak var delegate: DocumentPaneDelegate?
  let reader = ReaderViewController()

  private(set) var fileURL: URL?
  private(set) var rendered: RenderedDocument?
  private var source: MarkdownSource?
  private var backStack: [Place] = []
  private var forwardStack: [Place] = []
  private var watcher: FileWatcher?
  private var generation = 0
  private var renderTask: Task<Void, Never>?
  private var widthTask: Task<Void, Never>?
  private var lastDark: Bool?

  var canGoBack: Bool { !backStack.isEmpty }
  var canGoForward: Bool { !forwardStack.isEmpty }
  var headings: [OutlineHeading] { reader.headings }

  override init() {
    super.init()
    reader.delegate = self
    NotificationCenter.default.addObserver(
      self, selector: #selector(styleChanged), name: .readerStyleChanged, object: nil)
  }

  /// Stops watching and rendering; called when the pane is removed.
  func close() {
    watcher?.cancel()
    watcher = nil
    renderTask?.cancel()
    widthTask?.cancel()
    NotificationCenter.default.removeObserver(self)
  }

  private var isDark: Bool { reader.isDark }

  // MARK: Opening

  /// Shows a document. Folders go to the window, since the sidebar shows them.
  func open(_ url: URL, addToHistory: Bool = true, restoring location: Int? = nil) {
    let url = url.standardizedFileURL
    var isFolder: ObjCBool = false
    let exists = FileManager.default.fileExists(atPath: url.path, isDirectory: &isFolder)
    if exists, isFolder.boolValue {
      delegate?.pane(self, openFolder: url)
      return
    }
    if url == fileURL {
      if let location { reader.scroll(toCharacter: location, animated: false) }
      return
    }
    if addToHistory, let fileURL {
      backStack.append(Place(url: fileURL, location: reader.topCharacterIndex))
      forwardStack.removeAll()
    }
    fileURL = url
    source = nil
    rendered = nil
    watcher?.cancel()
    watcher = nil
    defer { delegate?.paneDidChangeDocument(self) }

    guard exists else {
      reader.showMessage(
        title: "“\(url.lastPathComponent)” Can’t Be Found",
        message: "It may have been moved, renamed or deleted.", symbol: "questionmark.folder")
      return
    }
    Preferences.shared.noteRecent(url)
    watch(url)
    if FileKind(url: url) == .html {
      reader.showHTML(url)
      return
    }
    load(restoring: location ?? 0, initial: true)
  }

  private func watch(_ url: URL) {
    watcher = FileWatcher(url: url) { [weak self] in
      Task { @MainActor in
        guard let self, self.fileURL == url, Preferences.shared.reloadOnChange else { return }
        if FileKind(url: url) == .html {
          self.reader.reloadHTML()
        } else {
          self.load(restoring: self.reader.topCharacterIndex, initial: false)
        }
      }
    }
  }

  func reload() {
    guard let fileURL else { return }
    if FileKind(url: fileURL) == .html {
      reader.reloadHTML()
    } else {
      load(restoring: reader.topCharacterIndex, initial: false)
    }
  }

  // MARK: Rendering

  /// Reads the file and renders it. On reloads a failed read keeps what's on screen, since
  /// editors briefly remove files while saving.
  private func load(restoring location: Int, initial: Bool) {
    guard let url = fileURL else { return }
    generation += 1
    let generation = generation
    renderTask?.cancel()
    renderTask = Task {
      let result = await Task.detached(priority: .userInitiated) { () -> Result<MarkdownSource, Error> in
        Result { MarkdownSource(try Self.readText(url)) }
      }.value
      guard generation == self.generation else { return }
      switch result {
      case .success(let source):
        self.source = source
        await self.render(restoring: location, generation: generation)
      case .failure(let error):
        guard initial else { return }
        self.reader.showMessage(
          title: "“\(url.lastPathComponent)” Can’t Be Opened", message: error.localizedDescription)
        self.delegate?.paneDidRender(self)
      }
    }
  }

  nonisolated static func readText(_ url: URL) throws -> String {
    let data = try Data(contentsOf: url, options: .mappedIfSafe)
    if data.count > 64 << 20 {
      throw CocoaError(.fileReadTooLarge)
    }
    if let text = String(data: data, encoding: .utf8) { return text }
    var encoding = String.Encoding.utf8
    if let text = try? String(contentsOf: url, usedEncoding: &encoding) { return text }
    return String(decoding: data, as: UTF8.self)
  }

  /// Re-renders the current source, keeping the reading position.
  private func rerender() {
    guard source != nil, reader.mode == .markdown else { return }
    generation += 1
    let generation = generation
    let location = reader.topCharacterIndex
    renderTask?.cancel()
    renderTask = Task { await self.render(restoring: location, generation: generation) }
  }

  private func render(restoring location: Int, generation: Int) async {
    guard let source, let url = fileURL else { return }
    let style = Preferences.shared.readerStyle(columnWidth: reader.textView.currentColumnWidth)
    let dark = isDark
    lastDark = dark
    let base = url.deletingLastPathComponent()
    let assets = AssetStore.shared.assets(dark: dark)
    let rendered = await Task.detached(priority: .userInitiated) {
      MarkdownRenderer.render(source, style: style, baseURL: base, assets: assets)
    }.value
    guard generation == self.generation, !Task.isCancelled else { return }
    show(rendered, restoring: location)

    guard !rendered.pendingImages.isEmpty || !rendered.pendingDiagrams.isEmpty else { return }
    let changed = await AssetStore.shared.fetch(
      images: rendered.pendingImages, diagrams: rendered.pendingDiagrams, dark: dark)
    guard changed, generation == self.generation, !Task.isCancelled else { return }
    let again = await Task.detached(priority: .userInitiated) {
      MarkdownRenderer.render(source, style: style, baseURL: base, assets: assets)
    }.value
    guard generation == self.generation, !Task.isCancelled else { return }
    show(again, restoring: reader.topCharacterIndex)
  }

  private func show(_ rendered: RenderedDocument, restoring location: Int) {
    self.rendered = rendered
    reader.showMarkdown(rendered, restoring: location)
    delegate?.paneDidRender(self)
  }

  @objc private func styleChanged() {
    reader.textView.updateInsets()
    reader.applyZoomToHTML()
    reader.updateHTMLAppearance()
    rerender()
  }

  func appearanceChanged() {
    reader.updateHTMLAppearance()
    // Text colours are dynamic; only diagrams are drawn for one appearance.
    guard let lastDark, lastDark != isDark, source?.body.contains("mermaid") == true else { return }
    rerender()
  }

  // MARK: Reader callbacks

  func reader(_ reader: ReaderViewController, open url: URL, inNewTab: Bool) {
    route(url, inNewTab: inNewTab)
  }

  func reader(_ reader: ReaderViewController, openFiles urls: [URL]) {
    delegate?.pane(self, openFiles: urls)
  }

  func reader(_ reader: ReaderViewController, didScrollToHeading index: Int?) {
    delegate?.pane(self, didScrollToHeading: index)
  }

  func readerColumnWidthChanged(_ reader: ReaderViewController) {
    guard
      source.map({ $0.body.contains("![") || $0.body.contains("<img") || $0.body.contains("mermaid") })
        == true
    else { return }
    widthTask?.cancel()
    widthTask = Task {
      try? await Task.sleep(for: .milliseconds(150))
      guard !Task.isCancelled else { return }
      self.rerender()
    }
  }

  /// Opens a link: anchors scroll, documents open here (or in a new tab), the rest goes to
  /// the system.
  func route(_ url: URL, inNewTab: Bool) {
    if url.scheme == anchorScheme {
      _ = reader.scroll(toAnchor: url.path)
      return
    }
    if url.isFileURL {
      let target = URL(fileURLWithPath: url.path).standardizedFileURL
      if target == fileURL {
        if let fragment = url.fragment { _ = reader.scroll(toAnchor: fragment) }
        return
      }
      var isFolder: ObjCBool = false
      let exists = FileManager.default.fileExists(atPath: target.path, isDirectory: &isFolder)
      if exists, !isFolder.boolValue, FileKind.isOpenable(target) {
        if inNewTab {
          delegate?.pane(self, openInNewTab: target)
        } else {
          open(target)
        }
        if let fragment = url.fragment, !inNewTab {
          Task {
            try? await Task.sleep(for: .milliseconds(250))
            _ = self.reader.scroll(toAnchor: fragment)
          }
        }
        return
      }
      if exists, isFolder.boolValue {
        NSWorkspace.shared.activateFileViewerSelecting([target])
        return
      }
      if !exists {
        NSSound.beep()
        return
      }
    }
    NSWorkspace.shared.open(url)
  }

  // MARK: History and headings

  func goBack() {
    guard let place = backStack.popLast(), let fileURL else { return }
    forwardStack.append(Place(url: fileURL, location: reader.topCharacterIndex))
    open(place.url, addToHistory: false, restoring: place.location)
  }

  func goForward() {
    guard let place = forwardStack.popLast(), let fileURL else { return }
    backStack.append(Place(url: fileURL, location: reader.topCharacterIndex))
    open(place.url, addToHistory: false, restoring: place.location)
  }

  func jumpHeading(by step: Int) {
    let headings = reader.headings
    guard !headings.isEmpty else { return }
    let top = reader.topCharacterIndex
    let index: Int?
    if step > 0 {
      index = headings.firstIndex { $0.location > top + 1 }
    } else {
      index = headings.lastIndex { $0.location < max(top - 1, 0) }
    }
    if let index { reader.scroll(toHeading: index) } else if step < 0 { reader.scrollToTop() }
  }
}

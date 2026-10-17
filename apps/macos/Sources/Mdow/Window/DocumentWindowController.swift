import AppKit
import MdowKit
import UniformTypeIdentifiers
import WebKit

/// One window (or tab): a sidebar and a reader showing one document at a time, with
/// back/forward history like Finder or Safari.
final class DocumentWindowController: NSWindowController, NSWindowDelegate, NSToolbarDelegate,
  NSMenuItemValidation, NSToolbarItemValidation, NSSharingServicePickerToolbarItemDelegate,
  ReaderDelegate, SidebarDelegate
{
  private struct Place {
    var url: URL
    var location: Int
  }

  private let split = NSSplitViewController()
  private let sidebar = SidebarViewController()
  let reader = ReaderViewController()
  private var sidebarItem: NSSplitViewItem!

  private(set) var fileURL: URL?
  /// A folder the user opened; without one, the sidebar shows the document's own folder.
  private(set) var openedFolder: URL?
  private var scannedFolder: URL?
  private(set) var folderTree: FolderNode?
  private var backStack: [Place] = []
  private var forwardStack: [Place] = []
  private var watcher: FileWatcher?
  private var source: MarkdownSource?
  private var rendered: RenderedDocument?
  private var generation = 0
  private var renderTask: Task<Void, Never>?
  private var widthTask: Task<Void, Never>?
  private var appearanceObservation: NSKeyValueObservation?
  private var lastDark: Bool?

  var isEmpty: Bool { fileURL == nil }

  init() {
    let window = NSWindow(
      contentRect: NSRect(x: 0, y: 0, width: 1080, height: 760),
      styleMask: [.titled, .closable, .miniaturizable, .resizable, .fullSizeContentView],
      backing: .buffered, defer: false)
    window.minSize = NSSize(width: 520, height: 360)
    window.tabbingIdentifier = "MdowDocument"
    window.tabbingMode = .automatic
    window.toolbarStyle = .unified
    window.titleVisibility = .visible
    window.isRestorable = true
    window.restorationClass = WindowRestoration.self
    window.identifier = .init("MdowDocumentWindow")
    window.setFrameAutosaveName("")
    super.init(window: window)

    sidebar.delegate = self
    reader.delegate = self
    sidebarItem = NSSplitViewItem(sidebarWithViewController: sidebar)
    sidebarItem.minimumThickness = 200
    sidebarItem.maximumThickness = 420
    sidebarItem.preferredThicknessFraction = 0.2
    sidebarItem.canCollapse = true
    sidebarItem.allowsFullHeightLayout = true
    let readerItem = NSSplitViewItem(viewController: reader)
    readerItem.minimumThickness = 320
    split.addSplitViewItem(sidebarItem)
    split.addSplitViewItem(readerItem)
    split.splitView.autosaveName = "MdowSidebar"
    window.contentViewController = split
    window.setContentSize(NSSize(width: 1080, height: 760))

    let toolbar = NSToolbar(identifier: "MdowDocumentToolbar")
    toolbar.delegate = self
    toolbar.displayMode = .iconOnly
    toolbar.allowsUserCustomization = true
    toolbar.autosavesConfiguration = true
    window.toolbar = toolbar
    window.delegate = self

    if !window.setFrameUsingName("MdowDocument") { window.center() }
    updateTitle()

    NotificationCenter.default.addObserver(
      self, selector: #selector(styleChanged), name: .readerStyleChanged, object: nil)
    appearanceObservation = window.observe(\.effectiveAppearance) { [weak self] _, _ in
      MainActor.assumeIsolated { self?.appearanceChanged() }
    }
  }

  required init?(coder: NSCoder) { fatalError() }

  deinit {
    watcher?.cancel()
  }

  private var isDark: Bool {
    window?.effectiveAppearance.bestMatch(from: [.aqua, .darkAqua]) == .darkAqua
  }

  // MARK: Opening

  /// Shows `url` in this window. Folders become the sidebar's folder.
  func open(_ url: URL, addToHistory: Bool = true, restoring location: Int? = nil) {
    let url = url.standardizedFileURL
    var isFolder: ObjCBool = false
    let exists = FileManager.default.fileExists(atPath: url.path, isDirectory: &isFolder)
    if exists, isFolder.boolValue {
      openFolder(at: url)
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
    updateTitle()
    sidebar.setCurrentFile(url)
    window?.invalidateRestorableState()

    guard exists else {
      reader.showMessage(
        title: "“\(url.lastPathComponent)” Can’t Be Found",
        message: "It may have been moved, renamed or deleted.", symbol: "questionmark.folder")
      sidebar.setHeadings([])
      return
    }
    Preferences.shared.noteRecent(url)
    watch(url)
    updateFolder()

    if FileKind(url: url) == .html {
      reader.showHTML(url)
      sidebar.setHeadings([])
      return
    }
    load(restoring: location ?? 0, initial: true)
  }

  func openFolder(at url: URL) {
    openedFolder = url.standardizedFileURL
    window?.invalidateRestorableState()
    updateFolder { [weak self] tree in
      guard let self else { return }
      // An empty window opens the folder's README.
      if self.fileURL == nil, let readme = tree.children.first(where: {
        !$0.isFolder && $0.url.deletingPathExtension().lastPathComponent.lowercased() == "readme"
      }) {
        self.open(readme.url)
      }
    }
    sidebar.setMode(.files)
    if sidebarItem.isCollapsed { sidebarItem.animator().isCollapsed = false }
    updateTitle()
  }

  private func updateFolder(then completion: ((FolderNode) -> Void)? = nil) {
    guard let folder = openedFolder ?? fileURL?.deletingLastPathComponent() else {
      sidebar.setFolder(nil)
      return
    }
    if folder == scannedFolder, let folderTree {
      completion?(folderTree)
      return
    }
    scannedFolder = folder
    Task {
      let tree = await Task.detached(priority: .userInitiated) { FolderNode.scan(folder) }.value
      guard folder == self.scannedFolder else { return }
      self.folderTree = tree
      self.sidebar.setFolder(tree)
      completion?(tree)
    }
  }

  private func rescanFolder() {
    scannedFolder = nil
    updateFolder()
  }

  private func watch(_ url: URL) {
    watcher = FileWatcher(url: url) { [weak self] in
      Task { @MainActor in
        guard let self, self.fileURL == url, Preferences.shared.reloadOnChange else { return }
        if FileKind(url: url) == .html {
          self.reader.showHTML(url)
        } else {
          self.load(restoring: self.reader.topCharacterIndex, initial: false)
        }
      }
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
        self.sidebar.setHeadings([])
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
    sidebar.setHeadings(rendered.headings)
    reader.showMarkdown(rendered, restoring: location)
    updateTitle()
  }

  @objc private func styleChanged() {
    reader.textView.updateInsets()
    reader.applyZoomToHTML()
    rerender()
  }

  private func appearanceChanged() {
    // Text colours are dynamic; only diagrams are drawn for one appearance.
    guard let lastDark, lastDark != isDark, source?.body.contains("mermaid") == true else { return }
    rerender()
  }

  func readerColumnWidthChanged(_ reader: ReaderViewController) {
    guard source.map({ $0.body.contains("![") || $0.body.contains("<img") || $0.body.contains("mermaid") })
      == true
    else { return }
    widthTask?.cancel()
    widthTask = Task {
      try? await Task.sleep(for: .milliseconds(150))
      guard !Task.isCancelled else { return }
      self.rerender()
    }
  }

  // MARK: Title

  private func updateTitle() {
    guard let window else { return }
    if let fileURL {
      window.title = FileManager.default.displayName(atPath: fileURL.path)
      window.subtitle = fileURL.deletingLastPathComponent().abbreviatedPath
      window.representedURL = fileURL
    } else if let openedFolder {
      window.title = openedFolder.lastPathComponent
      window.subtitle = ""
      window.representedURL = openedFolder
    } else {
      window.title = "Mdow"
      window.subtitle = ""
      window.representedURL = nil
    }
    window.tab.title = window.title
    if let title = rendered?.title, title != window.title { window.tab.toolTip = title }
  }

  // MARK: Reader and sidebar callbacks

  func reader(_ reader: ReaderViewController, open url: URL, inNewTab: Bool) {
    route(url, inNewTab: inNewTab)
  }

  func reader(_ reader: ReaderViewController, openFiles urls: [URL]) {
    AppDelegate.shared.open(urls, from: self)
  }

  func reader(_ reader: ReaderViewController, didScrollToHeading index: Int?) {
    sidebar.setCurrentHeading(index)
  }

  func sidebar(_ sidebar: SidebarViewController, didSelectHeading index: Int) {
    reader.scroll(toHeading: index)
  }

  func sidebar(_ sidebar: SidebarViewController, open url: URL, inNewTab: Bool) {
    if inNewTab {
      AppDelegate.shared.openInNewTab(url, from: self)
    } else {
      open(url)
    }
  }

  func sidebarModeChanged(_ sidebar: SidebarViewController) {
    if sidebar.mode == .outline {
      sidebar.setCurrentHeading(nil)
      rerunHeadingTracking()
    }
  }

  private func rerunHeadingTracking() {
    reader.refreshHeadingTracking()
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
          AppDelegate.shared.openInNewTab(target, from: self)
        } else {
          open(target)
        }
        if let fragment = url.fragment {
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

  // MARK: Actions

  @objc func openDocument(_ sender: Any?) {
    let panel = NSOpenPanel()
    panel.allowsMultipleSelection = true
    panel.canChooseDirectories = false
    panel.allowedContentTypes = AppDelegate.openableTypes
    if let folder = openedFolder ?? fileURL?.deletingLastPathComponent() { panel.directoryURL = folder }
    guard let window else { return }
    panel.beginSheetModal(for: window) { response in
      guard response == .OK else { return }
      AppDelegate.shared.open(panel.urls, from: self)
    }
  }

  @objc func openFolder(_ sender: Any?) {
    let panel = NSOpenPanel()
    panel.canChooseFiles = false
    panel.canChooseDirectories = true
    panel.prompt = "Open Folder"
    guard let window else { return }
    panel.beginSheetModal(for: window) { response in
      guard response == .OK, let url = panel.url else { return }
      self.openFolder(at: url)
    }
  }

  @objc override func newWindowForTab(_ sender: Any?) {
    AppDelegate.shared.newTab(from: self)
  }

  @objc func reloadDocument(_ sender: Any?) {
    rescanFolder()
    guard let fileURL else { return }
    if FileKind(url: fileURL) == .html {
      reader.showHTML(fileURL)
    } else {
      load(restoring: reader.topCharacterIndex, initial: false)
    }
  }

  @objc func showInFinder(_ sender: Any?) {
    if let url = fileURL ?? openedFolder { NSWorkspace.shared.activateFileViewerSelecting([url]) }
  }

  @objc func openWithDefaultEditor(_ sender: Any?) {
    guard let fileURL else { return }
    let ours = Bundle.main.bundleURL
    let apps = NSWorkspace.shared.urlsForApplications(toOpen: fileURL).filter { $0 != ours }
    let preferred = ["com.apple.TextEdit"]
    let app =
      apps.first { url in
        !preferred.contains(Bundle(url: url)?.bundleIdentifier ?? "")
          && Bundle(url: url)?.bundleIdentifier?.hasPrefix("com.zain.mdow") != true
      } ?? apps.first
    if let app {
      NSWorkspace.shared.open([fileURL], withApplicationAt: app, configuration: NSWorkspace.OpenConfiguration())
    } else {
      NSWorkspace.shared.open(fileURL)
    }
  }

  @objc func copyDocumentPath(_ sender: Any?) {
    guard let fileURL else { return }
    NSPasteboard.general.clearContents()
    NSPasteboard.general.setString(fileURL.path, forType: .string)
  }

  @objc func goBack(_ sender: Any?) {
    guard let place = backStack.popLast(), let fileURL else { return }
    forwardStack.append(Place(url: fileURL, location: reader.topCharacterIndex))
    open(place.url, addToHistory: false, restoring: place.location)
  }

  @objc func goForward(_ sender: Any?) {
    guard let place = forwardStack.popLast(), let fileURL else { return }
    backStack.append(Place(url: fileURL, location: reader.topCharacterIndex))
    open(place.url, addToHistory: false, restoring: place.location)
  }

  @objc func nextHeading(_ sender: Any?) { jumpHeading(by: 1) }
  @objc func previousHeading(_ sender: Any?) { jumpHeading(by: -1) }

  private func jumpHeading(by step: Int) {
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

  @objc func zoomIn(_ sender: Any?) { Preferences.shared.zoomIn() }
  @objc func zoomOut(_ sender: Any?) { Preferences.shared.zoomOut() }
  @objc func actualSize(_ sender: Any?) { Preferences.shared.resetZoom() }

  @objc func setReadingWidth(_ sender: NSMenuItem) {
    if let width = ReadingWidth(rawValue: sender.representedObject as? String ?? "") {
      Preferences.shared.readingWidth = width
    }
  }

  @objc func showOutline(_ sender: Any?) { showSidebar(.outline) }
  @objc func showFiles(_ sender: Any?) { showSidebar(.files) }
  @objc func showRecents(_ sender: Any?) { showSidebar(.recents) }

  private func showSidebar(_ mode: SidebarMode) {
    if sidebarItem.isCollapsed { sidebarItem.animator().isCollapsed = false }
    sidebar.setMode(mode)
    sidebar.focus()
  }

  @objc func quickOpen(_ sender: Any?) {
    guard let window else { return }
    CommandPalette.present(for: self, in: window)
  }

  /// Find goes to the document even when the sidebar has focus.
  @objc override func performTextFinderAction(_ sender: Any?) {
    guard reader.mode == .markdown else { return }
    reader.textView.performTextFinderAction(sender)
  }

  @objc func printDocument(_ sender: Any?) {
    runPrint(saveTo: nil)
  }

  @objc func exportAsPDF(_ sender: Any?) {
    guard let window, let fileURL else { return }
    let panel = NSSavePanel()
    panel.allowedContentTypes = [.pdf]
    panel.nameFieldStringValue = fileURL.deletingPathExtension().lastPathComponent + ".pdf"
    panel.directoryURL = fileURL.deletingLastPathComponent()
    panel.beginSheetModal(for: window) { response in
      guard response == .OK, let url = panel.url else { return }
      self.runPrint(saveTo: url)
    }
  }

  private func runPrint(saveTo url: URL?) {
    guard let window else { return }
    let info = NSPrintInfo.shared.copy() as! NSPrintInfo
    info.horizontalPagination = .fit
    info.verticalPagination = .automatic
    info.isHorizontallyCentered = false
    info.isVerticallyCentered = false
    info.topMargin = 54
    info.bottomMargin = 54
    info.leftMargin = 54
    info.rightMargin = 54
    if let url {
      info.jobDisposition = .save
      info.dictionary()[NSPrintInfo.AttributeKey.jobSavingURL] = url
    }
    let width = info.paperSize.width - info.leftMargin - info.rightMargin
    let operation: NSPrintOperation
    if reader.mode == .html, let web = reader.printableView(width: width) as? WKWebView {
      operation = web.printOperation(with: info)
      operation.view?.frame = NSRect(x: 0, y: 0, width: width, height: 100)
    } else if let view = reader.printableView(width: width) {
      operation = NSPrintOperation(view: view, printInfo: info)
    } else {
      NSSound.beep()
      return
    }
    operation.jobTitle = window.title
    operation.showsPrintPanel = url == nil
    operation.showsProgressPanel = true
    operation.runModal(for: window, delegate: nil, didRun: nil, contextInfo: nil)
  }

  // MARK: Validation

  func validateMenuItem(_ item: NSMenuItem) -> Bool {
    switch item.action {
    case #selector(goBack(_:)): return !backStack.isEmpty
    case #selector(goForward(_:)): return !forwardStack.isEmpty
    case #selector(reloadDocument(_:)), #selector(showInFinder(_:)):
      return fileURL != nil || openedFolder != nil
    case #selector(openWithDefaultEditor(_:)), #selector(copyDocumentPath(_:)):
      return fileURL != nil
    case #selector(printDocument(_:)), #selector(exportAsPDF(_:)):
      return reader.mode == .markdown || reader.mode == .html
    case #selector(performTextFinderAction(_:)), #selector(nextHeading(_:)), #selector(previousHeading(_:)):
      return reader.mode == .markdown
    case #selector(zoomIn(_:)): return Preferences.shared.canZoomIn
    case #selector(zoomOut(_:)): return Preferences.shared.canZoomOut
    case #selector(actualSize(_:)): return Preferences.shared.zoom != 1
    case #selector(setReadingWidth(_:)):
      item.state = (item.representedObject as? String) == Preferences.shared.readingWidth.rawValue ? .on : .off
      return true
    case #selector(showOutline(_:)):
      item.state = sidebar.mode == .outline && !sidebarItem.isCollapsed ? .on : .off
      return true
    case #selector(showFiles(_:)):
      item.state = sidebar.mode == .files && !sidebarItem.isCollapsed ? .on : .off
      return true
    case #selector(showRecents(_:)):
      item.state = sidebar.mode == .recents && !sidebarItem.isCollapsed ? .on : .off
      return true
    default: return true
    }
  }

  func validateToolbarItem(_ item: NSToolbarItem) -> Bool {
    switch item.action {
    case #selector(goBack(_:)): !backStack.isEmpty
    case #selector(goForward(_:)): !forwardStack.isEmpty
    default: true
    }
  }

  // MARK: Toolbar

  private static let backItem = NSToolbarItem.Identifier("back")
  private static let forwardItem = NSToolbarItem.Identifier("forward")
  private static let quickOpenItem = NSToolbarItem.Identifier("quickOpen")
  private static let shareItem = NSToolbarItem.Identifier("share")

  func toolbarDefaultItemIdentifiers(_ toolbar: NSToolbar) -> [NSToolbarItem.Identifier] {
    [.toggleSidebar, .sidebarTrackingSeparator, Self.backItem, Self.forwardItem, .flexibleSpace,
     Self.quickOpenItem, Self.shareItem]
  }

  func toolbarAllowedItemIdentifiers(_ toolbar: NSToolbar) -> [NSToolbarItem.Identifier] {
    toolbarDefaultItemIdentifiers(toolbar) + [.space, .flexibleSpace]
  }

  func toolbar(
    _ toolbar: NSToolbar, itemForItemIdentifier identifier: NSToolbarItem.Identifier,
    willBeInsertedIntoToolbar flag: Bool
  ) -> NSToolbarItem? {
    func button(_ label: String, _ symbol: String, _ action: Selector) -> NSToolbarItem {
      let item = NSToolbarItem(itemIdentifier: identifier)
      item.label = label
      item.paletteLabel = label
      item.toolTip = label
      item.image = NSImage(systemSymbolName: symbol, accessibilityDescription: label)
      item.isBordered = true
      item.target = self
      item.action = action
      return item
    }
    switch identifier {
    case Self.backItem:
      let item = button("Back", "chevron.left", #selector(goBack(_:)))
      item.isNavigational = true
      return item
    case Self.forwardItem:
      let item = button("Forward", "chevron.right", #selector(goForward(_:)))
      item.isNavigational = true
      return item
    case Self.quickOpenItem:
      return button("Quick Open", "magnifyingglass", #selector(quickOpen(_:)))
    case Self.shareItem:
      let item = NSSharingServicePickerToolbarItem(itemIdentifier: identifier)
      item.delegate = self
      item.label = "Share"
      item.toolTip = "Share"
      return item
    default:
      return nil
    }
  }

  func items(for pickerToolbarItem: NSSharingServicePickerToolbarItem) -> [Any] {
    fileURL.map { [$0] } ?? []
  }

  // MARK: Window

  func windowWillClose(_ notification: Notification) {
    watcher?.cancel()
    renderTask?.cancel()
    AppDelegate.shared.controllerClosed(self)
  }

  func windowDidEndLiveResize(_ notification: Notification) {
    window?.saveFrame(usingName: "MdowDocument")
  }

  func windowDidMove(_ notification: Notification) {
    window?.saveFrame(usingName: "MdowDocument")
  }

  func windowDidBecomeKey(_ notification: Notification) {
    if reader.mode == .markdown, window?.firstResponder === window {
      window?.makeFirstResponder(reader.textView)
    }
  }

  // MARK: State restoration

  func window(_ window: NSWindow, willEncodeRestorableState state: NSCoder) {
    state.encode(fileURL?.path as NSString?, forKey: "file")
    state.encode(openedFolder?.path as NSString?, forKey: "folder")
    state.encode(reader.topCharacterIndex, forKey: "location")
  }

  func window(_ window: NSWindow, didDecodeRestorableState state: NSCoder) {
    if let folder = state.decodeObject(of: NSString.self, forKey: "folder") as String? {
      openedFolder = URL(fileURLWithPath: folder)
      updateFolder()
    }
    if let file = state.decodeObject(of: NSString.self, forKey: "file") as String? {
      open(URL(fileURLWithPath: file), addToHistory: false, restoring: state.decodeInteger(forKey: "location"))
    }
    updateTitle()
  }

  // MARK: Palette

  /// What Quick Open offers for this window.
  func paletteItems() -> [PaletteItem] {
    var items: [PaletteItem] = []
    for (index, heading) in reader.headings.enumerated() {
      items.append(
        PaletteItem(
          section: .headings, title: String(repeating: "  ", count: heading.level - 1) + heading.title,
          detail: "",
          symbol: "number"
        ) { [weak self] in self?.reader.scroll(toHeading: index) })
    }
    let root = openedFolder ?? scannedFolder
    for url in folderTree?.allFiles.prefix(5000) ?? [] where url != fileURL {
      var detail = url.deletingLastPathComponent().path
      if let root, detail.hasPrefix(root.path) {
        detail = String(detail.dropFirst(root.path.count)).trimmingCharacters(in: CharacterSet(charactersIn: "/"))
      }
      items.append(
        PaletteItem(section: .files, title: url.lastPathComponent, detail: detail, symbol: "doc.text") {
          [weak self] in self?.open(url)
        })
    }
    let inFolder = Set(folderTree?.allFiles ?? [])
    for url in Preferences.shared.recents.prefix(15) where url != fileURL && !inFolder.contains(url) {
      items.append(
        PaletteItem(
          section: .recents, title: url.lastPathComponent,
          detail: url.deletingLastPathComponent().abbreviatedPath, symbol: "clock"
        ) { [weak self] in self?.open(url) })
    }
    func command(_ title: String, _ symbol: String, _ keys: String = "", _ action: Selector) {
      items.append(
        PaletteItem(section: .commands, title: title, detail: keys, symbol: symbol) {
          NSApp.sendAction(action, to: nil, from: nil)
        })
    }
    command("Open File…", "doc", "⌘O", #selector(openDocument(_:)))
    command("Open Folder…", "folder", "⇧⌘O", #selector(openFolder(_:)))
    command("New Tab", "plus.square.on.square", "⌘T", #selector(AppDelegate.newTabAction(_:)))
    command("Find in Document", "text.magnifyingglass", "⌘F", #selector(findAction))
    command("Toggle Sidebar", "sidebar.left", "⌃⌘S", #selector(NSSplitViewController.toggleSidebar(_:)))
    command("Show Outline", "list.bullet.indent", "⌃⌘1", #selector(showOutline(_:)))
    command("Show Files", "folder", "⌃⌘2", #selector(showFiles(_:)))
    command("Show Recents", "clock", "⌃⌘3", #selector(showRecents(_:)))
    command("Zoom In", "plus.magnifyingglass", "⌘+", #selector(zoomIn(_:)))
    command("Zoom Out", "minus.magnifyingglass", "⌘−", #selector(zoomOut(_:)))
    command("Actual Size", "1.magnifyingglass", "⌘0", #selector(actualSize(_:)))
    command("Reload", "arrow.clockwise", "⌘R", #selector(reloadDocument(_:)))
    command("Show in Finder", "finder", "⌥⌘R", #selector(showInFinder(_:)))
    command("Open in Editor", "square.and.pencil", "⇧⌘E", #selector(openWithDefaultEditor(_:)))
    command("Print…", "printer", "⌘P", #selector(printDocument(_:)))
    command("Export as PDF…", "doc.richtext", "", #selector(exportAsPDF(_:)))
    command("Settings…", "gearshape", "⌘,", #selector(AppDelegate.showSettings(_:)))
    command("Keyboard Shortcuts", "keyboard", "⌘/", #selector(AppDelegate.showShortcuts(_:)))
    for mode in AppearanceMode.allCases {
      items.append(
        PaletteItem(section: .commands, title: "Appearance: \(mode.displayName)", detail: "", symbol: "circle.lefthalf.filled") {
          Preferences.shared.appearance = mode
        })
    }
    for width in ReadingWidth.allCases {
      items.append(
        PaletteItem(section: .commands, title: "Text Width: \(width.displayName)", detail: "", symbol: "arrow.left.and.right") {
          Preferences.shared.readingWidth = width
        })
    }
    return items
  }

  @objc private func findAction() {
    window?.makeFirstResponder(reader.textView)
    let item = NSMenuItem()
    item.tag = NSTextFinder.Action.showFindInterface.rawValue
    reader.textView.performTextFinderAction(item)
  }
}

/// Recreates document windows when the app relaunches.
final class WindowRestoration: NSObject, NSWindowRestoration {
  static func restoreWindow(
    withIdentifier identifier: NSUserInterfaceItemIdentifier, state: NSCoder,
    completionHandler: @escaping (NSWindow?, (any Error)?) -> Void
  ) {
    MainActor.assumeIsolated {
      let controller = AppDelegate.shared.makeController()
      completionHandler(controller.window, nil)
    }
  }
}

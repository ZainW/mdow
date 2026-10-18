import AppKit
import MdowKit
import UniformTypeIdentifiers
import WebKit

/// One window (or tab): a sidebar and one document pane, or two side by side in split view.
/// Each pane keeps its own back/forward history, like Finder or Safari.
final class DocumentWindowController: NSWindowController, NSWindowDelegate, NSToolbarDelegate,
  NSMenuItemValidation, NSToolbarItemValidation, NSSharingServicePickerToolbarItemDelegate,
  DocumentPaneDelegate, SidebarDelegate
{
  private let split = NSSplitViewController()
  private let paneSplit = NSSplitViewController()
  private let sidebar = SidebarViewController()
  private var sidebarItem: NSSplitViewItem!
  let companion = CompanionViewController()
  private(set) var companionItem: NSSplitViewItem!

  private(set) var panes: [DocumentPane] = []
  /// The pane the sidebar, title and menu commands follow: the one last clicked.
  private(set) var activePane: DocumentPane!
  /// A folder the user opened; without one, the sidebar shows the document's own folder.
  private(set) var openedFolder: URL?
  private var scannedFolder: URL?
  private(set) var folderTree: FolderNode?
  private var appearanceObservation: NSKeyValueObservation?
  private var clickMonitor: Any?

  var reader: ReaderViewController { activePane.reader }
  var fileURL: URL? { activePane.fileURL }
  var isEmpty: Bool { panes.count == 1 && activePane.fileURL == nil }
  /// Every document showing in this window.
  var openURLs: [URL] { panes.compactMap(\.fileURL) }

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
    super.init(window: window)

    let pane = makePane()
    panes = [pane]
    activePane = pane

    sidebar.delegate = self
    sidebarItem = NSSplitViewItem(sidebarWithViewController: sidebar)
    sidebarItem.minimumThickness = 200
    sidebarItem.maximumThickness = 420
    sidebarItem.preferredThicknessFraction = 0.2
    sidebarItem.canCollapse = true
    sidebarItem.allowsFullHeightLayout = true

    paneSplit.splitView.isVertical = true
    paneSplit.splitView.dividerStyle = .thin
    let paneItem = NSSplitViewItem(viewController: pane.reader)
    paneItem.minimumThickness = 300
    paneSplit.addSplitViewItem(paneItem)

    let contentItem = NSSplitViewItem(viewController: paneSplit)
    contentItem.minimumThickness = 320
    split.addSplitViewItem(sidebarItem)
    split.addSplitViewItem(contentItem)
    companionItem = companion.makeSplitItem(context: self)
    split.addSplitViewItem(companionItem)
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

    appearanceObservation = window.observe(\.effectiveAppearance) { [weak self] _, _ in
      MainActor.assumeIsolated { self?.panes.forEach { $0.appearanceChanged() } }
    }
    // A click in a pane makes it the one the sidebar and commands follow.
    clickMonitor = NSEvent.addLocalMonitorForEvents(matching: [.leftMouseDown, .rightMouseDown]) {
      [weak self] event in
      MainActor.assumeIsolated {
        guard let self, event.window === self.window, self.panes.count > 1 else { return }
        for pane in self.panes {
          let view = pane.reader.view
          if view.bounds.contains(view.convert(event.locationInWindow, from: nil)) {
            self.setActive(pane)
          }
        }
      }
      return event
    }
  }

  required init?(coder: NSCoder) { fatalError() }

  private func makePane() -> DocumentPane {
    let pane = DocumentPane()
    pane.delegate = self
    return pane
  }

  private func setActive(_ pane: DocumentPane) {
    guard activePane !== pane else { return }
    activePane = pane
    sidebar.setCurrentFile(pane.fileURL)
    sidebar.setHeadings(pane.headings)
    pane.reader.refreshHeadingTracking()
    updateFolder()
    updateTitle()
    window?.toolbar?.validateVisibleItems()
  }

  // MARK: Opening

  /// Shows `url` in the active pane. Folders become the sidebar's folder.
  func open(_ url: URL, addToHistory: Bool = true, restoring location: Int? = nil) {
    activePane.open(url, addToHistory: addToHistory, restoring: location)
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

  // MARK: Panes

  func paneDidChangeDocument(_ pane: DocumentPane) {
    window?.invalidateRestorableState()
    guard pane === activePane else { return }
    sidebar.setCurrentFile(pane.fileURL)
    if pane.reader.mode != .markdown { sidebar.setHeadings([]) }
    updateFolder()
    updateTitle()
  }

  func paneDidRender(_ pane: DocumentPane) {
    guard pane === activePane else { return }
    sidebar.setHeadings(pane.headings)
    pane.reader.refreshHeadingTracking()
    updateTitle()
  }

  func pane(_ pane: DocumentPane, didScrollToHeading index: Int?) {
    if pane === activePane { sidebar.setCurrentHeading(index) }
  }

  func pane(_ pane: DocumentPane, openInNewTab url: URL) {
    AppDelegate.shared.openInNewTab(url, from: self)
  }

  func pane(_ pane: DocumentPane, openFolder url: URL) {
    openFolder(at: url)
  }

  func pane(_ pane: DocumentPane, openFiles urls: [URL]) {
    // Dropping one document on a pane shows it there; more open as tabs.
    if urls.count == 1, FileKind.isOpenable(urls[0]) {
      setActive(pane)
      pane.open(urls[0])
      return
    }
    AppDelegate.shared.open(urls, from: self)
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
    companion.documentContextChanged()
    window.tab.toolTip = activePane.rendered?.title
  }

  // MARK: Sidebar callbacks

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
      reader.refreshHeadingTracking()
    }
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
    activePane.reload()
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

  @objc func goBack(_ sender: Any?) { activePane.goBack() }
  @objc func goForward(_ sender: Any?) { activePane.goForward() }
  @objc func nextHeading(_ sender: Any?) { activePane.jumpHeading(by: 1) }
  @objc func previousHeading(_ sender: Any?) { activePane.jumpHeading(by: -1) }

  /// ⌘\ shows the document side by side with a second view of it, and hides that again.
  @objc func toggleSplitView(_ sender: Any?) {
    if panes.count > 1 {
      closePane(activePane === panes[0] ? panes[1] : activePane)
      return
    }
    let pane = makePane()
    panes.append(pane)
    let item = NSSplitViewItem(viewController: pane.reader)
    item.minimumThickness = 300
    paneSplit.addSplitViewItem(item)
    if let url = panes[0].fileURL {
      pane.open(url, addToHistory: false, restoring: panes[0].reader.topCharacterIndex)
    }
    setActive(pane)
    window?.invalidateRestorableState()
  }

  private func closePane(_ pane: DocumentPane) {
    guard panes.count > 1, let index = panes.firstIndex(where: { $0 === pane }) else { return }
    pane.close()
    panes.remove(at: index)
    if let item = paneSplit.splitViewItems.first(where: { $0.viewController === pane.reader }) {
      paneSplit.removeSplitViewItem(item)
    }
    setActive(panes[0])
    window?.invalidateRestorableState()
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

  /// Find goes to the document even when the sidebar has focus, and HTML pages get their own
  /// find bar since `NSTextFinder` can't search a web view.
  @objc override func performTextFinderAction(_ sender: Any?) {
    switch reader.mode {
    case .markdown:
      reader.textView.performTextFinderAction(sender)
    case .html:
      let tag = (sender as? NSValidatedUserInterfaceItem)?.tag ?? NSTextFinder.Action.showFindInterface.rawValue
      reader.performHTMLFind(NSTextFinder.Action(rawValue: tag) ?? .showFindInterface)
    default:
      break
    }
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
    case #selector(goBack(_:)): return activePane.canGoBack
    case #selector(goForward(_:)): return activePane.canGoForward
    case #selector(reloadDocument(_:)), #selector(showInFinder(_:)):
      return fileURL != nil || openedFolder != nil
    case #selector(openWithDefaultEditor(_:)), #selector(copyDocumentPath(_:)):
      return fileURL != nil
    case #selector(printDocument(_:)), #selector(exportAsPDF(_:)):
      return reader.mode == .markdown || reader.mode == .html
    case #selector(performTextFinderAction(_:)):
      return reader.mode == .markdown || reader.mode == .html
    case #selector(nextHeading(_:)), #selector(previousHeading(_:)):
      return reader.mode == .markdown
    case #selector(toggleSplitView(_:)):
      item.title = panes.count > 1 ? "Close Split View" : "Split View"
      return true
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
    case #selector(toggleCompanion(_:)):
      item.title = companionItem.isCollapsed ? "Show Companion" : "Hide Companion"
      return true
    default: return true
    }
  }

  func validateToolbarItem(_ item: NSToolbarItem) -> Bool {
    switch item.action {
    case #selector(goBack(_:)): activePane.canGoBack
    case #selector(goForward(_:)): activePane.canGoForward
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
     Self.quickOpenItem, Self.shareItem, .inspectorTrackingSeparator, .flexibleSpace,
     CompanionViewController.toolbarItemIdentifier]
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
    case CompanionViewController.toolbarItemIdentifier:
      return companion.makeToolbarItem(target: self, action: #selector(toggleCompanion(_:)))
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
    panes.forEach { $0.close() }
    if let clickMonitor { NSEvent.removeMonitor(clickMonitor) }
    clickMonitor = nil
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
    state.encode(openedFolder?.path as NSString?, forKey: "folder")
    state.encode(panes.map { ($0.fileURL?.path ?? "") as NSString } as NSArray, forKey: "files")
    state.encode(panes.map { NSNumber(value: $0.reader.topCharacterIndex) } as NSArray, forKey: "locations")
    state.encode(panes.firstIndex { $0 === activePane } ?? 0, forKey: "active")
  }

  func window(_ window: NSWindow, didDecodeRestorableState state: NSCoder) {
    if let folder = state.decodeObject(of: NSString.self, forKey: "folder") as String? {
      openedFolder = URL(fileURLWithPath: folder)
      updateFolder()
    }
    let files = state.decodeArrayOfObjects(ofClass: NSString.self, forKey: "files") ?? []
    let locations = state.decodeArrayOfObjects(ofClass: NSNumber.self, forKey: "locations") ?? []
    if files.count > 1 { toggleSplitView(nil) }
    for (index, file) in files.prefix(panes.count).enumerated() where file.length > 0 {
      let location = index < locations.count ? locations[index].intValue : 0
      panes[index].open(URL(fileURLWithPath: file as String), addToHistory: false, restoring: location)
    }
    let active = state.decodeInteger(forKey: "active")
    if panes.indices.contains(active) { setActive(panes[active]) }
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
    command("Toggle Split View", "rectangle.split.2x1", "⌘\\", #selector(toggleSplitView(_:)))
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
    if reader.mode == .markdown { window?.makeFirstResponder(reader.textView) }
    let item = NSMenuItem()
    item.tag = NSTextFinder.Action.showFindInterface.rawValue
    performTextFinderAction(item)
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

import AppKit
import MdowKit
import UniformTypeIdentifiers
import WebKit

/// One window: a sidebar, the selected tab's document on a card, and the companion. Each tab
/// has one document pane, or two side by side in split view, and each pane keeps its own
/// back/forward history, like Finder or Safari.
final class DocumentWindowController: NSWindowController, NSWindowDelegate, NSMenuItemValidation,
  DocumentPaneDelegate, SidebarDelegate, TabStripDelegate
{
  private let split = NSSplitViewController()
  private let sidebar = SidebarViewController()
  private var sidebarItem: NSSplitViewItem!
  private let cardHost = CardViewController()
  private(set) lazy var header = HeaderView(target: self)
  let companion = CompanionViewController()
  private(set) var companionItem: NSSplitViewItem!

  private(set) var tabs: [DocumentTab] = []
  /// The selected tab.
  private(set) var tab: DocumentTab!
  /// The folder tree the sidebar is showing, which may belong to another tab.
  private weak var sidebarTree: FolderNode?
  private var appearanceObservation: NSKeyValueObservation?
  private var clickMonitor: Any?

  var panes: [DocumentPane] { tab.panes }
  /// The pane the sidebar, title and menu commands follow.
  var activePane: DocumentPane { tab.activePane }
  var openedFolder: URL? { tab.openedFolder }
  var folderTree: FolderNode? { tab.folderTree }
  var reader: ReaderViewController { activePane.reader }
  var fileURL: URL? { activePane.fileURL }
  var isEmpty: Bool { tab.isEmpty }
  /// Every document showing in this window, in any tab.
  var openURLs: [URL] { tabs.flatMap(\.openURLs) }

  init() {
    let window = NSWindow(
      contentRect: NSRect(x: 0, y: 0, width: 1120, height: 780),
      styleMask: [.titled, .closable, .miniaturizable, .resizable, .fullSizeContentView],
      backing: .buffered, defer: false)
    window.minSize = NSSize(width: 560, height: 380)
    window.tabbingMode = .disallowed
    window.titlebarAppearsTransparent = true
    window.titleVisibility = .hidden
    window.titlebarSeparatorStyle = .none
    window.backgroundColor = Palette.chrome
    // An empty toolbar makes the title bar as tall as the header, which centres the window
    // buttons in it; the header itself is an ordinary view underneath.
    window.toolbar = NSToolbar(identifier: "MdowHeader")
    window.toolbarStyle = .unifiedCompact
    window.isRestorable = true
    window.restorationClass = WindowRestoration.self
    window.identifier = .init("MdowDocumentWindow")
    super.init(window: window)

    tab = DocumentTab(pane: makePane())
    tabs = [tab]

    sidebar.delegate = self
    sidebarItem = NSSplitViewItem(viewController: sidebar)
    sidebarItem.minimumThickness = 200
    sidebarItem.maximumThickness = 420
    sidebarItem.preferredThicknessFraction = 0.2
    sidebarItem.canCollapse = true
    sidebarItem.holdingPriority = .init(260)

    let splitView = ChromeSplitView()
    splitView.isVertical = true
    split.splitView = splitView
    let contentItem = NSSplitViewItem(viewController: cardHost)
    contentItem.minimumThickness = 320
    split.addSplitViewItem(sidebarItem)
    split.addSplitViewItem(contentItem)
    companionItem = companion.makeSplitItem(context: self)
    split.addSplitViewItem(companionItem)
    split.splitView.autosaveName = "MdowWindowSplit"
    cardHost.show(tab.paneSplit)

    let root = NSViewController()
    root.view = FilledView(fill: Palette.chrome)
    root.addChild(split)
    header.translatesAutoresizingMaskIntoConstraints = false
    split.view.translatesAutoresizingMaskIntoConstraints = false
    root.view.addSubview(split.view)
    root.view.addSubview(header)
    NSLayoutConstraint.activate([
      header.topAnchor.constraint(equalTo: root.view.topAnchor),
      header.leadingAnchor.constraint(equalTo: root.view.leadingAnchor),
      header.trailingAnchor.constraint(equalTo: root.view.trailingAnchor),
      header.heightAnchor.constraint(equalToConstant: Theme.headerHeight),
      split.view.topAnchor.constraint(equalTo: header.bottomAnchor),
      split.view.leadingAnchor.constraint(equalTo: root.view.leadingAnchor),
      split.view.trailingAnchor.constraint(equalTo: root.view.trailingAnchor),
      split.view.bottomAnchor.constraint(equalTo: root.view.bottomAnchor),
    ])
    header.tabs.delegate = self
    window.contentViewController = root
    window.setContentSize(NSSize(width: 1120, height: 780))
    window.delegate = self

    if !window.setFrameUsingName("MdowDocument") { window.center() }
    NotificationCenter.default.addObserver(
      self, selector: #selector(splitDidResize), name: NSSplitView.didResizeSubviewsNotification,
      object: split.splitView)
    updateTitle()

    appearanceObservation = window.observe(\.effectiveAppearance) { [weak self] observed, _ in
      MainActor.assumeIsolated {
        // Read it here: a pane in a background tab is out of the window, so it can't look it up.
        let dark = observed.effectiveAppearance.bestMatch(from: [.aqua, .darkAqua]) == .darkAqua
        self?.tabs.forEach { $0.panes.forEach { $0.appearanceChanged(dark: dark) } }
      }
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
    guard activePane !== pane, tab.panes.contains(where: { $0 === pane }) else { return }
    tab.activePane = pane
    followActivePane()
  }

  /// Points the sidebar, title and header at the active pane, e.g. after switching tabs.
  private func followActivePane() {
    sidebar.setCurrentFile(activePane.fileURL)
    sidebar.setHeadings(reader.mode == .markdown ? activePane.headings : [])
    reader.refreshHeadingTracking()
    updateFolder()
    updateTitle()
  }

  // MARK: Tabs

  /// Adds an empty tab after the selected one and selects it.
  @discardableResult
  func addTab() -> DocumentTab {
    let new = DocumentTab(pane: makePane())
    let index = (tabs.firstIndex { $0 === tab } ?? tabs.count - 1) + 1
    tabs.insert(new, at: index)
    select(new)
    window?.invalidateRestorableState()
    return new
  }

  func select(_ new: DocumentTab) {
    guard new !== tab || cardHost.content !== new.paneSplit else { return }
    tab = new
    cardHost.show(new.paneSplit)
    followActivePane()
    if reader.mode == .markdown { window?.makeFirstResponder(reader.textView) }
    window?.invalidateRestorableState()
  }

  func selectTab(at index: Int) {
    if tabs.indices.contains(index) { select(tabs[index]) }
  }

  /// Selects the tab showing `url`, if there is one.
  func reveal(_ url: URL) -> Bool {
    guard let match = tabs.first(where: { $0.openURLs.contains(url) }) else { return false }
    select(match)
    if let pane = match.panes.first(where: { $0.fileURL == url }) { setActive(pane) }
    return true
  }

  func closeTab(_ closing: DocumentTab) {
    guard tabs.count > 1 else {
      window?.performClose(nil)
      return
    }
    guard let index = tabs.firstIndex(where: { $0 === closing }) else { return }
    closing.close()
    tabs.remove(at: index)
    if closing === tab { select(tabs[min(index, tabs.count - 1)]) } else { updateTitle() }
    window?.invalidateRestorableState()
  }

  @objc func closeTabAction(_ sender: Any?) { closeTab(tab) }

  @objc func selectNextTab(_ sender: Any?) { stepTab(1) }
  @objc func selectPreviousTab(_ sender: Any?) { stepTab(-1) }

  private func stepTab(_ step: Int) {
    guard let index = tabs.firstIndex(where: { $0 === tab }), tabs.count > 1 else { return }
    select(tabs[(index + step + tabs.count) % tabs.count])
  }

  func tabStrip(_ strip: TabStrip, didSelect index: Int) { selectTab(at: index) }

  func tabStrip(_ strip: TabStrip, didClose index: Int) {
    if tabs.indices.contains(index) { closeTab(tabs[index]) }
  }

  func tabStrip(_ strip: TabStrip, didMove from: Int, to: Int) {
    guard tabs.indices.contains(from), tabs.indices.contains(to) else { return }
    tabs.insert(tabs.remove(at: from), at: to)
    updateChrome()
    window?.invalidateRestorableState()
  }

  func tabStripNewTab(_ strip: TabStrip) { addTab() }

  // MARK: Opening

  /// Shows `url` in the active pane. Folders become the sidebar's folder.
  func open(_ url: URL, addToHistory: Bool = true, restoring location: Int? = nil) {
    activePane.open(url, addToHistory: addToHistory, restoring: location)
  }

  func openFolder(at url: URL) {
    tab.openedFolder = url.standardizedFileURL
    window?.invalidateRestorableState()
    updateFolder { [weak self] tree in
      guard let self else { return }
      // An empty tab opens the folder's README.
      if self.fileURL == nil, let readme = tree.children.first(where: {
        !$0.isFolder && $0.url.deletingPathExtension().lastPathComponent.lowercased() == "readme"
      }) {
        self.open(readme.url)
      }
    }
    sidebar.setMode(.files)
    if sidebarItem.isCollapsed { toggleSidebarPanel(nil) }
    updateTitle()
  }

  /// Scans the selected tab's folder for the sidebar, unless it already has.
  private func updateFolder(then completion: ((FolderNode) -> Void)? = nil) {
    let tab = self.tab!
    guard let folder = tab.sidebarFolder else {
      sidebarTree = nil
      sidebar.setFolder(nil)
      return
    }
    if folder == tab.scannedFolder, let tree = tab.folderTree {
      showInSidebar(tree)
      completion?(tree)
      return
    }
    tab.scannedFolder = folder
    Task {
      let tree = await Task.detached(priority: .userInitiated) { FolderNode.scan(folder) }.value
      guard folder == tab.scannedFolder else { return }
      tab.folderTree = tree
      if tab === self.tab { self.showInSidebar(tree) }
      completion?(tree)
    }
  }

  private func showInSidebar(_ tree: FolderNode) {
    guard sidebarTree !== tree else { return }
    sidebarTree = tree
    sidebar.setFolder(tree)
    // The companion offers the folder's documents for @-mentions.
    companion.documentContextChanged()
  }

  private func rescanFolder() {
    tab.scannedFolder = nil
    updateFolder()
  }

  // MARK: Panes

  func paneDidChangeDocument(_ pane: DocumentPane) {
    window?.invalidateRestorableState()
    guard pane === activePane else {
      updateChrome()
      return
    }
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

  // MARK: Title and header

  private func updateTitle() {
    guard let window else { return }
    if let fileURL {
      window.title = FileManager.default.displayName(atPath: fileURL.path)
      window.representedURL = fileURL
    } else if let openedFolder {
      window.title = openedFolder.lastPathComponent
      window.representedURL = openedFolder
    } else {
      window.title = "Mdow"
      window.representedURL = nil
    }
    companion.documentContextChanged()
    updateChrome()
  }

  /// Refreshes the tabs and the header's buttons.
  func updateChrome() {
    let pending = Set(companion.pendingReviewPaths)
    let items = tabs.map { tab in
      var item = tab.stripItem
      item.badge = tab.openURLs.contains { pending.contains($0.path) }
      return item
    }
    header.tabs.update(items: items, selectedIndex: tabs.firstIndex { $0 === tab } ?? 0)
    header.backButton.isEnabled = activePane.canGoBack
    header.forwardButton.isEnabled = activePane.canGoForward
    header.companionButton.isOn = !companionItem.isCollapsed
  }

  /// Keeps the card flush with open side panels and the tabs lined up with the card.
  @objc private func splitDidResize() {
    cardHost.setInsets(
      leading: sidebarItem.isCollapsed ? Theme.gutter : 0,
      trailing: companionItem.isCollapsed ? Theme.gutter : 0)
    cardHost.view.layoutSubtreeIfNeeded()
    let card = cardHost.card.convert(cardHost.card.bounds, to: nil)
    header.contentLeading = sidebarItem.isCollapsed ? 0 : card.minX
    updateChrome()
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
    tab.add(pane)
    if let url = panes[0].fileURL {
      pane.open(url, addToHistory: false, restoring: panes[0].reader.topCharacterIndex)
    }
    setActive(pane)
    window?.invalidateRestorableState()
  }

  private func closePane(_ pane: DocumentPane) {
    guard panes.count > 1 else { return }
    tab.remove(pane)
    followActivePane()
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
    if sidebarItem.isCollapsed { toggleSidebarPanel(nil) }
    sidebar.setMode(mode)
    sidebar.focus()
  }

  /// View › Show Sidebar (⌃⌘S) and the header's sidebar button.
  @objc func toggleSidebarPanel(_ sender: Any?) {
    let collapse = !sidebarItem.isCollapsed
    if collapse, let responder = window?.firstResponder as? NSView, responder.isDescendant(of: sidebar.view) {
      window?.makeFirstResponder(reader.mode == .markdown ? reader.textView : nil)
    }
    NSAnimationContext.runAnimationGroup { context in
      context.duration = NSWorkspace.shared.accessibilityDisplayShouldReduceMotion ? 0 : 0.2
      context.timingFunction = CAMediaTimingFunction(name: .easeOut)
      sidebarItem.animator().isCollapsed = collapse
    }
  }

  /// The header's ⋯ button: document actions that don't need a button of their own.
  @objc func showMoreMenu(_ sender: Any?) {
    let menu = NSMenu()
    func add(_ title: String, _ action: Selector, _ key: String = "", _ modifiers: NSEvent.ModifierFlags = .command, symbol: String) {
      let item = NSMenuItem(title: title, action: action, keyEquivalent: key)
      item.keyEquivalentModifierMask = modifiers
      item.target = self
      item.image = NSImage(systemSymbolName: symbol, accessibilityDescription: nil)
      menu.addItem(item)
    }
    add("Open in Editor", #selector(openWithDefaultEditor(_:)), "e", [.command, .shift], symbol: "square.and.pencil")
    add("Show in Finder", #selector(showInFinder(_:)), "r", [.command, .option], symbol: "folder")
    add("Copy Path", #selector(copyDocumentPath(_:)), "c", [.command, .option], symbol: "doc.on.clipboard")
    add("Share…", #selector(shareDocument(_:)), symbol: "square.and.arrow.up")
    menu.addItem(.separator())
    add("Split View", #selector(toggleSplitView(_:)), "\\", symbol: "rectangle.split.2x1")
    add("Reload", #selector(reloadDocument(_:)), "r", symbol: "arrow.clockwise")
    menu.addItem(.separator())
    add("Export as PDF…", #selector(exportAsPDF(_:)), symbol: "doc.richtext")
    add("Print…", #selector(printDocument(_:)), "p", symbol: "printer")
    let button = header.moreButton
    menu.popUp(positioning: nil, at: NSPoint(x: 0, y: button.bounds.height + 4), in: button)
  }

  @objc func shareDocument(_ sender: Any?) {
    guard let fileURL else { return }
    let button = header.moreButton
    NSSharingServicePicker(items: [fileURL]).show(relativeTo: button.bounds, of: button, preferredEdge: .minY)
  }

  @objc func quickOpen(_ sender: Any?) {
    guard let window else { return }
    CommandPalette.present(for: self, in: window)
  }

  /// Find goes to the document's find bar even when the sidebar has focus.
  @objc override func performTextFinderAction(_ sender: Any?) {
    let tag = (sender as? NSValidatedUserInterfaceItem)?.tag ?? NSTextFinder.Action.showFindInterface.rawValue
    reader.performFind(NSTextFinder.Action(rawValue: tag) ?? .showFindInterface)
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
    case #selector(toggleSidebarPanel(_:)):
      item.title = sidebarItem.isCollapsed ? "Show Sidebar" : "Hide Sidebar"
      return true
    case #selector(shareDocument(_:)):
      return fileURL != nil
    case #selector(selectNextTab(_:)), #selector(selectPreviousTab(_:)):
      return tabs.count > 1
    default: return true
    }
  }

  // MARK: Window

  func windowWillClose(_ notification: Notification) {
    tabs.forEach { $0.close() }
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

  func windowWillEnterFullScreen(_ notification: Notification) { header.isFullScreen = true }
  func windowDidExitFullScreen(_ notification: Notification) { header.isFullScreen = false }

  func windowDidBecomeKey(_ notification: Notification) {
    if reader.mode == .markdown, window?.firstResponder === window {
      window?.makeFirstResponder(reader.textView)
    }
  }

  // MARK: State restoration

  private struct TabState: Codable {
    var folder: String?
    var files: [String]
    var locations: [Int]
    var active: Int
  }

  func window(_ window: NSWindow, willEncodeRestorableState state: NSCoder) {
    let states = tabs.map { tab in
      TabState(
        folder: tab.openedFolder?.path, files: tab.panes.map { $0.fileURL?.path ?? "" },
        locations: tab.panes.map { $0.reader.topCharacterIndex },
        active: tab.panes.firstIndex { $0 === tab.activePane } ?? 0)
    }
    if let data = try? JSONEncoder().encode(states) { state.encode(data as NSData, forKey: "tabs") }
    state.encode(tabs.firstIndex { $0 === tab } ?? 0, forKey: "selectedTab")
  }

  func window(_ window: NSWindow, didDecodeRestorableState state: NSCoder) {
    guard let data = state.decodeObject(of: NSData.self, forKey: "tabs") as Data?,
      let states = try? JSONDecoder().decode([TabState].self, from: data), !states.isEmpty
    else { return }
    for (index, saved) in states.enumerated() {
      if index > 0 { addTab() }
      if let folder = saved.folder { tab.openedFolder = URL(fileURLWithPath: folder) }
      if saved.files.count > 1 { toggleSplitView(nil) }
      for (paneIndex, file) in saved.files.prefix(panes.count).enumerated() where !file.isEmpty {
        let location = paneIndex < saved.locations.count ? saved.locations[paneIndex] : 0
        panes[paneIndex].open(URL(fileURLWithPath: file), addToHistory: false, restoring: location)
      }
      if panes.indices.contains(saved.active) { setActive(panes[saved.active]) }
      updateFolder()
    }
    selectTab(at: state.decodeInteger(forKey: "selectedTab"))
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
    let root = openedFolder ?? tab.scannedFolder
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
    command("Close Tab", "xmark.square", "⌘W", #selector(closeTabAction(_:)))
    command("Find in Document", "text.magnifyingglass", "⌘F", #selector(findAction))
    command("Toggle Sidebar", "sidebar.left", "⌃⌘S", #selector(toggleSidebarPanel(_:)))
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
    reader.performFind(.showFindInterface)
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

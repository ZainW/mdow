import AppKit
import MdowKit

@MainActor protocol SidebarDelegate: AnyObject {
  func sidebar(_ sidebar: SidebarViewController, didSelectHeading index: Int)
  func sidebar(_ sidebar: SidebarViewController, open url: URL, inNewTab: Bool)
  func sidebarModeChanged(_ sidebar: SidebarViewController)
}

/// One heading and the headings nested under it.
private final class HeadingNode {
  let index: Int
  let heading: OutlineHeading
  var children: [HeadingNode] = []

  init(index: Int, heading: OutlineHeading) {
    self.index = index
    self.heading = heading
  }

  static func tree(_ headings: [OutlineHeading]) -> [HeadingNode] {
    var roots: [HeadingNode] = []
    var stack: [HeadingNode] = []
    for (index, heading) in headings.enumerated() {
      let node = HeadingNode(index: index, heading: heading)
      while let last = stack.last, last.heading.level >= heading.level { stack.removeLast() }
      if let parent = stack.last { parent.children.append(node) } else { roots.append(node) }
      stack.append(node)
    }
    return roots
  }
}

private final class RecentItem {
  let url: URL
  init(_ url: URL) { self.url = url }
}

/// The source list on the left: the document's outline, the folder's files, or recents.
final class SidebarViewController: NSViewController, NSOutlineViewDataSource, NSOutlineViewDelegate,
  NSMenuDelegate, NSSearchFieldDelegate
{
  weak var delegate: SidebarDelegate?

  private(set) var mode: SidebarMode = Preferences.shared.sidebarMode
  private let modeControl = NSSegmentedControl()
  private let outlineView = NSOutlineView()
  private let scrollView = NSScrollView()
  private let filterField = NSSearchField()
  private let emptyLabel = NSTextField(wrappingLabelWithString: "")
  private let emptyButton = NSButton()
  private lazy var emptyStack = NSStackView(views: [emptyLabel, emptyButton])
  private var filterHeight: NSLayoutConstraint?

  private var headingRoots: [HeadingNode] = []
  private var headingNodes: [HeadingNode] = []
  private var folder: FolderNode?
  private var shownFolder: FolderNode?
  private var currentFile: URL?
  private var recents: [RecentItem] = []
  private var programmaticSelection = false

  override func loadView() {
    let root = NSView()
    view = root

    modeControl.segmentCount = SidebarMode.allCases.count
    modeControl.trackingMode = .selectOne
    modeControl.segmentDistribution = .fillEqually
    for (index, mode) in SidebarMode.allCases.enumerated() {
      modeControl.setImage(
        NSImage(systemSymbolName: mode.symbol, accessibilityDescription: mode.displayName), forSegment: index)
      modeControl.setToolTip(mode.displayName, forSegment: index)
      modeControl.setWidth(0, forSegment: index)
    }
    modeControl.setAccessibilityLabel("Sidebar content")
    modeControl.target = self
    modeControl.action = #selector(modeControlChanged)
    modeControl.translatesAutoresizingMaskIntoConstraints = false

    let column = NSTableColumn(identifier: .init("main"))
    column.resizingMask = .autoresizingMask
    outlineView.addTableColumn(column)
    outlineView.outlineTableColumn = column
    outlineView.headerView = nil
    outlineView.style = .sourceList
    outlineView.rowSizeStyle = .default
    outlineView.floatsGroupRows = false
    outlineView.autoresizesOutlineColumn = false
    outlineView.indentationPerLevel = 12
    outlineView.dataSource = self
    outlineView.delegate = self
    outlineView.target = self
    outlineView.action = #selector(rowClicked)
    let menu = NSMenu()
    menu.delegate = self
    outlineView.menu = menu

    scrollView.documentView = outlineView
    scrollView.hasVerticalScroller = true
    scrollView.autohidesScrollers = true
    scrollView.drawsBackground = false
    scrollView.translatesAutoresizingMaskIntoConstraints = false

    filterField.placeholderString = "Filter"
    filterField.controlSize = .regular
    filterField.delegate = self
    filterField.sendsSearchStringImmediately = true
    filterField.translatesAutoresizingMaskIntoConstraints = false

    emptyLabel.alignment = .center
    emptyLabel.textColor = .secondaryLabelColor
    emptyLabel.font = .systemFont(ofSize: NSFont.systemFontSize)
    emptyButton.bezelStyle = .push
    emptyButton.target = self
    emptyButton.action = #selector(emptyButtonClicked)
    emptyStack.orientation = .vertical
    emptyStack.spacing = 10
    emptyStack.translatesAutoresizingMaskIntoConstraints = false

    root.addSubview(scrollView)
    root.addSubview(modeControl)
    root.addSubview(filterField)
    root.addSubview(emptyStack)
    let filterHeight = filterField.heightAnchor.constraint(equalToConstant: 0)
    self.filterHeight = filterHeight
    NSLayoutConstraint.activate([
      modeControl.topAnchor.constraint(equalTo: root.safeAreaLayoutGuide.topAnchor, constant: 4),
      modeControl.leadingAnchor.constraint(equalTo: root.leadingAnchor, constant: 12),
      modeControl.trailingAnchor.constraint(equalTo: root.trailingAnchor, constant: -12),
      scrollView.topAnchor.constraint(equalTo: modeControl.bottomAnchor, constant: 8),
      scrollView.leadingAnchor.constraint(equalTo: root.leadingAnchor),
      scrollView.trailingAnchor.constraint(equalTo: root.trailingAnchor),
      scrollView.bottomAnchor.constraint(equalTo: filterField.topAnchor, constant: -8),
      filterField.leadingAnchor.constraint(equalTo: root.leadingAnchor, constant: 10),
      filterField.trailingAnchor.constraint(equalTo: root.trailingAnchor, constant: -10),
      filterField.bottomAnchor.constraint(equalTo: root.bottomAnchor, constant: -10),
      emptyStack.centerXAnchor.constraint(equalTo: scrollView.centerXAnchor),
      emptyStack.centerYAnchor.constraint(equalTo: scrollView.centerYAnchor, constant: -20),
      emptyStack.widthAnchor.constraint(lessThanOrEqualTo: root.widthAnchor, constant: -32),
    ])
    scrollView.automaticallyAdjustsContentInsets = false
    scrollView.contentInsets = NSEdgeInsets(top: 0, left: 0, bottom: 0, right: 0)

    NotificationCenter.default.addObserver(
      self, selector: #selector(recentsChanged), name: .recentsChanged, object: nil)
    reloadRecentsList()
    applyMode()
  }

  // MARK: Mode

  func setMode(_ mode: SidebarMode) {
    guard mode != self.mode || modeControl.selectedSegment < 0 else { return }
    self.mode = mode
    Preferences.shared.sidebarMode = mode
    applyMode()
    delegate?.sidebarModeChanged(self)
  }

  @objc private func modeControlChanged() {
    setMode(SidebarMode.allCases[modeControl.selectedSegment])
  }

  private func applyMode() {
    modeControl.selectedSegment = SidebarMode.allCases.firstIndex(of: mode) ?? 0
    let showsFilter = mode == .files && folder != nil
    filterField.isHidden = !showsFilter
    filterHeight?.isActive = !showsFilter
    reload()
  }

  private func reload() {
    programmaticSelection = true
    outlineView.reloadData()
    switch mode {
    case .outline:
      outlineView.expandItem(nil, expandChildren: true)
    case .files:
      if !filterField.stringValue.isEmpty { outlineView.expandItem(nil, expandChildren: true) }
      revealCurrentFile()
    case .recents:
      if let currentFile, let index = recents.firstIndex(where: { $0.url == currentFile }) {
        outlineView.selectRowIndexes([outlineView.row(forItem: recents[index])], byExtendingSelection: false)
      } else {
        outlineView.deselectAll(nil)
      }
    }
    programmaticSelection = false
    updateEmptyState()
  }

  private func updateEmptyState() {
    var message: String?
    var button: String?
    switch mode {
    case .outline:
      if headingRoots.isEmpty { message = currentFile == nil ? "No Document" : "No Headings" }
    case .files:
      if folder == nil {
        message = "No Folder Open"
        button = "Open Folder…"
      } else if shownFolder == nil || shownFolder?.children.isEmpty == true {
        message = filterField.stringValue.isEmpty ? "No Markdown Files" : "No Matches"
      }
    case .recents:
      if recents.isEmpty { message = "No Recent Documents" }
    }
    emptyStack.isHidden = message == nil
    emptyLabel.stringValue = message ?? ""
    emptyButton.isHidden = button == nil
    emptyButton.title = button ?? ""
  }

  @objc private func emptyButtonClicked() {
    NSApp.sendAction(#selector(DocumentWindowController.openFolder(_:)), to: nil, from: self)
  }

  // MARK: Content

  func setHeadings(_ headings: [OutlineHeading]) {
    headingRoots = HeadingNode.tree(headings)
    var flat: [HeadingNode] = []
    func walk(_ nodes: [HeadingNode]) {
      for node in nodes {
        flat.append(node)
        walk(node.children)
      }
    }
    walk(headingRoots)
    headingNodes = flat.sorted { $0.index < $1.index }
    if mode == .outline { reload() }
  }

  func setCurrentHeading(_ index: Int?) {
    guard mode == .outline else { return }
    programmaticSelection = true
    defer { programmaticSelection = false }
    guard let index, headingNodes.indices.contains(index) else {
      outlineView.deselectAll(nil)
      return
    }
    let node = headingNodes[index]
    var row = outlineView.row(forItem: node)
    if row < 0 {
      // Inside a collapsed parent: highlight the nearest visible ancestor.
      var ancestor: HeadingNode? = node
      while let current = ancestor, outlineView.row(forItem: current) < 0 {
        ancestor = headingNodes.last { $0.children.contains { $0 === current } }
      }
      row = ancestor.map { outlineView.row(forItem: $0) } ?? -1
    }
    guard row >= 0 else { return }
    outlineView.selectRowIndexes([row], byExtendingSelection: false)
    outlineView.scrollRowToVisible(row)
  }

  func setFolder(_ folder: FolderNode?) {
    self.folder = folder
    applyFilter()
    if mode == .files { applyMode() }
  }

  func setCurrentFile(_ url: URL?) {
    currentFile = url?.standardizedFileURL
    if mode != .outline { reload() }
  }

  private func applyFilter() {
    shownFolder = folder?.filtered(filterField.stringValue)
  }

  func controlTextDidChange(_ notification: Notification) {
    applyFilter()
    reload()
  }

  private func revealCurrentFile() {
    guard let currentFile, let root = shownFolder else {
      outlineView.deselectAll(nil)
      return
    }
    func path(to node: FolderNode) -> [FolderNode]? {
      if !node.isFolder { return node.url.standardizedFileURL == currentFile ? [node] : nil }
      for child in node.children {
        if let found = path(to: child) { return [node] + found }
      }
      return nil
    }
    guard let chain = path(to: root) else {
      outlineView.deselectAll(nil)
      return
    }
    for folder in chain.dropFirst().dropLast() { outlineView.expandItem(folder) }
    let row = outlineView.row(forItem: chain.last)
    if row >= 0 {
      outlineView.selectRowIndexes([row], byExtendingSelection: false)
      outlineView.scrollRowToVisible(row)
    }
  }

  @objc private func recentsChanged() {
    reloadRecentsList()
    if mode == .recents { reload() }
  }

  private func reloadRecentsList() {
    recents = Preferences.shared.recents.map(RecentItem.init)
  }

  // MARK: Data source

  func outlineView(_ outlineView: NSOutlineView, numberOfChildrenOfItem item: Any?) -> Int {
    switch mode {
    case .outline: (item as? HeadingNode)?.children.count ?? headingRoots.count
    case .files: ((item as? FolderNode) ?? shownFolder)?.children.count ?? 0
    case .recents: item == nil ? recents.count : 0
    }
  }

  func outlineView(_ outlineView: NSOutlineView, child index: Int, ofItem item: Any?) -> Any {
    switch mode {
    case .outline: (item as? HeadingNode)?.children[index] ?? headingRoots[index]
    case .files: ((item as? FolderNode) ?? shownFolder!).children[index]
    case .recents: recents[index]
    }
  }

  func outlineView(_ outlineView: NSOutlineView, isItemExpandable item: Any) -> Bool {
    if let node = item as? HeadingNode { return !node.children.isEmpty }
    if let node = item as? FolderNode { return node.isFolder }
    return false
  }

  func outlineView(_ outlineView: NSOutlineView, viewFor tableColumn: NSTableColumn?, item: Any) -> NSView? {
    // Headings have no icons, so their text starts where the icons would.
    let isHeading = item is HeadingNode
    let identifier = NSUserInterfaceItemIdentifier(isHeading ? "heading" : "cell")
    let cell =
      outlineView.makeView(withIdentifier: identifier, owner: self) as? NSTableCellView
      ?? makeCell(identifier, icon: !isHeading)
    let text = cell.textField!
    text.font = .systemFont(ofSize: NSFont.systemFontSize)
    text.textColor = .labelColor
    cell.toolTip = nil
    if let node = item as? HeadingNode {
      text.stringValue = node.heading.title
      if node.heading.level == 1 { text.font = .systemFont(ofSize: NSFont.systemFontSize, weight: .semibold) }
      return cell
    }
    let image = cell.imageView!
    image.contentTintColor = nil
    switch item {
    case let node as FolderNode:
      text.stringValue = node.name
      image.isHidden = false
      if node.isFolder {
        image.image = NSImage(systemSymbolName: "folder", accessibilityDescription: "Folder")
        image.contentTintColor = .controlAccentColor
      } else {
        let symbol = FileKind(url: node.url) == .html ? "globe" : "doc.text"
        image.image = NSImage(systemSymbolName: symbol, accessibilityDescription: nil)
        image.contentTintColor = .secondaryLabelColor
      }
      cell.toolTip = node.url.abbreviatedPath
    case let recent as RecentItem:
      text.stringValue = recent.url.lastPathComponent
      image.isHidden = false
      image.image = NSImage(systemSymbolName: "doc.text", accessibilityDescription: nil)
      image.contentTintColor = .secondaryLabelColor
      cell.toolTip = recent.url.abbreviatedPath
      if !FileManager.default.fileExists(atPath: recent.url.path) { text.textColor = .tertiaryLabelColor }
    default:
      break
    }
    return cell
  }

  private func makeCell(_ identifier: NSUserInterfaceItemIdentifier, icon: Bool) -> NSTableCellView {
    let cell = NSTableCellView()
    cell.identifier = identifier
    guard icon else {
      let text = NSTextField(labelWithString: "")
      text.translatesAutoresizingMaskIntoConstraints = false
      text.lineBreakMode = .byTruncatingTail
      cell.addSubview(text)
      cell.textField = text
      NSLayoutConstraint.activate([
        text.leadingAnchor.constraint(equalTo: cell.leadingAnchor, constant: 2),
        text.trailingAnchor.constraint(lessThanOrEqualTo: cell.trailingAnchor, constant: -2),
        text.centerYAnchor.constraint(equalTo: cell.centerYAnchor),
      ])
      return cell
    }
    let image = NSImageView()
    image.translatesAutoresizingMaskIntoConstraints = false
    image.symbolConfiguration = .init(scale: .medium)
    let text = NSTextField(labelWithString: "")
    text.translatesAutoresizingMaskIntoConstraints = false
    text.lineBreakMode = .byTruncatingTail
    text.cell?.truncatesLastVisibleLine = true
    cell.addSubview(image)
    cell.addSubview(text)
    cell.imageView = image
    cell.textField = text
    let leading = text.leadingAnchor.constraint(equalTo: image.trailingAnchor, constant: 6)
    leading.priority = .defaultHigh
    NSLayoutConstraint.activate([
      image.leadingAnchor.constraint(equalTo: cell.leadingAnchor, constant: 2),
      image.centerYAnchor.constraint(equalTo: cell.centerYAnchor),
      image.widthAnchor.constraint(equalToConstant: 18),
      leading,
      text.trailingAnchor.constraint(lessThanOrEqualTo: cell.trailingAnchor, constant: -2),
      text.centerYAnchor.constraint(equalTo: cell.centerYAnchor),
    ])
    return cell
  }

  func outlineViewItemDidCollapse(_ notification: Notification) {}

  // MARK: Selection

  @objc private func rowClicked() {
    activate(row: outlineView.clickedRow, newTab: NSApp.currentEvent?.modifierFlags.contains(.command) == true)
  }

  func outlineViewSelectionDidChange(_ notification: Notification) {
    guard !programmaticSelection else { return }
    // Mouse clicks are handled by `rowClicked`; this follows the keyboard.
    if let event = NSApp.currentEvent, event.type == .keyDown {
      activate(row: outlineView.selectedRow, newTab: false)
    }
  }

  private func activate(row: Int, newTab: Bool) {
    guard row >= 0, let item = outlineView.item(atRow: row) else { return }
    switch item {
    case let node as HeadingNode:
      delegate?.sidebar(self, didSelectHeading: node.index)
    case let node as FolderNode:
      if node.isFolder {
        if NSApp.currentEvent?.type != .keyDown {
          outlineView.isItemExpanded(node) ? outlineView.collapseItem(node) : outlineView.expandItem(node)
        }
      } else {
        delegate?.sidebar(self, open: node.url, inNewTab: newTab)
      }
    case let recent as RecentItem:
      delegate?.sidebar(self, open: recent.url, inNewTab: newTab)
    default:
      break
    }
  }

  // MARK: Context menu

  private var menuURL: URL?

  func menuNeedsUpdate(_ menu: NSMenu) {
    menu.removeAllItems()
    let row = outlineView.clickedRow
    guard row >= 0, let item = outlineView.item(atRow: row) else { return }
    let url: URL
    switch item {
    case let node as FolderNode: url = node.url
    case let recent as RecentItem: url = recent.url
    case let node as HeadingNode:
      menu.addItem(withTitle: "Copy Link to Heading", action: #selector(copyHeadingLink(_:)), keyEquivalent: "")
        .representedObject = node.heading.anchor
      menu.items.forEach { $0.target = self }
      return
    default: return
    }
    menuURL = url
    let isFolder = (item as? FolderNode)?.isFolder == true
    if !isFolder {
      menu.addItem(withTitle: "Open", action: #selector(openFromMenu), keyEquivalent: "")
      menu.addItem(withTitle: "Open in New Tab", action: #selector(openInNewTabFromMenu), keyEquivalent: "")
      menu.addItem(.separator())
    }
    menu.addItem(withTitle: "Show in Finder", action: #selector(showInFinder), keyEquivalent: "")
    menu.addItem(withTitle: "Copy Path", action: #selector(copyPath), keyEquivalent: "")
    if item is RecentItem {
      menu.addItem(.separator())
      menu.addItem(withTitle: "Remove from Recents", action: #selector(removeRecent), keyEquivalent: "")
      menu.addItem(withTitle: "Clear Recents", action: #selector(clearRecents), keyEquivalent: "")
    }
    menu.items.forEach { $0.target = self }
  }

  @objc private func openFromMenu() {
    if let menuURL { delegate?.sidebar(self, open: menuURL, inNewTab: false) }
  }

  @objc private func openInNewTabFromMenu() {
    if let menuURL { delegate?.sidebar(self, open: menuURL, inNewTab: true) }
  }

  @objc private func showInFinder() {
    if let menuURL { NSWorkspace.shared.activateFileViewerSelecting([menuURL]) }
  }

  @objc private func copyPath() {
    guard let menuURL else { return }
    NSPasteboard.general.clearContents()
    NSPasteboard.general.setString(menuURL.path, forType: .string)
  }

  @objc private func copyHeadingLink(_ sender: NSMenuItem) {
    guard let anchor = sender.representedObject as? String else { return }
    NSPasteboard.general.clearContents()
    NSPasteboard.general.setString("#" + anchor, forType: .string)
  }

  @objc private func removeRecent() {
    if let menuURL { Preferences.shared.removeRecent(menuURL) }
  }

  @objc private func clearRecents() {
    Preferences.shared.clearRecents()
  }

  /// Focus the list, e.g. after switching modes from the keyboard.
  func focus() {
    view.window?.makeFirstResponder(outlineView)
  }
}

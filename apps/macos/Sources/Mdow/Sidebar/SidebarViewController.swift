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

/// The sidebar: the document's outline, the folder's files, or recents, as a flat list on the
/// window's own background.
final class SidebarViewController: NSViewController, NSOutlineViewDataSource, NSOutlineViewDelegate,
  NSMenuDelegate, NSTextFieldDelegate
{
  weak var delegate: SidebarDelegate?

  private(set) var mode: SidebarMode = Preferences.shared.sidebarMode
  private var modeButtons: [ChromeButton] = []
  private let outlineView = SidebarOutlineView()
  private let scrollView = NSScrollView()
  private let titleLabel = NSTextField(labelWithString: "")
  private let filterBox = FilledView(fill: Palette.hover)
  private let filterField = NSTextField()
  private let emptyLabel = NSTextField(wrappingLabelWithString: "")
  private lazy var emptyButton = ChromeButton(
    symbol: "folder", title: "Open Folder…", style: .field, target: self, action: #selector(emptyButtonClicked))
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
    let root = FilledView(fill: Palette.chrome)
    view = root

    let modeBar = NSStackView()
    modeBar.orientation = .horizontal
    modeBar.spacing = 2
    modeBar.translatesAutoresizingMaskIntoConstraints = false
    for mode in SidebarMode.allCases {
      let button = ChromeButton(
        symbol: nil, title: mode.displayName, style: .text, target: self, action: #selector(modeButtonClicked(_:)))
      button.titleFont = Theme.ui(12.5, .medium)
      button.toolTip = "\(mode.displayName)  ⌃⌘\(SidebarMode.allCases.firstIndex(of: mode)! + 1)"
      button.tag = SidebarMode.allCases.firstIndex(of: mode)!
      modeButtons.append(button)
      modeBar.addArrangedSubview(button)
    }
    modeBar.setAccessibilityRole(.radioGroup)
    modeBar.setAccessibilityLabel("Sidebar content")

    titleLabel.font = Theme.ui(11.5, .medium)
    titleLabel.textColor = Palette.tertiaryText
    titleLabel.lineBreakMode = .byTruncatingMiddle
    titleLabel.translatesAutoresizingMaskIntoConstraints = false

    let column = NSTableColumn(identifier: .init("main"))
    column.resizingMask = .autoresizingMask
    outlineView.addTableColumn(column)
    outlineView.outlineTableColumn = column
    outlineView.headerView = nil
    outlineView.style = .plain
    outlineView.backgroundColor = .clear
    outlineView.rowHeight = 28
    outlineView.intercellSpacing = NSSize(width: 0, height: 1)
    outlineView.floatsGroupRows = false
    outlineView.autoresizesOutlineColumn = false
    outlineView.indentationPerLevel = 14
    outlineView.focusRingType = .none
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
    scrollView.scrollerStyle = .overlay
    scrollView.drawsBackground = false
    scrollView.translatesAutoresizingMaskIntoConstraints = false

    filterBox.wantsLayer = true
    filterBox.layer?.cornerRadius = 7
    filterBox.translatesAutoresizingMaskIntoConstraints = false
    let filterIcon = NSImageView(
      image: NSImage(systemSymbolName: "line.3.horizontal.decrease", accessibilityDescription: nil)!)
    filterIcon.symbolConfiguration = .init(pointSize: 11, weight: .medium)
    filterIcon.contentTintColor = Palette.tertiaryText
    filterIcon.translatesAutoresizingMaskIntoConstraints = false
    filterField.placeholderAttributedString = NSAttributedString(
      string: "Filter files",
      attributes: [.font: Theme.ui(12.5), .foregroundColor: Palette.tertiaryText])
    filterField.font = Theme.ui(12.5)
    filterField.textColor = Palette.text
    filterField.isBordered = false
    filterField.drawsBackground = false
    filterField.focusRingType = .none
    filterField.delegate = self
    filterField.cell?.sendsActionOnEndEditing = false
    filterField.setAccessibilityLabel("Filter files")
    filterField.translatesAutoresizingMaskIntoConstraints = false
    filterBox.addSubview(filterIcon)
    filterBox.addSubview(filterField)

    emptyLabel.alignment = .center
    emptyLabel.textColor = Palette.secondaryText
    emptyLabel.font = Theme.ui(12.5)
    emptyStack.orientation = .vertical
    emptyStack.spacing = 12
    emptyStack.translatesAutoresizingMaskIntoConstraints = false

    let settings = ChromeButton(
      symbol: "gearshape", title: "Settings", style: .text, hint: "⌘,", target: nil,
      action: #selector(AppDelegate.showSettings(_:)))
    settings.titleFont = Theme.ui(12.5, .medium)
    settings.symbolSize = 13
    let appearance = ChromeButton(
      symbol: "circle.lefthalf.filled", title: "Switch Appearance", target: self,
      action: #selector(cycleAppearance))
    appearance.symbolSize = 13

    for view: NSView in [modeBar, titleLabel, filterBox, scrollView, emptyStack, settings, appearance] {
      root.addSubview(view)
    }
    let filterHeight = filterBox.heightAnchor.constraint(equalToConstant: 0)
    self.filterHeight = filterHeight
    NSLayoutConstraint.activate([
      modeBar.topAnchor.constraint(equalTo: root.topAnchor, constant: 2),
      modeBar.leadingAnchor.constraint(equalTo: root.leadingAnchor, constant: 8),
      modeBar.trailingAnchor.constraint(lessThanOrEqualTo: root.trailingAnchor, constant: -8),
      filterBox.topAnchor.constraint(equalTo: modeBar.bottomAnchor, constant: 10),
      filterBox.leadingAnchor.constraint(equalTo: root.leadingAnchor, constant: 10),
      filterBox.trailingAnchor.constraint(equalTo: root.trailingAnchor, constant: -10),
      filterIcon.leadingAnchor.constraint(equalTo: filterBox.leadingAnchor, constant: 9),
      filterIcon.centerYAnchor.constraint(equalTo: filterBox.centerYAnchor),
      filterField.leadingAnchor.constraint(equalTo: filterIcon.trailingAnchor, constant: 7),
      filterField.trailingAnchor.constraint(equalTo: filterBox.trailingAnchor, constant: -8),
      filterField.centerYAnchor.constraint(equalTo: filterBox.centerYAnchor),
      titleLabel.topAnchor.constraint(equalTo: filterBox.bottomAnchor, constant: 10),
      titleLabel.leadingAnchor.constraint(equalTo: root.leadingAnchor, constant: 18),
      titleLabel.trailingAnchor.constraint(lessThanOrEqualTo: root.trailingAnchor, constant: -16),
      scrollView.topAnchor.constraint(equalTo: titleLabel.bottomAnchor, constant: 4),
      scrollView.leadingAnchor.constraint(equalTo: root.leadingAnchor),
      scrollView.trailingAnchor.constraint(equalTo: root.trailingAnchor),
      scrollView.bottomAnchor.constraint(equalTo: settings.topAnchor, constant: -6),
      settings.leadingAnchor.constraint(equalTo: root.leadingAnchor, constant: 8),
      settings.bottomAnchor.constraint(equalTo: root.bottomAnchor, constant: -8),
      appearance.trailingAnchor.constraint(equalTo: root.trailingAnchor, constant: -8),
      appearance.centerYAnchor.constraint(equalTo: settings.centerYAnchor),
      emptyStack.centerXAnchor.constraint(equalTo: scrollView.centerXAnchor),
      emptyStack.centerYAnchor.constraint(equalTo: scrollView.centerYAnchor, constant: -30),
      emptyStack.widthAnchor.constraint(lessThanOrEqualTo: root.widthAnchor, constant: -40),
    ])
    filterHeight.isActive = true

    NotificationCenter.default.addObserver(
      self, selector: #selector(recentsChanged), name: .recentsChanged, object: nil)
    reloadRecentsList()
    applyMode()
  }

  @objc private func cycleAppearance() {
    let dark = view.effectiveAppearance.bestMatch(from: [.aqua, .darkAqua]) == .darkAqua
    Preferences.shared.appearance = dark ? .light : .dark
  }

  // MARK: Mode

  func setMode(_ mode: SidebarMode) {
    guard mode != self.mode else { return }
    self.mode = mode
    Preferences.shared.sidebarMode = mode
    applyMode()
    delegate?.sidebarModeChanged(self)
  }

  @objc private func modeButtonClicked(_ sender: ChromeButton) {
    setMode(SidebarMode.allCases[sender.tag])
  }

  private func applyMode() {
    for (index, button) in modeButtons.enumerated() {
      button.isOn = SidebarMode.allCases[index] == mode
      button.setAccessibilityValue(button.isOn)
    }
    let showsFilter = mode == .files && folder != nil
    filterBox.isHidden = !showsFilter
    filterHeight?.constant = showsFilter ? 28 : 0
    updateTitle()
    reload()
  }

  private func updateTitle() {
    switch mode {
    case .outline: titleLabel.stringValue = headingRoots.isEmpty ? "" : "On this page"
    case .files: titleLabel.stringValue = folder?.url.lastPathComponent ?? ""
    case .recents: titleLabel.stringValue = recents.isEmpty ? "" : "Recently opened"
    }
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
      if headingRoots.isEmpty {
        message = currentFile == nil ? "Open a document to see its outline." : "This document has no headings."
      }
    case .files:
      if folder == nil {
        message = "Open a folder to browse its documents."
        button = "Open Folder…"
      } else if shownFolder == nil || shownFolder?.children.isEmpty == true {
        message = filterField.stringValue.isEmpty ? "No Markdown files here." : "Nothing matches."
      }
    case .recents:
      if recents.isEmpty { message = "Documents you open show up here." }
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
    if mode == .outline {
      updateTitle()
      reload()
    }
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
    _ = view
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

  func control(_ control: NSControl, textView: NSTextView, doCommandBy selector: Selector) -> Bool {
    // Escape clears the filter; arrows move into the list.
    if selector == #selector(cancelOperation(_:)), !filterField.stringValue.isEmpty {
      filterField.stringValue = ""
      controlTextDidChange(Notification(name: NSControl.textDidChangeNotification))
      return true
    }
    if selector == #selector(moveDown(_:)) {
      view.window?.makeFirstResponder(outlineView)
      if outlineView.selectedRow < 0, outlineView.numberOfRows > 0 {
        outlineView.selectRowIndexes([0], byExtendingSelection: false)
      }
      return true
    }
    return false
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
    if mode == .recents {
      updateTitle()
      reload()
    }
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

  func outlineView(_ outlineView: NSOutlineView, rowViewForItem item: Any) -> NSTableRowView? {
    SidebarRowView()
  }

  func outlineView(_ outlineView: NSOutlineView, viewFor tableColumn: NSTableColumn?, item: Any) -> NSView? {
    // Headings have no icons, so their text starts where the icons would.
    let isHeading = item is HeadingNode
    let identifier = NSUserInterfaceItemIdentifier(isHeading ? "heading" : "cell")
    let cell =
      outlineView.makeView(withIdentifier: identifier, owner: self) as? SidebarCell
      ?? SidebarCell(identifier: identifier, icon: !isHeading)
    cell.toolTip = nil
    cell.dimmed = false
    if let node = item as? HeadingNode {
      cell.textField?.stringValue = node.heading.title
      cell.prominent = node.heading.level == 1
      return cell
    }
    cell.prominent = false
    switch item {
    case let node as FolderNode:
      cell.textField?.stringValue = node.name
      if node.isFolder {
        cell.symbol = "folder"
      } else {
        cell.symbol = FileKind(url: node.url) == .html ? "globe" : "doc.text"
      }
      cell.toolTip = node.url.abbreviatedPath
    case let recent as RecentItem:
      cell.textField?.stringValue = recent.url.lastPathComponent
      cell.symbol = FileKind(url: recent.url) == .html ? "globe" : "doc.text"
      cell.toolTip = recent.url.abbreviatedPath
      cell.dimmed = !FileManager.default.fileExists(atPath: recent.url.path)
    default:
      break
    }
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

/// The sidebar list. Rows are drawn by `SidebarRowView`, so the system's own selection is off.
private final class SidebarOutlineView: NSOutlineView {
  override func frameOfOutlineCell(atRow row: Int) -> NSRect {
    // Disclosure chevrons sit a little in from the edge, inside the row's rounded highlight.
    super.frameOfOutlineCell(atRow: row).offsetBy(dx: 8, dy: 0)
  }

  override func frameOfCell(atColumn column: Int, row: Int) -> NSRect {
    var frame = super.frameOfCell(atColumn: column, row: row)
    frame.origin.x += 8
    frame.size.width -= 16
    return frame
  }
}

/// A sidebar row: a rounded pill when selected, a fainter one under the pointer.
private final class SidebarRowView: NSTableRowView {
  private var hovering = false

  override func updateTrackingAreas() {
    super.updateTrackingAreas()
    trackingAreas.forEach(removeTrackingArea)
    addTrackingArea(
      NSTrackingArea(rect: .zero, options: [.mouseEnteredAndExited, .activeInKeyWindow, .inVisibleRect], owner: self))
  }

  override func mouseEntered(with event: NSEvent) {
    hovering = true
    needsDisplay = true
  }

  override func mouseExited(with event: NSEvent) {
    hovering = false
    needsDisplay = true
  }

  private var pill: NSBezierPath {
    NSBezierPath(roundedRect: bounds.insetBy(dx: 8, dy: 0), xRadius: 7, yRadius: 7)
  }

  override func drawBackground(in dirtyRect: NSRect) {
    guard hovering, !isSelected else { return }
    Palette.hover.setFill()
    pill.fill()
  }

  override func drawSelection(in dirtyRect: NSRect) {
    Palette.selected.setFill()
    pill.fill()
  }

  override var isEmphasized: Bool {
    get { false }
    set {}
  }

  override var interiorBackgroundStyle: NSView.BackgroundStyle { .normal }

  override var isSelected: Bool {
    didSet {
      for case let cell as SidebarCell in subviews { cell.selected = isSelected }
    }
  }
}

/// A row's content: an optional icon and a title, in the chrome's font and colours.
private final class SidebarCell: NSTableCellView {
  private let icon = NSImageView()
  private let hasIcon: Bool

  var symbol: String? {
    didSet {
      guard symbol != oldValue else { return }
      icon.image = symbol.flatMap { NSImage(systemSymbolName: $0, accessibilityDescription: nil) }
    }
  }
  var selected = false { didSet { updateColors() } }
  var dimmed = false { didSet { updateColors() } }
  /// Top-level headings read a little stronger than the ones under them.
  var prominent = false { didSet { updateColors() } }

  init(identifier: NSUserInterfaceItemIdentifier, icon hasIcon: Bool) {
    self.hasIcon = hasIcon
    super.init(frame: .zero)
    self.identifier = identifier
    let text = NSTextField(labelWithString: "")
    text.font = Theme.ui(13)
    text.lineBreakMode = .byTruncatingTail
    text.cell?.truncatesLastVisibleLine = true
    text.translatesAutoresizingMaskIntoConstraints = false
    addSubview(text)
    textField = text
    if hasIcon {
      icon.symbolConfiguration = .init(pointSize: 12, weight: .regular)
      icon.translatesAutoresizingMaskIntoConstraints = false
      addSubview(icon)
      NSLayoutConstraint.activate([
        icon.leadingAnchor.constraint(equalTo: leadingAnchor, constant: 4),
        icon.centerYAnchor.constraint(equalTo: centerYAnchor),
        icon.widthAnchor.constraint(equalToConstant: 16),
        text.leadingAnchor.constraint(equalTo: icon.trailingAnchor, constant: 7),
      ])
    } else {
      text.leadingAnchor.constraint(equalTo: leadingAnchor, constant: 6).isActive = true
    }
    NSLayoutConstraint.activate([
      text.trailingAnchor.constraint(lessThanOrEqualTo: trailingAnchor, constant: -6),
      text.centerYAnchor.constraint(equalTo: centerYAnchor),
    ])
    updateColors()
  }

  required init?(coder: NSCoder) { fatalError() }

  // The row draws the selection; text keeps its own colours on it.
  override var backgroundStyle: NSView.BackgroundStyle {
    get { .normal }
    set {}
  }

  private func updateColors() {
    let text: NSColor =
      dimmed ? Palette.tertiaryText
      : selected || (prominent && !hasIcon) ? Palette.text
      : hasIcon ? Palette.text : Palette.secondaryText
    textField?.textColor = text
    icon.contentTintColor = selected ? Palette.secondaryText : Palette.tertiaryText
  }
}

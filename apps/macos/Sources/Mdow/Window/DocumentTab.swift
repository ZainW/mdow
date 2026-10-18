import AppKit
import MdowKit

/// One tab in a window: a document pane, or two side by side in split view, and the folder the
/// sidebar shows while the tab is selected.
@MainActor final class DocumentTab {
  let paneSplit = NSSplitViewController()
  private(set) var panes: [DocumentPane] = []
  /// The pane the sidebar, title and menu commands follow: the one last clicked.
  var activePane: DocumentPane
  /// A folder the user opened; without one, the sidebar shows the document's own folder.
  var openedFolder: URL?
  var scannedFolder: URL?
  var folderTree: FolderNode?

  init(pane: DocumentPane) {
    activePane = pane
    let splitView = ChromeSplitView()
    splitView.lineColor = Palette.line
    splitView.isVertical = true
    paneSplit.splitView = splitView
    add(pane)
  }

  var fileURL: URL? { activePane.fileURL }
  var isEmpty: Bool { panes.count == 1 && activePane.fileURL == nil }
  var openURLs: [URL] { panes.compactMap(\.fileURL) }

  /// The folder the sidebar shows for this tab.
  var sidebarFolder: URL? { openedFolder ?? fileURL?.deletingLastPathComponent() }

  func add(_ pane: DocumentPane) {
    panes.append(pane)
    let item = NSSplitViewItem(viewController: pane.reader)
    item.minimumThickness = 300
    paneSplit.addSplitViewItem(item)
  }

  func remove(_ pane: DocumentPane) {
    guard let index = panes.firstIndex(where: { $0 === pane }) else { return }
    pane.close()
    panes.remove(at: index)
    if let item = paneSplit.splitViewItems.first(where: { $0.viewController === pane.reader }) {
      paneSplit.removeSplitViewItem(item)
    }
    if activePane === pane, let first = panes.first { activePane = first }
  }

  func close() {
    panes.forEach { $0.close() }
  }

  var title: String {
    if let fileURL { return FileManager.default.displayName(atPath: fileURL.path) }
    if let openedFolder { return openedFolder.lastPathComponent }
    return "New Tab"
  }

  var stripItem: TabStripItem {
    let symbol: String
    if let fileURL {
      symbol = FileKind(url: fileURL) == .html ? "globe" : "doc.text"
    } else {
      symbol = openedFolder == nil ? "sparkles.rectangle.stack" : "folder"
    }
    return TabStripItem(
      title: title, symbol: symbol,
      toolTip: (fileURL ?? openedFolder).map { $0.abbreviatedPath } ?? "New Tab")
  }
}

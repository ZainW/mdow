import AppKit
import MdowKit
import SwiftUI

/// What the companion needs from the window it sits in.
@MainActor protocol CompanionDocumentContext: AnyObject {
  /// The document on screen, if any.
  var documentURL: URL? { get }
  /// A folder the user opened, if any. Without one the companion works in the document's folder.
  var folderURL: URL? { get }
  /// Documents the user can attach with @, usually the sidebar's folder.
  var folderDocuments: [URL] { get }
  /// Shows a document the companion refers to, such as one with a suggested edit.
  func companionOpenDocument(_ url: URL)
}

extension CompanionDocumentContext {
  var folderDocuments: [URL] { [] }
  func companionOpenDocument(_ url: URL) { AppDelegate.shared.open([url], from: nil) }
}

/// What the panel shows about the window's document; refreshed by `documentContextChanged()`.
@MainActor @Observable
final class CompanionDocumentState {
  var documentURL: URL?
  var folderURL: URL?
  var documents: [URL] = []
}

/// The companion inspector: a trailing split view item hosting the SwiftUI panel.
final class CompanionViewController: NSViewController {
  static let toolbarItemIdentifier = NSToolbarItem.Identifier("companion")

  let session = CompanionSession()
  let document = CompanionDocumentState()
  private var collapseObservation: NSKeyValueObservation?
  private var windowObserver: NSObjectProtocol?
  private var visible = false

  weak var context: CompanionDocumentContext? {
    didSet {
      session.context = context
      documentContextChanged()
    }
  }

  override func loadView() {
    let host = NSHostingView(
      rootView: CompanionPanel(session: session, runtime: .shared, document: document) {
        [weak self] url in self?.context?.companionOpenDocument(url)
      })
    host.sizingOptions = []
    view = host
    view.setAccessibilityLabel("Companion")
  }

  /// A trailing inspector item for `context`'s split view, collapsed until the user opens it.
  func makeSplitItem(context: CompanionDocumentContext) -> NSSplitViewItem {
    self.context = context
    let item = NSSplitViewItem(inspectorWithViewController: self)
    item.minimumThickness = 300
    item.maximumThickness = 560
    item.preferredThicknessFraction = 0.3
    item.canCollapse = true
    item.isCollapsed = true
    item.holdingPriority = .init(260)
    collapseObservation = item.observe(\.isCollapsed, options: [.new]) { [weak self] item, _ in
      MainActor.assumeIsolated { self?.updateVisibility() }
    }
    return item
  }

  func makeToolbarItem(target: AnyObject, action: Selector) -> NSToolbarItem {
    let item = NSToolbarItem(itemIdentifier: Self.toolbarItemIdentifier)
    item.label = "Companion"
    item.paletteLabel = "Companion"
    item.toolTip = "Show or hide the companion"
    item.image = NSImage(systemSymbolName: "sparkles", accessibilityDescription: "Companion")
    item.isBordered = true
    item.target = target
    item.action = action
    return item
  }

  /// Call when the window's document or folder changes.
  func documentContextChanged() {
    let documentURL = context?.documentURL
    let folderURL = context?.folderURL
    if document.documentURL != documentURL { document.documentURL = documentURL }
    if document.folderURL != folderURL { document.folderURL = folderURL }
    if visible { refreshDocuments() }
  }

  private func refreshDocuments() {
    let documents = context?.folderDocuments ?? []
    if document.documents != documents { document.documents = documents }
  }

  /// Paths with an edit waiting for review, e.g. to mark tabs.
  var pendingReviewPaths: [String] { session.conversation.pendingReviewPaths }

  override func viewDidAppear() {
    super.viewDidAppear()
    if windowObserver == nil, let window = view.window {
      windowObserver = NotificationCenter.default.addObserver(
        forName: NSWindow.willCloseNotification, object: window, queue: .main
      ) { [weak self] _ in
        MainActor.assumeIsolated {
          guard let self else { return }
          self.session.shutdown()
          self.setVisible(false)
          if let observer = self.windowObserver { NotificationCenter.default.removeObserver(observer) }
          self.windowObserver = nil
        }
      }
    }
    updateVisibility()
  }

  override func viewDidDisappear() {
    super.viewDidDisappear()
    updateVisibility()
  }

  private func updateVisibility() {
    let item = (parent as? NSSplitViewController)?.splitViewItem(for: self)
    setVisible(view.window != nil && item?.isCollapsed == false)
  }

  private func setVisible(_ visible: Bool) {
    guard visible != self.visible else { return }
    self.visible = visible
    session.runtime.paneVisibilityChanged(visible)
    guard visible else { return }
    refreshDocuments()
    if !session.runtime.isAvailable {
      Task { await session.runtime.connect(directory: session.directory) }
    }
  }

  /// Moves focus to the message field.
  func focusComposer() {
    view.window?.makeFirstResponder(view)
    NotificationCenter.default.post(name: .companionFocusComposer, object: session)
  }
}

extension Notification.Name {
  static let companionFocusComposer = Notification.Name("MdowCompanionFocusComposer")
}

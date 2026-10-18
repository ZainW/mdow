import AppKit
import MdowKit

/// The window side of the companion: what it reads from the window, and the toggle action.
extension DocumentWindowController: CompanionDocumentContext {
  var documentURL: URL? { fileURL }
  var folderURL: URL? { openedFolder }
  var folderDocuments: [URL] { folderTree?.allFiles.filter(FileKind.isOpenable) ?? [] }

  func companionOpenDocument(_ url: URL) { open(url) }

  /// View › Show Companion (⌥⌘0) and the header's sparkle button.
  @objc func toggleCompanion(_ sender: Any?) {
    let show = companionItem.isCollapsed
    if !show, let responder = window?.firstResponder as? NSView,
      responder.isDescendant(of: companion.view)
    {
      window?.makeFirstResponder(reader.mode == .markdown ? reader.textView : nil)
    }
    NSAnimationContext.runAnimationGroup { context in
      context.duration = NSWorkspace.shared.accessibilityDisplayShouldReduceMotion ? 0 : 0.2
      context.timingFunction = CAMediaTimingFunction(name: .easeOut)
      companionItem.animator().isCollapsed = !show
    } completionHandler: {
      MainActor.assumeIsolated {
        self.updateChrome()
        if show { self.companion.focusComposer() }
      }
    }
  }
}

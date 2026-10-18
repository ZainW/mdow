import AppKit

/// `MDOW_ACTIONS="appearance=light;sidebar=files;palette=read;find=Pele;scroll=diagrams"` runs
/// these once the first window has settled, for screenshots and manual checks.
@MainActor enum DebugActions {
  static func runIfRequested() {
    guard let script = ProcessInfo.processInfo.environment["MDOW_ACTIONS"], !script.isEmpty else { return }
    Task {
      try? await Task.sleep(for: .seconds(1.2))
      NSApp.activate()
      for step in script.split(separator: ";") {
        let parts = step.split(separator: "=", maxSplits: 1).map(String.init)
        run(parts[0], parts.count > 1 ? parts[1] : "")
        try? await Task.sleep(for: .milliseconds(700))
      }
    }
  }

  private static var controller: DocumentWindowController? {
    NSApp.orderedWindows.lazy.compactMap { $0.windowController as? DocumentWindowController }.first
  }

  private static func run(_ action: String, _ value: String) {
    switch action {
    case "appearance":
      Preferences.shared.appearance = AppearanceMode(rawValue: value) ?? .system
    case "sidebar":
      switch value {
      case "files": controller?.showFiles(nil)
      case "recents": controller?.showRecents(nil)
      case "hide": NSApp.sendAction(#selector(NSSplitViewController.toggleSidebar(_:)), to: nil, from: nil)
      default: controller?.showOutline(nil)
      }
    case "palette":
      controller?.quickOpen(nil)
      if !value.isEmpty, let field = NSApp.keyWindow?.firstResponder as? NSTextView {
        field.insertText(value, replacementRange: field.selectedRange())
      }
    case "find":
      guard let controller else { return }
      controller.window?.makeFirstResponder(controller.reader.textView)
      let item = NSMenuItem()
      item.tag = NSTextFinder.Action.showFindInterface.rawValue
      controller.reader.textView.performTextFinderAction(item)
      if !value.isEmpty, let field = NSApp.keyWindow?.firstResponder as? NSTextView {
        field.insertText(value, replacementRange: field.selectedRange())
      }
    case "scroll":
      _ = controller?.reader.scroll(toAnchor: value)
    case "settings":
      AppDelegate.shared.showSettings(nil)
    case "shortcuts":
      AppDelegate.shared.showShortcuts(nil)
    case "newtab":
      AppDelegate.shared.newTabAction(nil)
    case "open":
      AppDelegate.shared.open([URL(fileURLWithPath: value)], from: nil)
    case "zoom":
      Preferences.shared.zoom = Double(value) ?? 1
    case "companion":
      // `companion` shows the pane, `companion=hide` hides it, `companion=ask:text` sends text.
      guard let controller else { return }
      if (value == "hide") != controller.companionItem.isCollapsed { controller.toggleCompanion(nil) }
      if value.hasPrefix("ask:") { controller.companion.session.send(String(value.dropFirst(4))) }
    default:
      break
    }
  }
}

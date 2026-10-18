import AppKit
import MdowKit
import SwiftUI
import UniformTypeIdentifiers

@MainActor final class AppDelegate: NSObject, NSApplicationDelegate, NSMenuDelegate {
  static let shared = AppDelegate()

  static let openableTypes: [UTType] = {
    var types: [UTType] = [.html]
    if let markdown = UTType("net.daringfireball.markdown") { types.append(markdown) }
    for ext in FileKind.markdownExtensions {
      if let type = UTType(filenameExtension: ext) { types.append(type) }
    }
    return types
  }()

  private var controllers: [DocumentWindowController] = []
  private var settingsWindow: NSWindow?
  private var shortcutsWindow: NSWindow?
  private var launched = false
  private var keyMonitor: Any?

  // MARK: Lifecycle

  func applicationWillFinishLaunching(_ notification: Notification) {
    NSApp.mainMenu = MainMenu.build(recentsDelegate: self)
    NSApp.appearance = Preferences.shared.appearance.appearance
    // Tabs live in the window's own header.
    NSWindow.allowsAutomaticWindowTabbing = false
    Theme.registerFonts()
  }

  func applicationDidFinishLaunching(_ notification: Notification) {
    launched = true
    Updater.shared.start()
    keyMonitor = NSEvent.addLocalMonitorForEvents(matching: .keyDown) { [weak self] event in
      self?.handleTabShortcut(event) == true ? nil : event
    }
    let arguments = CommandLine.arguments.dropFirst().filter { !$0.hasPrefix("-") }
    let urls = arguments.map { URL(fileURLWithPath: $0) }.filter {
      FileManager.default.fileExists(atPath: $0.path)
    }
    if !urls.isEmpty { open(urls, from: nil) }
    // Restored windows and Finder open events arrive before this runs out.
    DispatchQueue.main.async {
      if self.controllers.isEmpty { self.newWindow() }
      DebugActions.runIfRequested()
    }
  }

  func application(_ application: NSApplication, open urls: [URL]) {
    open(urls, from: nil)
  }

  func applicationShouldHandleReopen(_ sender: NSApplication, hasVisibleWindows flag: Bool) -> Bool {
    if !flag, controllers.allSatisfy({ $0.window?.isMiniaturized != true }) { newWindow() }
    return true
  }

  func applicationSupportsSecureRestorableState(_ app: NSApplication) -> Bool { true }

  func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool { false }

  func applicationDockMenu(_ sender: NSApplication) -> NSMenu? {
    let menu = NSMenu()
    menu.addItem(withTitle: "New Window", action: #selector(newWindowAction(_:)), keyEquivalent: "")
      .target = self
    return menu
  }

  // MARK: Windows

  func makeController() -> DocumentWindowController {
    let controller = DocumentWindowController()
    controllers.append(controller)
    return controller
  }

  func controllerClosed(_ controller: DocumentWindowController) {
    controllers.removeAll { $0 === controller }
  }

  private var keyController: DocumentWindowController? {
    (NSApp.keyWindow?.windowController as? DocumentWindowController)
      ?? (NSApp.mainWindow?.windowController as? DocumentWindowController)
      // Inactive (e.g. opening from Finder): the frontmost document window.
      ?? NSApp.orderedWindows.lazy.compactMap { $0.windowController as? DocumentWindowController }
      .first { $0.window?.isVisible == true && $0.window?.isMiniaturized == false }
  }

  @discardableResult
  func newWindow() -> DocumentWindowController {
    let controller = makeController()
    if let last = controllers.dropLast().last?.window, let window = controller.window {
      window.setFrameTopLeftPoint(window.cascadeTopLeft(from: NSPoint(x: last.frame.minX, y: last.frame.maxY)))
    }
    controller.showWindow(nil)
    return controller
  }

  @discardableResult
  func newTab(from source: DocumentWindowController?) -> DocumentWindowController {
    guard let host = source ?? keyController else { return newWindow() }
    host.addTab()
    host.window?.makeKeyAndOrderFront(nil)
    return host
  }

  /// Opens files and folders: an empty window is reused, a document that's already open is
  /// brought forward, and the rest open in tabs or windows per the user's setting.
  func open(_ urls: [URL], from source: DocumentWindowController?) {
    for url in urls {
      let url = url.standardizedFileURL
      if let existing = controllers.first(where: { $0.openURLs.contains(url) }) {
        _ = existing.reveal(url)
        existing.window?.makeKeyAndOrderFront(nil)
        continue
      }
      let target = source ?? keyController
      if let target, target.isEmpty {
        target.open(url)
        target.window?.makeKeyAndOrderFront(nil)
        continue
      }
      let controller = Preferences.shared.openInTabs && target != nil ? newTab(from: target) : newWindow()
      controller.open(url)
    }
    NSApp.activate()
  }

  func openInNewTab(_ url: URL, from source: DocumentWindowController) {
    if let existing = controllers.first(where: { $0.openURLs.contains(url.standardizedFileURL) }) {
      _ = existing.reveal(url.standardizedFileURL)
      existing.window?.makeKeyAndOrderFront(nil)
      return
    }
    newTab(from: source).open(url)
  }

  /// ⌘1–⌘9 pick a tab, ⌘9 the last, like Safari.
  private func handleTabShortcut(_ event: NSEvent) -> Bool {
    guard event.modifierFlags.intersection(.deviceIndependentFlagsMask) == .command,
      let characters = event.charactersIgnoringModifiers, let number = Int(characters),
      (1...9).contains(number),
      let controller = NSApp.keyWindow?.windowController as? DocumentWindowController,
      controller.tabs.count > 1
    else { return false }
    controller.selectTab(at: number == 9 ? controller.tabs.count - 1 : number - 1)
    return true
  }

  // MARK: Actions (when no document window handles them)

  @objc func newWindowAction(_ sender: Any?) { newWindow() }

  @objc func newTabAction(_ sender: Any?) {
    if keyController != nil { newTab(from: keyController) } else { newWindow() }
  }

  @objc func openDocument(_ sender: Any?) {
    let panel = NSOpenPanel()
    panel.allowsMultipleSelection = true
    panel.allowedContentTypes = Self.openableTypes
    guard panel.runModal() == .OK else { return }
    open(panel.urls, from: nil)
  }

  @objc func openFolder(_ sender: Any?) {
    let panel = NSOpenPanel()
    panel.canChooseFiles = false
    panel.canChooseDirectories = true
    panel.prompt = "Open Folder"
    guard panel.runModal() == .OK, let url = panel.url else { return }
    let controller = keyController.flatMap { $0.isEmpty ? $0 : nil } ?? newWindow()
    controller.openFolder(at: url)
  }

  @objc func showSettings(_ sender: Any?) {
    if settingsWindow == nil {
      let window = NSWindow(contentViewController: NSHostingController(rootView: SettingsView()))
      window.title = "Settings"
      window.styleMask = [.titled, .closable]
      window.isReleasedWhenClosed = false
      window.identifier = .init("MdowSettings")
      window.center()
      settingsWindow = window
    }
    settingsWindow?.makeKeyAndOrderFront(nil)
  }

  @objc func showShortcuts(_ sender: Any?) {
    if shortcutsWindow == nil {
      let window = NSWindow(contentViewController: NSHostingController(rootView: ShortcutsView()))
      window.title = "Keyboard Shortcuts"
      window.styleMask = [.titled, .closable]
      window.isReleasedWhenClosed = false
      window.center()
      shortcutsWindow = window
    }
    shortcutsWindow?.makeKeyAndOrderFront(nil)
  }

  @objc func openRecent(_ sender: NSMenuItem) {
    guard let url = sender.representedObject as? URL else { return }
    open([url], from: nil)
  }

  @objc func clearRecents(_ sender: Any?) {
    Preferences.shared.clearRecents()
  }

  // MARK: Open Recent menu

  func menuNeedsUpdate(_ menu: NSMenu) {
    menu.removeAllItems()
    let recents = Preferences.shared.recents.prefix(15)
    for url in recents {
      let item = NSMenuItem(
        title: FileManager.default.displayName(atPath: url.path), action: #selector(openRecent(_:)),
        keyEquivalent: "")
      item.target = self
      item.representedObject = url
      item.toolTip = url.abbreviatedPath
      let icon = NSWorkspace.shared.icon(forFile: url.path)
      icon.size = NSSize(width: 16, height: 16)
      item.image = icon
      menu.addItem(item)
    }
    if !recents.isEmpty { menu.addItem(.separator()) }
    let clear = NSMenuItem(title: "Clear Menu", action: #selector(clearRecents(_:)), keyEquivalent: "")
    clear.target = self
    clear.isEnabled = !recents.isEmpty
    menu.addItem(clear)
  }
}

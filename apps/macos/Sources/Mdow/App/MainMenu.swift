import AppKit
import MdowKit

/// The menu bar. Standard items use AppKit's own actions so the system can extend them (tab
/// items, Writing Tools, dictation, full screen).
@MainActor enum MainMenu {
  static func build(recentsDelegate: NSMenuDelegate) -> NSMenu {
    let main = NSMenu()
    main.addItem(submenu(appMenu()))
    main.addItem(submenu(fileMenu(recentsDelegate: recentsDelegate)))
    main.addItem(submenu(editMenu()))
    main.addItem(submenu(viewMenu()))
    main.addItem(submenu(goMenu()))
    let window = windowMenu()
    main.addItem(submenu(window))
    NSApp.windowsMenu = window
    let help = helpMenu()
    main.addItem(submenu(help))
    NSApp.helpMenu = help
    return main
  }

  private static func submenu(_ menu: NSMenu) -> NSMenuItem {
    let item = NSMenuItem(title: menu.title, action: nil, keyEquivalent: "")
    item.submenu = menu
    return item
  }

  private static func item(
    _ title: String, _ action: Selector?, _ key: String = "",
    _ modifiers: NSEvent.ModifierFlags = .command, symbol: String? = nil
  ) -> NSMenuItem {
    let item = NSMenuItem(title: title, action: action, keyEquivalent: key)
    item.keyEquivalentModifierMask = modifiers
    if let symbol { item.image = NSImage(systemSymbolName: symbol, accessibilityDescription: nil) }
    return item
  }

  private static func appMenu() -> NSMenu {
    let name = "Mdow"
    let menu = NSMenu(title: name)
    menu.addItem(item("About \(name)", #selector(NSApplication.orderFrontStandardAboutPanel(_:)), ""))
    menu.addItem(.separator())
    menu.addItem(item("Settings…", #selector(AppDelegate.showSettings(_:)), ","))
    menu.addItem(.separator())
    let services = item("Services", nil)
    services.submenu = NSMenu(title: "Services")
    NSApp.servicesMenu = services.submenu
    menu.addItem(services)
    menu.addItem(.separator())
    menu.addItem(item("Hide \(name)", #selector(NSApplication.hide(_:)), "h"))
    menu.addItem(item("Hide Others", #selector(NSApplication.hideOtherApplications(_:)), "h", [.command, .option]))
    menu.addItem(item("Show All", #selector(NSApplication.unhideAllApplications(_:))))
    menu.addItem(.separator())
    menu.addItem(item("Quit \(name)", #selector(NSApplication.terminate(_:)), "q"))
    return menu
  }

  private static func fileMenu(recentsDelegate: NSMenuDelegate) -> NSMenu {
    let menu = NSMenu(title: "File")
    menu.addItem(item("New Window", #selector(AppDelegate.newWindowAction(_:)), "n"))
    menu.addItem(item("New Tab", #selector(AppDelegate.newTabAction(_:)), "t"))
    menu.addItem(item("Open…", #selector(DocumentWindowController.openDocument(_:)), "o"))
    menu.addItem(item("Open Folder…", #selector(DocumentWindowController.openFolder(_:)), "o", [.command, .shift]))
    let recent = item("Open Recent", nil)
    let recentMenu = NSMenu(title: "Open Recent")
    recentMenu.delegate = recentsDelegate
    recentMenu.autoenablesItems = false
    recent.submenu = recentMenu
    menu.addItem(recent)
    menu.addItem(.separator())
    menu.addItem(item("Close", #selector(NSWindow.performClose(_:)), "w"))
    menu.addItem(item("Reload", #selector(DocumentWindowController.reloadDocument(_:)), "r"))
    menu.addItem(.separator())
    menu.addItem(item("Show in Finder", #selector(DocumentWindowController.showInFinder(_:)), "r", [.command, .option]))
    menu.addItem(item("Open in Editor", #selector(DocumentWindowController.openWithDefaultEditor(_:)), "e", [.command, .shift]))
    menu.addItem(item("Copy Path", #selector(DocumentWindowController.copyDocumentPath(_:)), "c", [.command, .option]))
    menu.addItem(.separator())
    menu.addItem(item("Export as PDF…", #selector(DocumentWindowController.exportAsPDF(_:))))
    menu.addItem(item("Page Setup…", #selector(NSApplication.runPageLayout(_:)), "p", [.command, .shift]))
    menu.addItem(item("Print…", #selector(DocumentWindowController.printDocument(_:)), "p"))
    return menu
  }

  private static func editMenu() -> NSMenu {
    let menu = NSMenu(title: "Edit")
    menu.addItem(item("Copy", #selector(NSText.copy(_:)), "c"))
    menu.addItem(item("Select All", #selector(NSText.selectAll(_:)), "a"))
    menu.addItem(.separator())
    let find = item("Find", nil)
    let findMenu = NSMenu(title: "Find")
    func finder(_ title: String, _ action: NSTextFinder.Action, _ key: String, _ modifiers: NSEvent.ModifierFlags = .command) {
      let entry = item(title, #selector(NSResponder.performTextFinderAction(_:)), key, modifiers)
      entry.tag = action.rawValue
      findMenu.addItem(entry)
    }
    finder("Find…", .showFindInterface, "f")
    finder("Find Next", .nextMatch, "g")
    finder("Find Previous", .previousMatch, "g", [.command, .shift])
    finder("Use Selection for Find", .setSearchString, "e")
    findMenu.addItem(item("Jump to Selection", #selector(NSTextView.centerSelectionInVisibleArea(_:)), "j"))
    find.submenu = findMenu
    menu.addItem(find)
    menu.addItem(.separator())
    let speech = item("Speech", nil)
    let speechMenu = NSMenu(title: "Speech")
    speechMenu.addItem(item("Start Speaking", #selector(NSTextView.startSpeaking(_:))))
    speechMenu.addItem(item("Stop Speaking", #selector(NSTextView.stopSpeaking(_:))))
    speech.submenu = speechMenu
    menu.addItem(speech)
    return menu
  }

  private static func viewMenu() -> NSMenu {
    let menu = NSMenu(title: "View")
    menu.addItem(item("Show Sidebar", #selector(NSSplitViewController.toggleSidebar(_:)), "s", [.command, .control]))
    menu.addItem(item("Outline", #selector(DocumentWindowController.showOutline(_:)), "1", [.command, .control]))
    menu.addItem(item("Files", #selector(DocumentWindowController.showFiles(_:)), "2", [.command, .control]))
    menu.addItem(item("Recents", #selector(DocumentWindowController.showRecents(_:)), "3", [.command, .control]))
    menu.addItem(.separator())
    menu.addItem(item("Split View", #selector(DocumentWindowController.toggleSplitView(_:)), "\\"))
    menu.addItem(.separator())
    menu.addItem(item("Actual Size", #selector(DocumentWindowController.actualSize(_:)), "0"))
    menu.addItem(item("Zoom In", #selector(DocumentWindowController.zoomIn(_:)), "+"))
    let zoomInAlternate = item("Zoom In", #selector(DocumentWindowController.zoomIn(_:)), "=")
    zoomInAlternate.isAlternate = true
    zoomInAlternate.isHidden = true
    zoomInAlternate.allowsKeyEquivalentWhenHidden = true
    menu.addItem(zoomInAlternate)
    menu.addItem(item("Zoom Out", #selector(DocumentWindowController.zoomOut(_:)), "-"))
    let width = item("Text Width", nil)
    let widthMenu = NSMenu(title: "Text Width")
    for option in ReadingWidth.allCases {
      let entry = item(option.displayName, #selector(DocumentWindowController.setReadingWidth(_:)))
      entry.representedObject = option.rawValue
      widthMenu.addItem(entry)
    }
    width.submenu = widthMenu
    menu.addItem(width)
    menu.addItem(.separator())
    menu.addItem(item("Enter Full Screen", #selector(NSWindow.toggleFullScreen(_:)), "f", [.command, .control]))
    return menu
  }

  private static func goMenu() -> NSMenu {
    let menu = NSMenu(title: "Go")
    menu.addItem(item("Back", #selector(DocumentWindowController.goBack(_:)), "["))
    menu.addItem(item("Forward", #selector(DocumentWindowController.goForward(_:)), "]"))
    menu.addItem(.separator())
    menu.addItem(item("Previous Heading", #selector(DocumentWindowController.previousHeading(_:)), String(Character(UnicodeScalar(NSUpArrowFunctionKey)!)), [.command, .option]))
    menu.addItem(item("Next Heading", #selector(DocumentWindowController.nextHeading(_:)), String(Character(UnicodeScalar(NSDownArrowFunctionKey)!)), [.command, .option]))
    menu.addItem(.separator())
    menu.addItem(item("Quick Open…", #selector(DocumentWindowController.quickOpen(_:)), "k"))
    return menu
  }

  private static func windowMenu() -> NSMenu {
    let menu = NSMenu(title: "Window")
    menu.addItem(item("Minimize", #selector(NSWindow.performMiniaturize(_:)), "m"))
    menu.addItem(item("Zoom", #selector(NSWindow.performZoom(_:))))
    menu.addItem(.separator())
    menu.addItem(item("Bring All to Front", #selector(NSApplication.arrangeInFront(_:))))
    return menu
  }

  private static func helpMenu() -> NSMenu {
    let menu = NSMenu(title: "Help")
    menu.addItem(item("Keyboard Shortcuts", #selector(AppDelegate.showShortcuts(_:)), "/"))
    return menu
  }
}

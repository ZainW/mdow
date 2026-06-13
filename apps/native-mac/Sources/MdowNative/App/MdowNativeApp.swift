import AppKit
import os
import SwiftUI

final class AppDelegate: NSObject, NSApplicationDelegate {
    static let openURLsNotification = Notification.Name("MdowNativeOpenURLs")
    static let focusSearchNotification = Notification.Name("MdowNativeFocusSearch")
    static let nextSearchResultNotification = Notification.Name("MdowNativeNextSearchResult")
    static let previousSearchResultNotification = Notification.Name("MdowNativePreviousSearchResult")
    static let showCommandPaletteNotification = Notification.Name("MdowNativeShowCommandPalette")
    static let showShortcutsNotification = Notification.Name("MdowNativeShowShortcuts")
    static let toggleSidebarNotification = Notification.Name("MdowNativeToggleSidebar")
    static let toggleWideModeNotification = Notification.Name("MdowNativeToggleWideMode")
    static let closeTabNotification = Notification.Name("MdowNativeCloseTab")
    static let nextTabNotification = Notification.Name("MdowNativeNextTab")
    static let previousTabNotification = Notification.Name("MdowNativePreviousTab")
    static let zoomInNotification = Notification.Name("MdowNativeZoomIn")
    static let zoomOutNotification = Notification.Name("MdowNativeZoomOut")
    static let resetZoomNotification = Notification.Name("MdowNativeResetZoom")

    @MainActor private static var hasOpenURLObserver = false
    @MainActor private static var pendingOpenURLs: [URL] = []

    private var keyMonitor: Any?
    private let logger = Logger(subsystem: "com.zain.mdow.native", category: "Launch")

    func application(
        _ application: NSApplication,
        shouldSaveApplicationState coder: NSCoder
    ) -> Bool {
        false
    }

    func application(
        _ application: NSApplication,
        shouldRestoreApplicationState coder: NSCoder
    ) -> Bool {
        false
    }

    func applicationSupportsSecureRestorableState(_ application: NSApplication) -> Bool {
        false
    }

    func applicationDidFinishLaunching(_ notification: Notification) {
        Task { @MainActor in
            finishLaunchingOnMainActor()
        }
    }

    @MainActor
    private func finishLaunchingOnMainActor() {
        logger.info("Application did finish launching")
        NSApp.setActivationPolicy(.regular)
        NSApp.activate(ignoringOtherApps: true)
        installKeyMonitor()
        LaunchWindowRecovery.shared.schedule()
    }

    func application(_ application: NSApplication, open urls: [URL]) {
        Task { @MainActor in
            Self.deliverOpenURLs(urls)
            LaunchWindowRecovery.shared.schedule()
        }
    }

    func application(_ sender: NSApplication, openFile filename: String) -> Bool {
        Task { @MainActor in
            Self.deliverOpenURLs([URL(fileURLWithPath: filename)])
            LaunchWindowRecovery.shared.schedule()
        }
        return true
    }

    func applicationShouldHandleReopen(
        _ sender: NSApplication,
        hasVisibleWindows flag: Bool
    ) -> Bool {
        guard !flag else { return true }
        Task { @MainActor in
            LaunchWindowRecovery.shared.reopenDocumentWindow()
        }
        return false
    }

    @MainActor
    static func markOpenURLObserverReady() -> [URL] {
        hasOpenURLObserver = true
        defer { pendingOpenURLs = [] }
        return pendingOpenURLs
    }

    @MainActor
    private static func deliverOpenURLs(_ urls: [URL]) {
        guard hasOpenURLObserver else {
            pendingOpenURLs.append(contentsOf: urls)
            return
        }

        NotificationCenter.default.post(name: openURLsNotification, object: urls)
    }

    @MainActor
    private func installKeyMonitor() {
        keyMonitor = NSEvent.addLocalMonitorForEvents(matching: .keyDown) { event in
            let modifiers = event.modifierFlags.intersection(.deviceIndependentFlagsMask)
            guard modifiers.contains(.command),
                  let characters = event.charactersIgnoringModifiers?.lowercased() else {
                return event
            }

            switch characters {
            case "f" where modifiers == .command:
                NotificationCenter.default.post(name: Self.focusSearchNotification, object: nil)
                return nil
            case "g" where modifiers == .command:
                NotificationCenter.default.post(name: Self.nextSearchResultNotification, object: nil)
                return nil
            case "g" where modifiers == [.command, .shift]:
                NotificationCenter.default.post(name: Self.previousSearchResultNotification, object: nil)
                return nil
            case "k" where modifiers == .command:
                NotificationCenter.default.post(name: Self.showCommandPaletteNotification, object: nil)
                return nil
            case "/" where modifiers == .command:
                NotificationCenter.default.post(name: Self.showShortcutsNotification, object: nil)
                return nil
            case "b" where modifiers == .command:
                NotificationCenter.default.post(name: Self.toggleSidebarNotification, object: nil)
                return nil
            case "=" where modifiers == .command:
                NotificationCenter.default.post(name: Self.zoomInNotification, object: nil)
                return nil
            case "+" where modifiers == .command:
                NotificationCenter.default.post(name: Self.zoomInNotification, object: nil)
                return nil
            case "-" where modifiers == .command:
                NotificationCenter.default.post(name: Self.zoomOutNotification, object: nil)
                return nil
            case "0" where modifiers == .command:
                NotificationCenter.default.post(name: Self.resetZoomNotification, object: nil)
                return nil
            default:
                return event
            }
        }
    }

}

@MainActor
final class LaunchWindowRecovery {
    static let shared = LaunchWindowRecovery()

    private static let documentWindowIdentifier = NSUserInterfaceItemIdentifier("main")
    private static let fallbackDocumentWindowIdentifier = NSUserInterfaceItemIdentifier("MdowDocumentWindow")

    private var fallbackWindowController: NSWindowController?
    private var didOpenFallbackWindow = false
    private let logger = Logger(subsystem: "com.zain.mdow.native", category: "Launch")

    func schedule() {
        for delay in [0.2, 0.5, 0.9, 1.3, 1.8, 2.6, 3.6] {
            DispatchQueue.main.asyncAfter(deadline: .now() + delay) { [weak self] in
                self?.coalesceDuplicateDocumentWindows()
                self?.frontExistingDocumentWindowIfPossible()
            }
        }

        for delay in [1.2, 2.4, 3.6] {
            DispatchQueue.main.asyncAfter(deadline: .now() + delay) { [weak self] in
                self?.openFallbackWindowIfNeeded()
            }
        }
    }

    func reopenDocumentWindow() {
        coalesceDuplicateDocumentWindows()
        if frontExistingDocumentWindowIfPossible() {
            return
        }

        didOpenFallbackWindow = false
        openFallbackWindowIfNeeded()
    }

    private func openFallbackWindowIfNeeded() {
        coalesceDuplicateDocumentWindows()
        guard visibleDocumentWindowCount() == 0 else { return }
        if frontExistingDocumentWindowIfPossible() {
            return
        }
        guard !didOpenFallbackWindow else { return }

        for window in recoverableDocumentWindows() {
            window.close()
        }

        let rootView = ContentView()
            .environmentObject(DocumentStore.shared)
            .frame(minWidth: 720, minHeight: 520)
        let hostingController = NSHostingController(rootView: rootView)
        let window = NSWindow(contentViewController: hostingController)
        window.identifier = Self.fallbackDocumentWindowIdentifier
        window.title = DocumentStore.shared.document?.title ?? "Mdow"
        window.representedURL = DocumentStore.shared.document?.url
        window.setContentSize(NSSize(width: 1040, height: 700))
        window.minSize = NSSize(width: 720, height: 520)
        window.styleMask = [.titled, .closable, .miniaturizable, .resizable, .fullSizeContentView]
        window.titlebarAppearsTransparent = true
        window.titleVisibility = .hidden
        let bestAppearance = window.effectiveAppearance.bestMatch(from: [.aqua, .darkAqua])
        window.backgroundColor = bestAppearance == .darkAqua
            ? MdowStyle.backgroundDark
            : MdowStyle.backgroundLight
        window.isOpaque = true
        window.isReleasedWhenClosed = false
        window.center()

        let controller = NSWindowController(window: window)
        fallbackWindowController = controller
        controller.showWindow(nil)
        didOpenFallbackWindow = true
        foreground(window)
        logger.info("Opened fallback document window")
    }

    @discardableResult
    private func frontExistingDocumentWindowIfPossible() -> Bool {
        guard let window = recoverableDocumentWindows().first else { return false }

        configureWindowForDisplay(window)
        foreground(window)
        return window.isVisible
    }

    private func coalesceDuplicateDocumentWindows() {
        let documentWindows = recoverableDocumentWindows()
        guard documentWindows.count > 1 else { return }

        let windowToKeep = documentWindows.first { $0.isKeyWindow }
            ?? documentWindows.first { $0.isMainWindow }
            ?? documentWindows[0]

        for window in documentWindows where window !== windowToKeep {
            window.close()
        }
        windowToKeep.makeKeyAndOrderFront(nil)
        logger.info("Coalesced duplicate document windows down to one")
    }

    private func recoverableDocumentWindows() -> [NSWindow] {
        NSApp.windows.filter { window in
            isDocumentWindow(window)
                && window.parent == nil
                && !(window is NSPanel)
                && window.frame.width > 10
                && window.frame.height > 10
        }
    }

    private func visibleDocumentWindowCount() -> Int {
        recoverableDocumentWindows().filter { $0.isVisible && !$0.isMiniaturized }.count
    }

    private func isDocumentWindow(_ window: NSWindow) -> Bool {
        let identifier = window.identifier?.rawValue
        return identifier == Self.documentWindowIdentifier.rawValue
            || identifier == Self.fallbackDocumentWindowIdentifier.rawValue
    }

    private func configureWindowForDisplay(_ window: NSWindow) {
        if window.identifier == nil {
            window.identifier = Self.fallbackDocumentWindowIdentifier
        }
        window.title = DocumentStore.shared.document?.title ?? "Mdow"
        window.representedURL = DocumentStore.shared.document?.url
        window.minSize = NSSize(width: 720, height: 520)
        window.styleMask.insert([.titled, .closable, .miniaturizable, .resizable, .fullSizeContentView])
        window.titlebarAppearsTransparent = true
        window.titleVisibility = .hidden
        let bestAppearance = window.effectiveAppearance.bestMatch(from: [.aqua, .darkAqua])
        window.backgroundColor = bestAppearance == .darkAqua
            ? MdowStyle.backgroundDark
            : MdowStyle.backgroundLight
        window.isOpaque = true
    }

    private func foreground(_ window: NSWindow) {
        NSApp.setActivationPolicy(.regular)
        NSRunningApplication.current.activate(options: [.activateAllWindows])
        window.deminiaturize(nil)
        window.orderFrontRegardless()
        window.makeMain()
        window.makeKey()
        window.makeKeyAndOrderFront(nil)
        NSApp.activate(ignoringOtherApps: true)
        NSRunningApplication.current.activate(options: [.activateAllWindows])
    }
}

@main
struct MdowNativeApp: App {
    @NSApplicationDelegateAdaptor(AppDelegate.self) private var appDelegate
    @StateObject private var store = DocumentStore.shared

    init() {
        Task { @MainActor in
            LaunchWindowRecovery.shared.schedule()
        }
    }

    var body: some Scene {
        Window("Mdow", id: "main") {
            ContentView()
                .environmentObject(store)
                .frame(minWidth: 720, minHeight: 520)
        }
        .commands {
            CommandGroup(replacing: .newItem) {
                Button("Open...") {
                    store.openWithPanel()
                }
                .keyboardShortcut("o", modifiers: .command)

                Button("Open Folder...") {
                    store.openFolderWithPanel()
                }
                .keyboardShortcut("o", modifiers: [.command, .shift])
            }

            CommandMenu("Find") {
                Button("Find") {
                    NotificationCenter.default.post(name: AppDelegate.focusSearchNotification, object: nil)
                }
                .keyboardShortcut("f", modifiers: .command)

                Button("Find Next") {
                    NotificationCenter.default.post(name: AppDelegate.nextSearchResultNotification, object: nil)
                }
                .keyboardShortcut("g", modifiers: .command)

                Button("Find Previous") {
                    NotificationCenter.default.post(name: AppDelegate.previousSearchResultNotification, object: nil)
                }
                .keyboardShortcut("g", modifiers: [.command, .shift])
            }

            CommandMenu("Navigate") {
                Button("Quick Open") {
                    NotificationCenter.default.post(
                        name: AppDelegate.showCommandPaletteNotification,
                        object: nil
                    )
                }
                .keyboardShortcut("k", modifiers: .command)

                Button("Toggle Sidebar") {
                    NotificationCenter.default.post(name: AppDelegate.toggleSidebarNotification, object: nil)
                }
                .keyboardShortcut("b", modifiers: .command)

                Button("Toggle Full Width") {
                    NotificationCenter.default.post(name: AppDelegate.toggleWideModeNotification, object: nil)
                }
                .keyboardShortcut("w", modifiers: [.command, .shift])

                Button("Keyboard Shortcuts") {
                    NotificationCenter.default.post(name: AppDelegate.showShortcutsNotification, object: nil)
                }
                .keyboardShortcut("/", modifiers: .command)

                Divider()

                Button("Zoom In") {
                    NotificationCenter.default.post(name: AppDelegate.zoomInNotification, object: nil)
                }
                .keyboardShortcut("=", modifiers: .command)

                Button("Zoom Out") {
                    NotificationCenter.default.post(name: AppDelegate.zoomOutNotification, object: nil)
                }
                .keyboardShortcut("-", modifiers: .command)

                Button("Reset Zoom") {
                    NotificationCenter.default.post(name: AppDelegate.resetZoomNotification, object: nil)
                }
                .keyboardShortcut("0", modifiers: .command)
            }

            CommandMenu("Tabs") {
                Button("Close Tab") {
                    NotificationCenter.default.post(name: AppDelegate.closeTabNotification, object: nil)
                }
                .keyboardShortcut("w", modifiers: .command)

                Button("Next Tab") {
                    NotificationCenter.default.post(name: AppDelegate.nextTabNotification, object: nil)
                }
                .keyboardShortcut("]", modifiers: [.command, .shift])

                Button("Previous Tab") {
                    NotificationCenter.default.post(name: AppDelegate.previousTabNotification, object: nil)
                }
                .keyboardShortcut("[", modifiers: [.command, .shift])
            }
        }

        Settings {
            SettingsView(showsHeader: false)
                .environmentObject(store)
                .frame(width: 820)
                .frame(minHeight: 620)
        }
    }

}

import AppKit
import Darwin
import Foundation
import MdowNativeCore
import UniformTypeIdentifiers

enum SidebarMode: String, CaseIterable, Identifiable {
    case recents
    case folder
    case outline

    var id: String { rawValue }

    var title: String {
        switch self {
        case .recents: "Recents"
        case .folder: "Folder"
        case .outline: "Outline"
        }
    }

    var systemImage: String {
        switch self {
        case .recents: "clock"
        case .folder: "folder"
        case .outline: "list.bullet"
        }
    }
}

enum FolderScanState: Equatable {
    case none
    case scanning
    case loaded
    case empty
    case truncated(limit: Int)
    case failed
}

@MainActor
final class DocumentStore: ObservableObject {
    static let shared = DocumentStore()

    @Published private(set) var openDocuments: [MarkdownDocument] = []
    @Published private(set) var activeURL: URL? {
        didSet {
            guard activeURL != oldValue else { return }
            resetSearchNavigation()
        }
    }
    @Published private(set) var blocksByURL: [URL: [MarkdownBlock]] = [:]
    @Published private(set) var recents: [MarkdownFileSummary] = []
    @Published private(set) var folderURL: URL?
    @Published private(set) var folderFiles: [MarkdownFileSummary] = []
    @Published private(set) var folderScanState: FolderScanState = .none
    @Published var fileError: MarkdownFileErrorModel?
    @Published var errorMessage: String?
    @Published var searchQuery = "" {
        didSet {
            guard searchQuery != oldValue else { return }
            searchNavigationQuery = ""
            searchNavigationPosition = -1
        }
    }
    @Published var searchOpen = false
    @Published var sidebarMode: SidebarMode = .recents {
        didSet {
            UserDefaults.standard.set(sidebarMode.rawValue, forKey: "MdowNativeSidebarMode")
        }
    }
    @Published var appTheme: AppTheme = .system {
        didSet {
            UserDefaults.standard.set(appTheme.rawValue, forKey: "MdowNativeAppTheme")
            applyAppTheme()
        }
    }
    @Published var contentFont: ContentFontPreset = .inter {
        didSet {
            UserDefaults.standard.set(contentFont.rawValue, forKey: "MdowNativeContentFont")
        }
    }
    @Published var codeFont: CodeFontPreset = .geistMono {
        didSet {
            UserDefaults.standard.set(codeFont.rawValue, forKey: "MdowNativeCodeFont")
        }
    }
    @Published private(set) var sidebarOpen = true
    @Published private(set) var zoomLevel = 100
    @Published var readingWidth: ReadingWidth = .standard {
        didSet {
            UserDefaults.standard.set(readingWidth.rawValue, forKey: "MdowNativeReadingWidth")
        }
    }
    @Published var interfaceScale: InterfaceScale = .compact {
        didSet {
            UserDefaults.standard.set(interfaceScale.rawValue, forKey: "MdowNativeInterfaceScale")
        }
    }
    @Published private(set) var wideMode = false

    private var didOpenLaunchArgument = false
    private var searchNavigationQuery = ""
    @Published private var searchNavigationPosition = -1
    private var openURLObserver: NSObjectProtocol?
    private var folderScanTask: Task<Void, Never>?
    private var watchedFileDescriptor: CInt = -1
    private var watchedFileSource: DispatchSourceFileSystemObject?

    var document: MarkdownDocument? {
        guard let activeURL else { return nil }
        return openDocuments.first { $0.url == activeURL }
    }

    var blocks: [MarkdownBlock] {
        guard let activeURL else { return [] }
        return blocksByURL[activeURL] ?? []
    }

    var outline: [MarkdownOutlineItem] {
        MarkdownParser.outline(blocks)
    }

    var searchMatchCount: Int {
        searchTargets.count
    }

    var searchCurrentIndex: Int? {
        MarkdownSearchNavigation.currentIndex(
            position: searchNavigationPosition,
            targetCount: searchTargets.count
        )
    }

    var activeSearchTarget: MarkdownSearchNavigationTarget? {
        let targets = searchTargets
        guard let searchCurrentIndex,
              targets.indices.contains(searchCurrentIndex) else {
            return nil
        }
        return targets[searchCurrentIndex]
    }

    var searchCounterText: String {
        MarkdownSearchNavigation.counterText(
            query: searchQuery,
            targetCount: searchTargets.count,
            currentIndex: searchCurrentIndex
        )
    }

    var searchableBlockIndices: [Int] {
        Array(Set(searchTargets.map(\.blockIndex))).sorted()
    }

    private var searchTargets: [MarkdownSearchNavigationTarget] {
        guard !searchQuery.isEmpty else { return [] }
        return MarkdownSearchNavigation.targets(
            in: blocks.map(Self.searchableText(for:)),
            query: searchQuery
        )
    }

    init() {
        recents = Self.loadRecents()
        appTheme = Self.loadAppTheme()
        contentFont = Self.loadContentFont()
        codeFont = Self.loadCodeFont()
        sidebarMode = Self.loadSidebarMode()
        sidebarOpen = Self.loadSidebarOpen()
        zoomLevel = Self.loadZoomLevel()
        readingWidth = Self.loadReadingWidth()
        interfaceScale = Self.loadInterfaceScale()
        wideMode = Self.loadWideMode()
        applyAppTheme()
        openURLObserver = NotificationCenter.default.addObserver(
            forName: AppDelegate.openURLsNotification,
            object: nil,
            queue: .main
        ) { [weak self] notification in
            guard let urls = notification.object as? [URL] else { return }
            Task { @MainActor in
                urls.forEach { self?.routeOpenURL($0) }
            }
        }

        for pendingURL in AppDelegate.markOpenURLObserverReady() {
            routeOpenURL(pendingURL)
        }

        openLaunchArgumentIfPresent()
    }

    func openWithPanel() {
        let panel = NSOpenPanel()
        panel.allowsMultipleSelection = false
        panel.canChooseDirectories = false
        panel.canChooseFiles = true
        panel.allowedContentTypes = Self.supportedContentTypes
        panel.message = "Choose a Markdown file to read."

        guard panel.runModal() == .OK, let url = panel.url else { return }
        open(url)
    }

    func openFolderWithPanel() {
        let panel = NSOpenPanel()
        panel.allowsMultipleSelection = false
        panel.canChooseDirectories = true
        panel.canChooseFiles = false
        panel.message = "Choose a folder of Markdown files."

        guard panel.runModal() == .OK, let url = panel.url else { return }
        openFolder(url)
    }

    func openFolder(_ url: URL) {
        folderScanTask?.cancel()
        folderURL = url
        folderFiles = []
        folderScanState = .scanning
        sidebarMode = .folder
        UserDefaults.standard.set(url.path, forKey: "MdowNativeLastFolder")
        errorMessage = nil

        folderScanTask = Task { [weak self] in
            do {
                let files = try await Task.detached(priority: .userInitiated) {
                    try MarkdownFolder.scanWithMetadata(url)
                }.value

                guard !Task.isCancelled else { return }
                self?.finishFolderScan(url: url, result: files)
            } catch {
                guard !Task.isCancelled else { return }
                self?.finishFolderScanFailure(url: url)
            }
        }
    }

    func openLaunchArgumentIfPresent(arguments: [String] = ProcessInfo.processInfo.arguments) {
        guard !didOpenLaunchArgument else { return }
        didOpenLaunchArgument = true

        let argumentPaths = arguments.dropFirst()
            .filter { !$0.hasPrefix("-") }

        let urls = argumentPaths
            .map { URL(fileURLWithPath: $0) }

        urls.forEach(routeOpenURL)
    }

    func routeOpenURL(_ url: URL) {
        switch MarkdownOpenURLRouting.target(for: url) {
        case .folder(let folderURL):
            openFolder(folderURL)
        case .markdownFile(let fileURL):
            open(fileURL)
        case .unsupportedFile(let fileURL):
            open(fileURL)
        case .ignored:
            break
        }
    }

    func open(_ url: URL) {
        do {
            let previousActiveURL = activeURL
            let loadedDocument = try MarkdownFile.load(url)
            if let existingIndex = openDocuments.firstIndex(where: { $0.url == loadedDocument.url }) {
                openDocuments[existingIndex] = loadedDocument
            } else {
                openDocuments.append(loadedDocument)
            }
            activeURL = loadedDocument.url
            blocksByURL[loadedDocument.url] = MarkdownParser.parse(loadedDocument.content)
            if previousActiveURL == loadedDocument.url {
                resetSearchNavigation()
            }
            addRecent(loadedDocument.url)
            startWatching(loadedDocument.url)
            fileError = nil
            errorMessage = nil
        } catch {
            restoreActiveDocumentAfterOpenFailure()
            fileError = MarkdownFileErrorModel(url: url, error: error)
            errorMessage = nil
        }
    }

    func retryFileError() {
        guard let fileError else { return }
        open(fileError.url)
    }

    func closeFileError() {
        if let url = fileError?.url {
            openDocuments.removeAll { $0.url == url }
            blocksByURL.removeValue(forKey: url)
        }
        fileError = nil
        errorMessage = nil
        activeURL = openDocuments.last?.url
        if let activeURL {
            startWatching(activeURL)
        } else {
            stopWatching()
        }
    }

    func revealFileErrorInFinder() {
        guard let fileError else { return }
        if FileManager.default.fileExists(atPath: fileError.url.path) {
            revealInFinder(fileError.url)
            return
        }

        NSWorkspace.shared.activateFileViewerSelecting([fileError.url.deletingLastPathComponent()])
    }

    func activate(_ url: URL) {
        guard openDocuments.contains(where: { $0.url == url }) else {
            open(url)
            return
        }

        activeURL = url
        startWatching(url)
    }

    func close(_ url: URL) {
        openDocuments.removeAll { $0.url == url }
        blocksByURL.removeValue(forKey: url)

        if activeURL == url {
            activeURL = openDocuments.last?.url
            if let activeURL {
                startWatching(activeURL)
            } else {
                stopWatching()
            }
        }
    }

    func closeOtherDocuments(keeping url: URL) {
        let keptURLs = Set(DocumentTabOperations.closeOthers(
            in: openDocuments.map(\.url),
            keeping: url
        ))
        guard !keptURLs.isEmpty else { return }

        openDocuments.removeAll { !keptURLs.contains($0.url) }
        blocksByURL = blocksByURL.filter { keptURLs.contains($0.key) }
        activeURL = url
        startWatching(url)
    }

    func closeDocumentsToRight(after url: URL) {
        let keptURLs = Set(DocumentTabOperations.closeToRight(
            in: openDocuments.map(\.url),
            after: url
        ))
        guard !keptURLs.isEmpty else { return }

        openDocuments.removeAll { !keptURLs.contains($0.url) }
        blocksByURL = blocksByURL.filter { keptURLs.contains($0.key) }

        if let activeURL, keptURLs.contains(activeURL) {
            startWatching(activeURL)
        } else {
            activeURL = url
            startWatching(url)
        }
    }

    func closeAllDocuments() {
        openDocuments = []
        blocksByURL = [:]
        activeURL = nil
        stopWatching()
    }

    func closeActiveDocument() {
        guard let activeURL else { return }
        close(activeURL)
    }

    func activateNextDocument() {
        activateDocument(offset: 1)
    }

    func activatePreviousDocument() {
        activateDocument(offset: -1)
    }

    func copyPath(_ url: URL) {
        NSPasteboard.general.clearContents()
        NSPasteboard.general.setString(url.path, forType: .string)
    }

    func revealInFinder(_ url: URL) {
        NSWorkspace.shared.activateFileViewerSelecting([url])
    }

    func removeRecent(_ url: URL) {
        recents = RecentFileList.removing(url, from: recents)
        UserDefaults.standard.set(recents.map(\.url.path), forKey: "MdowNativeRecents")
    }

    func nextSearchResultBlock() -> Int? {
        searchResultBlock(offset: 1)
    }

    func previousSearchResultBlock() -> Int? {
        searchResultBlock(offset: -1)
    }

    func toggleSidebar() {
        applyChromePreferences(ReaderChromePreferences(
            appTheme: appTheme,
            contentFont: contentFont,
            codeFont: codeFont,
            sidebarOpen: sidebarOpen,
            zoomLevel: zoomLevel,
            readingWidth: readingWidth,
            interfaceScale: interfaceScale,
            wideMode: wideMode
        ).toggledSidebar())
    }

    func zoomIn() {
        applyChromePreferences(ReaderChromePreferences(
            appTheme: appTheme,
            contentFont: contentFont,
            codeFont: codeFont,
            sidebarOpen: sidebarOpen,
            zoomLevel: zoomLevel,
            readingWidth: readingWidth,
            interfaceScale: interfaceScale,
            wideMode: wideMode
        ).zoomedIn())
    }

    func zoomOut() {
        applyChromePreferences(ReaderChromePreferences(
            appTheme: appTheme,
            contentFont: contentFont,
            codeFont: codeFont,
            sidebarOpen: sidebarOpen,
            zoomLevel: zoomLevel,
            readingWidth: readingWidth,
            interfaceScale: interfaceScale,
            wideMode: wideMode
        ).zoomedOut())
    }

    func resetZoom() {
        applyChromePreferences(ReaderChromePreferences(
            appTheme: appTheme,
            contentFont: contentFont,
            codeFont: codeFont,
            sidebarOpen: sidebarOpen,
            zoomLevel: zoomLevel,
            readingWidth: readingWidth,
            interfaceScale: interfaceScale,
            wideMode: wideMode
        ).resetZoom())
    }

    func toggleWideMode() {
        applyChromePreferences(ReaderChromePreferences(
            appTheme: appTheme,
            contentFont: contentFont,
            codeFont: codeFont,
            sidebarOpen: sidebarOpen,
            zoomLevel: zoomLevel,
            readingWidth: readingWidth,
            interfaceScale: interfaceScale,
            wideMode: wideMode
        ).toggledWideMode())
    }

    private static var supportedContentTypes: [UTType] {
        ["md", "markdown", "mdx"].compactMap { UTType(filenameExtension: $0) }
    }

    private static func message(for error: Error) -> String {
        if let localized = (error as? LocalizedError)?.errorDescription {
            return localized
        }
        return "Could not open this Markdown file."
    }

    private static func searchableText(for block: MarkdownBlock) -> String {
        switch block {
        case .heading(_, let text),
             .paragraph(let text),
             .unorderedListItem(let text),
             .orderedListItem(_, let text),
             .taskListItem(_, let text),
             .blockquote(let text):
            text
        case .callout(_, let title, let text):
            "\(title) \(text)"
        case .footnoteDefinition(let label, let text):
            "\(label) \(text)"
        case .horizontalRule:
            ""
        case .codeBlock(let language, let code):
            [language, code].compactMap { $0 }.joined(separator: "\n")
        case .mermaidDiagram(let diagram):
            ([diagram.source] + diagram.nodes.map(\.label)).joined(separator: " ")
        case .mathBlock(let formula):
            formula
        case .table(let headers, let rows):
            (headers + rows.flatMap { $0 }).joined(separator: " ")
        case .image(let alt, let source):
            "\(alt) \(source)"
        }
    }

    private func searchResultBlock(offset: Int) -> Int? {
        let targets = searchTargets
        guard !targets.isEmpty else { return nil }

        let queryChanged = searchNavigationQuery != searchQuery
        if queryChanged {
            searchNavigationQuery = searchQuery
        }

        guard let nextPosition = MarkdownSearchNavigation.advancedPosition(
            currentPosition: searchNavigationPosition,
            targetCount: targets.count,
            offset: offset,
            queryChanged: queryChanged
        ) else {
            return nil
        }

        searchNavigationPosition = nextPosition
        return targets[searchNavigationPosition].blockIndex
    }

    private func resetSearchNavigation() {
        searchNavigationQuery = ""
        searchNavigationPosition = -1
    }

    private func finishFolderScan(url: URL, result: MarkdownFolderScanResult) {
        guard folderURL == url else { return }
        folderFiles = result.files
        if result.didReachLimit {
            folderScanState = .truncated(limit: result.maxScannedEntries)
        } else if result.files.isEmpty {
            folderScanState = .empty
        } else {
            folderScanState = .loaded
        }
        errorMessage = nil
    }

    private func finishFolderScanFailure(url: URL) {
        guard folderURL == url else { return }
        folderFiles = []
        folderScanState = .failed
        errorMessage = "Could not scan this folder."
    }

    private func addRecent(_ url: URL) {
        recents = RecentFileList.prepending(url, to: recents, limit: 20)
        UserDefaults.standard.set(recents.map(\.url.path), forKey: "MdowNativeRecents")
    }

    private static func loadRecents() -> [MarkdownFileSummary] {
        let paths = UserDefaults.standard.stringArray(forKey: "MdowNativeRecents") ?? []
        let recents = paths
            .map { URL(fileURLWithPath: $0) }
            .filter { url in
                MarkdownFile.isSupported(url)
                    && !url.lastPathComponent.hasPrefix(".mdow-native-verify.")
                    && FileManager.default.fileExists(atPath: url.path)
            }
            .map { MarkdownFileSummary(url: $0) }
        let dedupedRecents = RecentFileList.deduped(recents)
        UserDefaults.standard.set(dedupedRecents.map(\.url.path), forKey: "MdowNativeRecents")
        return dedupedRecents
    }

    private static func loadSidebarOpen() -> Bool {
        guard UserDefaults.standard.object(forKey: "MdowNativeSidebarOpen") != nil else {
            return true
        }

        return UserDefaults.standard.bool(forKey: "MdowNativeSidebarOpen")
    }

    private static func loadAppTheme() -> AppTheme {
        guard let rawValue = UserDefaults.standard.string(forKey: "MdowNativeAppTheme") else {
            return .system
        }

        return AppTheme(rawValue: rawValue) ?? .system
    }

    private static func loadContentFont() -> ContentFontPreset {
        guard let rawValue = UserDefaults.standard.string(forKey: "MdowNativeContentFont") else {
            return .inter
        }

        return ContentFontPreset(rawValue: rawValue) ?? .inter
    }

    private static func loadCodeFont() -> CodeFontPreset {
        guard let rawValue = UserDefaults.standard.string(forKey: "MdowNativeCodeFont") else {
            return .geistMono
        }

        return CodeFontPreset(rawValue: rawValue) ?? .geistMono
    }

    private static func loadSidebarMode() -> SidebarMode {
        guard let rawValue = UserDefaults.standard.string(forKey: "MdowNativeSidebarMode") else {
            return .recents
        }

        return SidebarMode(rawValue: rawValue) ?? .recents
    }

    private static func loadZoomLevel() -> Int {
        guard UserDefaults.standard.object(forKey: "MdowNativeZoomLevel") != nil else {
            return 100
        }

        return ReaderChromePreferences(
            zoomLevel: UserDefaults.standard.integer(forKey: "MdowNativeZoomLevel")
        ).zoomLevel
    }

    private static func loadReadingWidth() -> ReadingWidth {
        guard let rawValue = UserDefaults.standard.string(forKey: "MdowNativeReadingWidth") else {
            return .standard
        }

        return ReadingWidth(rawValue: rawValue) ?? .standard
    }

    private static func loadInterfaceScale() -> InterfaceScale {
        guard let rawValue = UserDefaults.standard.string(forKey: "MdowNativeInterfaceScale") else {
            return .compact
        }

        return InterfaceScale(rawValue: rawValue) ?? .compact
    }

    private static func loadWideMode() -> Bool {
        UserDefaults.standard.bool(forKey: "MdowNativeWideMode")
    }

    private func applyChromePreferences(_ preferences: ReaderChromePreferences) {
        appTheme = preferences.appTheme
        contentFont = preferences.contentFont
        codeFont = preferences.codeFont
        sidebarOpen = preferences.sidebarOpen
        zoomLevel = preferences.zoomLevel
        readingWidth = preferences.readingWidth
        interfaceScale = preferences.interfaceScale
        wideMode = preferences.wideMode
        UserDefaults.standard.set(appTheme.rawValue, forKey: "MdowNativeAppTheme")
        UserDefaults.standard.set(contentFont.rawValue, forKey: "MdowNativeContentFont")
        UserDefaults.standard.set(codeFont.rawValue, forKey: "MdowNativeCodeFont")
        UserDefaults.standard.set(sidebarOpen, forKey: "MdowNativeSidebarOpen")
        UserDefaults.standard.set(zoomLevel, forKey: "MdowNativeZoomLevel")
        UserDefaults.standard.set(readingWidth.rawValue, forKey: "MdowNativeReadingWidth")
        UserDefaults.standard.set(interfaceScale.rawValue, forKey: "MdowNativeInterfaceScale")
        UserDefaults.standard.set(wideMode, forKey: "MdowNativeWideMode")
    }

    private func activateDocument(offset: Int) {
        guard let activeURL,
              let activeIndex = openDocuments.firstIndex(where: { $0.url == activeURL }),
              !openDocuments.isEmpty else {
            activeURL = openDocuments.first?.url
            if let activeURL {
                startWatching(activeURL)
            }
            return
        }

        let nextIndex = (activeIndex + offset + openDocuments.count) % openDocuments.count
        let nextURL = openDocuments[nextIndex].url
        self.activeURL = nextURL
        startWatching(nextURL)
    }

    private func restoreActiveDocumentAfterOpenFailure() {
        if let activeURL,
           openDocuments.contains(where: { $0.url == activeURL }) {
            startWatching(activeURL)
            return
        }

        activeURL = openDocuments.last?.url
        if let activeURL {
            startWatching(activeURL)
        } else {
            stopWatching()
        }
    }

    private func applyAppTheme() {
        switch appTheme {
        case .system:
            NSApp.appearance = nil
        case .light:
            NSApp.appearance = NSAppearance(named: .aqua)
        case .dark:
            NSApp.appearance = NSAppearance(named: .darkAqua)
        }
    }

    private func startWatching(_ url: URL) {
        stopWatching()

        let descriptor = Darwin.open(url.path, O_EVTONLY)
        guard descriptor >= 0 else { return }

        watchedFileDescriptor = descriptor
        let source = DispatchSource.makeFileSystemObjectSource(
            fileDescriptor: descriptor,
            eventMask: [.write, .delete, .rename],
            queue: .main
        )
        source.setEventHandler { [weak self] in
            guard let self else { return }
            self.open(url)
        }
        source.setCancelHandler {
            Darwin.close(descriptor)
        }
        watchedFileSource = source
        source.resume()
    }

    private func stopWatching() {
        watchedFileSource?.cancel()
        watchedFileSource = nil
        watchedFileDescriptor = -1
    }
}

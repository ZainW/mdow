import AppKit
import MdowNativeCore
import SwiftUI
import UniformTypeIdentifiers

struct ContentView: View {
    @EnvironmentObject private var store: DocumentStore
    @State private var isDropTargeted = false
    @State private var scrollTarget: Int?
    @State private var isCommandPalettePresented = false
    @State private var isSettingsPresented = false
    @State private var isShortcutsPresented = false
    @State private var expandedFolderPaths: Set<String> = []
    @State private var isSearchFocused = false

    var body: some View {
        ZStack {
            MdowStyle.background
                .ignoresSafeArea()

            VStack(spacing: 0) {
                titlebarInset
                HStack(spacing: 0) {
                    if store.sidebarOpen {
                        sidebar
                        Rectangle()
                            .fill(MdowStyle.borderSubtle.opacity(0.72))
                            .frame(width: 1)
                    }
                    mainColumn
                }
                .frame(maxWidth: .infinity, maxHeight: .infinity)
                .animation(.easeInOut(duration: 0.16), value: store.sidebarOpen)
            }
        }
        .ignoresSafeArea(.container, edges: .top)
        .font(.system(size: CGFloat(store.interfaceScale.controlFontSize)))
        .foregroundStyle(MdowStyle.foreground)
        .background(
            WindowChromeConfigurator(
                title: store.document?.title ?? "Mdow",
                representedURL: store.document?.url
            )
        )
        .onDrop(
            of: [UTType.fileURL.identifier],
            isTargeted: $isDropTargeted,
            perform: openDroppedFile
        )
        .overlay {
            if isDropTargeted {
                RoundedRectangle(cornerRadius: 10)
                    .stroke(MdowStyle.primary, lineWidth: 2)
                    .padding(12)
                    .allowsHitTesting(false)
            }
        }
        .onReceive(NotificationCenter.default.publisher(for: AppDelegate.focusSearchNotification)) { _ in
            guard store.document != nil else { return }
            isSettingsPresented = false
            store.searchOpen = true
            isSearchFocused = true
        }
        .onReceive(NotificationCenter.default.publisher(for: AppDelegate.nextSearchResultNotification)) { _ in
            if store.searchOpen {
                isSettingsPresented = false
            }
            scrollTarget = store.nextSearchResultBlock()
        }
        .onReceive(NotificationCenter.default.publisher(for: AppDelegate.previousSearchResultNotification)) { _ in
            if store.searchOpen {
                isSettingsPresented = false
            }
            scrollTarget = store.previousSearchResultBlock()
        }
        .onReceive(NotificationCenter.default.publisher(for: AppDelegate.showCommandPaletteNotification)) { _ in
            isCommandPalettePresented = true
        }
        .onReceive(NotificationCenter.default.publisher(for: AppDelegate.showSettingsNotification)) { _ in
            isSettingsPresented = true
        }
        .onReceive(NotificationCenter.default.publisher(for: AppDelegate.showShortcutsNotification)) { _ in
            isShortcutsPresented = true
        }
        .onReceive(NotificationCenter.default.publisher(for: AppDelegate.toggleSidebarNotification)) { _ in
            store.toggleSidebar()
        }
        .onReceive(NotificationCenter.default.publisher(for: AppDelegate.zoomInNotification)) { _ in
            store.zoomIn()
        }
        .onReceive(NotificationCenter.default.publisher(for: AppDelegate.zoomOutNotification)) { _ in
            store.zoomOut()
        }
        .onReceive(NotificationCenter.default.publisher(for: AppDelegate.resetZoomNotification)) { _ in
            store.resetZoom()
        }
        .onChange(of: store.folderURL) { _, _ in
            expandedFolderPaths = defaultExpandedFolderPaths()
        }
        .onChange(of: store.activeURL) { _, _ in
            expandActiveDocumentAncestors()
        }
        .sheet(isPresented: $isCommandPalettePresented) {
            CommandPaletteView(isPresented: $isCommandPalettePresented) {
                isSettingsPresented = false
            }
                .environmentObject(store)
        }
        .sheet(isPresented: $isShortcutsPresented) {
            ShortcutsView(isPresented: $isShortcutsPresented)
        }
    }

    private var titlebarInset: some View {
        ZStack {
            MdowStyle.background
            TitlebarDragRegion()
            HStack {
                Button {
                    store.toggleSidebar()
                } label: {
                    Image(systemName: "sidebar.left")
                        .font(.system(size: 13, weight: .medium))
                        .frame(width: 28, height: 24)
                }
                .buttonStyle(ChromeIconButtonStyle())
                .help(store.sidebarOpen ? "Hide Sidebar" : "Show Sidebar")
                .accessibilityLabel(store.sidebarOpen ? "Hide Sidebar" : "Show Sidebar")

                Spacer()
            }
            .padding(.leading, 72)
            .padding(.trailing, 10)
        }
        .frame(height: 34)
        .overlay(alignment: .bottom) {
            Rectangle().fill(MdowStyle.borderSubtle.opacity(0.14)).frame(height: 1)
        }
    }

    private var sidebar: some View {
        VStack(spacing: 0) {
            sidebarModeTabs
                .padding(.horizontal, 10)
                .padding(.top, 10)
                .padding(.bottom, 9)

            Rectangle()
                .fill(MdowStyle.borderSubtle.opacity(0.42))
                .frame(height: 1)

            ScrollView {
                VStack(alignment: .leading, spacing: 10) {
                    sidebarContent
                }
                .padding(.horizontal, 8)
                .padding(.vertical, 9)
            }

            Rectangle()
                .fill(MdowStyle.borderSubtle.opacity(0.42))
                .frame(height: 1)

            Button {
                isSettingsPresented = true
            } label: {
                Label("Settings", systemImage: "gearshape")
                    .frame(maxWidth: .infinity, alignment: .leading)
            }
            .buttonStyle(SidebarRowButtonStyle(
                isActive: isSettingsPresented,
                fontSize: CGFloat(store.interfaceScale.controlFontSize)
            ))
            .foregroundStyle(MdowStyle.mutedForeground)
            .padding(8)
            .accessibilityLabel("Settings")
            .accessibilityValue(isSettingsPresented ? "Selected" : "")
        }
        .frame(width: CGFloat(store.interfaceScale.sidebarWidth))
        .background(MdowStyle.sidebar)
    }

    private var sidebarModeTabs: some View {
        HStack(spacing: 5) {
            ForEach(SidebarMode.allCases) { mode in
                Button {
                    store.sidebarMode = mode
                } label: {
                    HStack(spacing: 5) {
                        Image(systemName: mode.systemImage)
                            .font(.system(size: 12, weight: .medium))
                        Text(mode.title)
                            .font(.system(size: CGFloat(store.interfaceScale.controlFontSize), weight: .medium))
                            .lineLimit(1)
                    }
                    .frame(maxWidth: .infinity)
                    .frame(height: CGFloat(store.interfaceScale.tabHeight + 1))
                }
                .buttonStyle(SegmentedModeButtonStyle(isSelected: store.sidebarMode == mode))
                .help(mode.title)
                .accessibilityLabel(mode.title)
                .accessibilityValue(store.sidebarMode == mode ? "Selected" : "")
            }
        }
        .padding(3)
        .background(MdowStyle.muted.opacity(0.34), in: RoundedRectangle(cornerRadius: 8))
        .overlay {
            RoundedRectangle(cornerRadius: 8)
                .stroke(MdowStyle.borderSubtle.opacity(0.46), lineWidth: 1)
        }
    }

    @ViewBuilder
    private var sidebarContent: some View {
        switch store.sidebarMode {
        case .recents:
            if store.recents.isEmpty {
                emptySidebarState(icon: "clock", title: "No recents", hint: "Open a document to build this list.")
            } else {
                sidebarSection("Recent")
                ForEach(store.recents) { fileRow($0, icon: "clock") }
            }

        case .folder:
            if store.folderFiles.isEmpty {
                emptySidebarState(icon: "folder", title: "No folder open", hint: "Open a folder or drop one here.")
            } else {
                folderSidebarHeader
                ForEach(folderTree(), id: \.id) { node in
                    FolderTreeRow(
                        node: node,
                        activeURL: store.activeURL,
                        interfaceScale: store.interfaceScale,
                        expandedFolderPaths: $expandedFolderPaths
                    ) { url in
                        isSettingsPresented = false
                        store.open(url)
                    } copyPath: { url in
                        store.copyPath(url)
                    } revealInFinder: { url in
                        store.revealInFinder(url)
                    }
                }
            }

        case .outline:
            if store.outline.isEmpty {
                emptySidebarState(icon: "list.bullet", title: store.document == nil ? "No document open" : "No headings", hint: "Open a document with headings.")
            } else {
                sidebarSection("Outline")
                ForEach(store.outline) { item in
                    Button {
                        scrollTarget = item.blockIndex
                    } label: {
                        Text(item.title)
                            .font(.system(size: CGFloat(store.interfaceScale.controlFontSize)))
                            .lineLimit(1)
                            .padding(.leading, CGFloat(max(0, item.level - 1)) * 10)
                            .frame(maxWidth: .infinity, alignment: .leading)
                    }
                    .buttonStyle(SidebarRowButtonStyle(
                        isActive: false,
                        fontSize: CGFloat(store.interfaceScale.controlFontSize)
                    ))
                    .accessibilityLabel("Jump to \(item.title)")
                }
            }
        }
    }

    private var mainColumn: some View {
        VStack(spacing: 0) {
            if !store.openDocuments.isEmpty && !isSettingsPresented {
                tabBar
            }

            if isSettingsPresented {
                SettingsView(isPresented: $isSettingsPresented)
                    .environmentObject(store)
            } else if let fileError = store.fileError {
                fileErrorView(fileError)
            } else if let document = store.document {
                breadcrumb(document)
                ZStack(alignment: .topTrailing) {
                    MarkdownDocumentView(
                        document: document,
                        blocks: store.blocks,
                        searchQuery: store.searchQuery,
                        scrollTarget: $scrollTarget,
                        isWide: store.wideMode,
                        zoomLevel: store.zoomLevel,
                        readingWidth: store.readingWidth,
                        contentFont: store.contentFont,
                        codeFont: store.codeFont
                    )
                    .environment(
                        \.openURL,
                        OpenURLAction { url in
                            handleMarkdownOpenURL(url, document: document)
                        }
                    )
                    if store.searchOpen {
                        readerSearchBar
                    }
                    zoomIndicator
                }
            } else {
                emptyState
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(MdowStyle.background)
    }

    private var tabBar: some View {
        ScrollView(.horizontal, showsIndicators: false) {
            HStack(spacing: 1) {
                ForEach(Array(store.openDocuments.enumerated()), id: \.element.url) { index, document in
                    HStack(spacing: 6) {
                        Button {
                            isSettingsPresented = false
                            store.activate(document.url)
                        } label: {
                            HStack(spacing: 6) {
                                Image(systemName: "doc.text")
                                    .font(.system(size: CGFloat(store.interfaceScale.controlFontSize + 1)))
                                    .foregroundStyle(MdowStyle.mutedForeground)
                                    .accessibilityHidden(true)
                                Text(document.title)
                                    .lineLimit(1)
                            }
                            .frame(maxWidth: CGFloat(store.interfaceScale == .compact ? 190 : store.interfaceScale == .comfortable ? 210 : 230))
                        }
                        .buttonStyle(.plain)
                        .help("Switch to \(document.title)")
                        .accessibilityLabel("Switch to \(document.title)")

                        Button {
                            store.close(document.url)
                        } label: {
                            Image(systemName: "xmark")
                                .font(.system(size: CGFloat(store.interfaceScale.controlFontSize - 2), weight: .bold))
                        }
                        .buttonStyle(.plain)
                        .foregroundStyle(MdowStyle.mutedForeground)
                        .help("Close \(document.title)")
                        .accessibilityLabel("Close \(document.title)")
                    }
                    .font(.system(size: CGFloat(store.interfaceScale.controlFontSize)))
                    .padding(.horizontal, 9)
                    .frame(height: CGFloat(store.interfaceScale.tabHeight))
                    .contextMenu {
                        tabContextMenu(for: document, index: index)
                    }
                    .background(
                        document.url == store.activeURL
                            ? MdowStyle.elevatedSurface
                            : Color.clear,
                        in: RoundedRectangle(cornerRadius: 6)
                    )
                    .overlay {
                        if document.url == store.activeURL {
                            RoundedRectangle(cornerRadius: 6)
                                .stroke(MdowStyle.borderSubtle.opacity(0.52), lineWidth: 1)
                        }
                    }
                    .shadow(color: .black.opacity(document.url == store.activeURL ? 0.018 : 0), radius: 2, y: 1)
                }
            }
            .padding(.horizontal, 7)
        }
        .frame(height: CGFloat(store.interfaceScale.tabBarHeight))
        .background(MdowStyle.background)
        .overlay(alignment: .bottom) {
            Rectangle().fill(MdowStyle.borderSubtle.opacity(0.22)).frame(height: 1)
        }
    }

    @ViewBuilder
    private func tabContextMenu(for document: MarkdownDocument, index: Int) -> some View {
        Button("Close") {
            store.close(document.url)
        }

        Button("Close Others") {
            store.closeOtherDocuments(keeping: document.url)
        }
        .disabled(store.openDocuments.count <= 1)

        Button("Close to the Right") {
            store.closeDocumentsToRight(after: document.url)
        }
        .disabled(index >= store.openDocuments.count - 1)

        Button("Close All") {
            store.closeAllDocuments()
        }

        Divider()

        Button("Copy Path") {
            store.copyPath(document.url)
        }

        Button("Reveal in Finder") {
            store.revealInFinder(document.url)
        }
    }

    private func breadcrumb(_ document: MarkdownDocument) -> some View {
        HStack(spacing: 4) {
            let secondaryFilename = DocumentDisplayTitle.secondaryFilename(
                documentTitle: document.title,
                fileURL: document.url
            )
            let segments = DocumentBreadcrumbSegments.parentSegments(
                for: document.url,
                rootURL: store.folderURL
            )
            ForEach(segments) { segment in
                Button {
                    store.revealInFinder(segment.url)
                } label: {
                    Text(segment.name)
                        .lineLimit(1)
                }
                .buttonStyle(BreadcrumbButtonStyle(
                    isCurrentDocument: false,
                    fontSize: CGFloat(store.interfaceScale.secondaryFontSize)
                ))
                .help("Reveal in Finder: \(segment.url.path)")

                Image(systemName: "chevron.right")
                    .font(.system(size: CGFloat(store.interfaceScale.secondaryFontSize - 2), weight: .semibold))
                    .foregroundStyle(MdowStyle.mutedForeground.opacity(0.55))
            }

            Button {
                store.revealInFinder(document.url)
            } label: {
                Text(document.title)
                    .lineLimit(1)
            }
            .buttonStyle(BreadcrumbButtonStyle(
                isCurrentDocument: true,
                fontSize: CGFloat(store.interfaceScale.secondaryFontSize)
            ))
            .help("Reveal in Finder: \(document.url.path)")

            if let secondaryFilename {
                Text(secondaryFilename)
                    .lineLimit(1)
                    .foregroundStyle(MdowStyle.mutedForeground.opacity(0.62))
                    .font(.system(size: CGFloat(max(9, store.interfaceScale.secondaryFontSize - 1))))
                    .padding(.leading, 2)
            }

            Spacer()

            Button {
                store.wideMode.toggle()
            } label: {
                Image(systemName: store.wideMode ? "arrow.left.and.right.righttriangle.left.righttriangle.right" : "arrow.left.and.right")
                    .font(.system(size: 12, weight: .medium))
            }
            .buttonStyle(.plain)
            .foregroundStyle(MdowStyle.mutedForeground)
            .help(store.wideMode ? "Constrained width" : "Full width")
            .accessibilityLabel(store.wideMode ? "Use constrained reading width" : "Use full reading width")
        }
        .font(.system(size: CGFloat(store.interfaceScale.secondaryFontSize)))
        .foregroundStyle(MdowStyle.mutedForeground.opacity(0.85))
        .padding(.horizontal, 14)
        .frame(height: CGFloat(store.interfaceScale.breadcrumbHeight))
        .background(MdowStyle.background)
        .overlay(alignment: .bottom) {
            Rectangle().fill(MdowStyle.borderSubtle.opacity(0.24)).frame(height: 1)
        }
    }

    private var readerSearchBar: some View {
        HStack(spacing: 10) {
            NativeSearchTextField(
                text: $store.searchQuery,
                isFocused: $isSearchFocused
            ) {
                scrollTarget = store.nextSearchResultBlock()
            } onPrevious: {
                scrollTarget = store.previousSearchResultBlock()
            } onClose: {
                closeSearch()
            }
                .frame(width: 170)

            Text(store.searchCounterText)
                .font(.system(size: 11))
                .monospacedDigit()
                .foregroundStyle(MdowStyle.mutedForeground)
                .frame(width: 64)

            Button {
                scrollTarget = store.previousSearchResultBlock()
            } label: {
                Image(systemName: "chevron.up")
                    .frame(width: 18, height: 18)
            }
            .buttonStyle(.plain)
            .foregroundStyle(store.searchMatchCount == 0 ? MdowStyle.mutedForeground.opacity(0.35) : MdowStyle.mutedForeground)
            .disabled(store.searchableBlockIndices.isEmpty)
            .help("Previous result")
            .accessibilityLabel("Previous search result")

            Button {
                scrollTarget = store.nextSearchResultBlock()
            } label: {
                Image(systemName: "chevron.down")
                    .frame(width: 18, height: 18)
            }
            .buttonStyle(.plain)
            .foregroundStyle(store.searchMatchCount == 0 ? MdowStyle.mutedForeground.opacity(0.35) : MdowStyle.mutedForeground)
            .disabled(store.searchableBlockIndices.isEmpty)
            .help("Next result")
            .accessibilityLabel("Next search result")

            Button {
                closeSearch()
            } label: {
                Image(systemName: "xmark")
                    .frame(width: 18, height: 18)
            }
            .buttonStyle(.plain)
            .foregroundStyle(MdowStyle.mutedForeground)
            .help("Close search")
            .accessibilityLabel("Close search")
        }
        .font(.system(size: CGFloat(store.interfaceScale.controlFontSize)))
        .padding(.horizontal, 12)
        .padding(.vertical, 8)
        .background(.regularMaterial, in: UnevenRoundedRectangle(bottomLeadingRadius: 10))
        .overlay {
            UnevenRoundedRectangle(bottomLeadingRadius: 10)
                .stroke(
                    isSearchFocused ? MdowStyle.primary.opacity(0.64) : MdowStyle.border.opacity(0.42),
                    lineWidth: isSearchFocused ? 1.5 : 1
                )
        }
        .shadow(color: .black.opacity(0.08), radius: 8, y: 3)
        .padding(.top, 0)
        .onExitCommand {
            closeSearch()
        }
    }

    private func closeSearch() {
        store.searchOpen = false
        store.searchQuery = ""
        isSearchFocused = false
    }

    private func handleMarkdownOpenURL(_ url: URL, document: MarkdownDocument) -> OpenURLAction.Result {
        let href = url.absoluteString
        switch MarkdownLinkResolver.resolve(href: href, documentURL: document.url) {
        case .anchor(let anchor):
            scrollToAnchor(anchor, in: store.blocks)
            return .handled

        case .markdownFile(let fileURL, let anchor):
            store.open(fileURL)
            if let anchor {
                DispatchQueue.main.async {
                    scrollToAnchor(anchor, in: store.blocks)
                }
            }
            return .handled

        case .external(let externalURL):
            NSWorkspace.shared.open(externalURL)
            return .handled

        case .ignored:
            return .discarded
        }
    }

    private func scrollToAnchor(_ anchor: String, in blocks: [MarkdownBlock]) {
        guard let index = MarkdownAnchorResolver.blockIndex(for: anchor, in: blocks) else {
            return
        }

        scrollTarget = index
    }

    private var zoomIndicator: some View {
        HStack(spacing: 6) {
            Button {
                store.zoomOut()
            } label: {
                Image(systemName: "minus")
                    .frame(width: 18, height: 18)
            }
            .disabled(store.zoomLevel <= 60)
            .help("Zoom out")
            .accessibilityLabel("Zoom out")

            Text("\(store.zoomLevel)%")
                .font(.system(size: 11))
                .monospacedDigit()
                .frame(width: 44)

            Button {
                store.zoomIn()
            } label: {
                Image(systemName: "plus")
                    .frame(width: 18, height: 18)
            }
            .disabled(store.zoomLevel >= 200)
            .help("Zoom in")
            .accessibilityLabel("Zoom in")

            Button {
                store.resetZoom()
            } label: {
                Image(systemName: "arrow.counterclockwise")
                    .frame(width: 18, height: 18)
            }
            .disabled(store.zoomLevel == 100)
            .help("Reset zoom")
            .accessibilityLabel("Reset zoom")
        }
        .buttonStyle(.plain)
        .font(.system(size: 11, weight: .medium))
        .foregroundStyle(MdowStyle.foreground)
        .padding(.horizontal, 9)
        .padding(.vertical, 7)
        .background(.regularMaterial, in: RoundedRectangle(cornerRadius: 9))
        .overlay {
            RoundedRectangle(cornerRadius: 9)
                .stroke(MdowStyle.border.opacity(0.44), lineWidth: 1)
        }
        .shadow(color: .black.opacity(0.075), radius: 8, y: 2)
        .padding(14)
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .bottomTrailing)
        .opacity(store.zoomLevel == 100 ? 0 : 1)
        .allowsHitTesting(store.zoomLevel != 100)
        .animation(.easeInOut(duration: 0.14), value: store.zoomLevel)
    }

    private var emptyState: some View {
        HStack(alignment: .top, spacing: store.recents.isEmpty ? 0 : 44) {
            welcomeIntro

            if !store.recents.isEmpty {
                welcomeRecentList
            }
        }
        .padding(.horizontal, 52)
        .padding(.vertical, 42)
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(isDropTargeted ? MdowStyle.primary.opacity(0.035) : Color.clear)
        .animation(.easeInOut(duration: 0.15), value: isDropTargeted)
    }

    private var welcomeIntro: some View {
        VStack(alignment: store.recents.isEmpty ? .center : .leading, spacing: 13) {
            RoundedRectangle(cornerRadius: 12)
                .fill(MdowStyle.primary)
                .frame(width: 48, height: 48)
                .overlay {
                    Text("M")
                        .font(.system(size: 24, weight: .bold))
                        .foregroundStyle(.white)
                }
                .shadow(color: .black.opacity(0.08), radius: 4, y: 1)
                .padding(.bottom, store.recents.isEmpty ? 2 : 0)

            Text("Mdow")
                .font(.system(size: 24, weight: .semibold))

            Text("A quiet markdown viewer. Drop a file anywhere, or open one below.")
                .font(.system(size: 14))
                .foregroundStyle(MdowStyle.mutedForeground)
                .lineSpacing(3)
                .multilineTextAlignment(store.recents.isEmpty ? .center : .leading)
                .frame(maxWidth: 350, alignment: store.recents.isEmpty ? .center : .leading)

            HStack(spacing: 8) {
                Button { store.openWithPanel() } label: {
                    Label("Open File", systemImage: "doc")
                }
                Button { store.openFolderWithPanel() } label: {
                    Label("Open Folder", systemImage: "folder")
                }
            }
            .buttonStyle(.bordered)
            .padding(.top, 2)

            welcomeDropHint

            folderErrorMessage
        }
        .frame(maxWidth: store.recents.isEmpty ? 520 : 380, alignment: store.recents.isEmpty ? .center : .leading)
    }

    @ViewBuilder
    private var folderErrorMessage: some View {
        if let errorMessage = store.errorMessage {
            Text(errorMessage)
                .font(.system(size: 12))
                .foregroundStyle(.red)
                .multilineTextAlignment(store.recents.isEmpty ? .center : .leading)
                .frame(maxWidth: 350, alignment: store.recents.isEmpty ? .center : .leading)
        }
    }

    private func fileErrorView(_ error: MarkdownFileErrorModel) -> some View {
        VStack(spacing: 16) {
            Image(systemName: fileErrorSymbol(error.kind))
                .font(.system(size: 25, weight: .medium))
                .foregroundStyle(MdowStyle.mutedForeground)
                .frame(width: 48, height: 48)
                .background(MdowStyle.muted, in: Circle())

            VStack(spacing: 6) {
                Text(error.title)
                    .font(.system(size: 16, weight: .medium))
                    .foregroundStyle(MdowStyle.foreground)

                Text(error.body)
                    .font(.system(size: 13))
                    .foregroundStyle(MdowStyle.mutedForeground)
                    .multilineTextAlignment(.center)
                    .lineSpacing(2)
                    .frame(maxWidth: 380)
            }

            Text(truncatedPath(error.url.path))
                .font(.system(size: 12, design: .monospaced))
                .foregroundStyle(MdowStyle.mutedForeground.opacity(0.7))
                .lineLimit(1)
                .truncationMode(.middle)
                .help(error.url.path)
                .frame(maxWidth: 390)

            HStack(spacing: 8) {
                Button("Try again") {
                    store.retryFileError()
                }
                .buttonStyle(.bordered)

                if error.canRevealInFinder {
                    Button {
                        store.revealFileErrorInFinder()
                    } label: {
                        Label("Show in folder", systemImage: "folder")
                    }
                    .buttonStyle(.plain)
                    .foregroundStyle(MdowStyle.mutedForeground)
                }

                Button("Close tab") {
                    store.closeFileError()
                }
                .buttonStyle(.plain)
                .foregroundStyle(MdowStyle.mutedForeground)
            }
        }
        .padding(42)
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    private func fileErrorSymbol(_ kind: MarkdownFileErrorKind) -> String {
        switch kind {
        case .notFound:
            "doc.badge.questionmark"
        case .permissionDenied:
            "lock.shield"
        case .readError:
            "exclamationmark.triangle"
        }
    }

    private func truncatedPath(_ path: String) -> String {
        guard path.count > 54 else { return path }
        let prefix = path.prefix(24)
        let suffix = path.suffix(24)
        return "\(prefix)...\(suffix)"
    }

    private var welcomeDropHint: some View {
        HStack(spacing: 4) {
            Text("Anywhere in this window")
                .fontWeight(.medium)
                .foregroundStyle(MdowStyle.foreground.opacity(0.9))
            Text("— drop")
            Text(".md")
                .font(.system(size: 11, design: .monospaced))
                .padding(.horizontal, 5)
                .padding(.vertical, 2)
                .background(MdowStyle.muted, in: RoundedRectangle(cornerRadius: 4))
            Text("files or a folder.")
        }
        .font(.system(size: 12))
        .foregroundStyle(MdowStyle.mutedForeground)
        .padding(.horizontal, 13)
        .padding(.vertical, 10)
        .frame(maxWidth: 420, alignment: store.recents.isEmpty ? .center : .leading)
        .background(
            isDropTargeted ? MdowStyle.primary.opacity(0.08) : MdowStyle.muted.opacity(0.32),
            in: RoundedRectangle(cornerRadius: 8)
        )
        .overlay {
            RoundedRectangle(cornerRadius: 8)
                .stroke(
                    isDropTargeted ? MdowStyle.primary.opacity(0.45) : MdowStyle.border.opacity(0.8),
                    style: StrokeStyle(lineWidth: 1, dash: [4, 3])
                )
        }
        .padding(.top, 4)
    }

    private var welcomeRecentList: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text("RECENT")
                .font(.system(size: 11, weight: .medium, design: .monospaced))
                .tracking(1.8)
                .foregroundStyle(MdowStyle.mutedForeground.opacity(0.7))

            VStack(alignment: .leading, spacing: 1) {
                ForEach(WelcomeRecents.displayed(store.recents)) { file in
                    recentFileRow(file, icon: "doc.text")
                }
            }
        }
        .frame(width: 270, alignment: .topLeading)
    }

    private func sidebarSection(_ title: String) -> some View {
        Text(title.uppercased())
            .font(.system(size: CGFloat(store.interfaceScale.secondaryFontSize), weight: .medium, design: .monospaced))
            .tracking(1.7)
            .foregroundStyle(MdowStyle.mutedForeground.opacity(0.72))
            .padding(.horizontal, 6)
            .padding(.top, 4)
    }

    private var folderSidebarHeader: some View {
        HStack {
            Text("Folder")
                .font(.system(size: CGFloat(store.interfaceScale.secondaryFontSize)))
                .foregroundStyle(MdowStyle.mutedForeground)
            Spacer()
            Text(store.folderURL?.lastPathComponent ?? "")
                .font(.system(size: CGFloat(store.interfaceScale.secondaryFontSize)))
                .foregroundStyle(MdowStyle.mutedForeground.opacity(0.85))
                .lineLimit(1)
        }
        .padding(.horizontal, 6)
        .padding(.top, 2)
        .padding(.bottom, 4)
    }

    private func fileRow(_ file: MarkdownFileSummary, icon: String) -> some View {
        recentFileRow(file, icon: icon)
    }

    private func recentFileRow(_ file: MarkdownFileSummary, icon: String) -> some View {
        Button {
            isSettingsPresented = false
            store.open(file.url)
        } label: {
            Label(file.title, systemImage: icon)
                .lineLimit(1)
                .frame(maxWidth: .infinity, alignment: .leading)
        }
        .buttonStyle(SidebarRowButtonStyle(
            isActive: file.url == store.activeURL,
            fontSize: CGFloat(store.interfaceScale.controlFontSize)
        ))
        .contextMenu {
            recentFileContextMenu(file)
        }
        .help(file.url.path)
        .accessibilityLabel(file.title)
        .accessibilityValue(file.url == store.activeURL ? "Selected" : "")
    }

    @ViewBuilder
    private func recentFileContextMenu(_ file: MarkdownFileSummary) -> some View {
        Button("Open") {
            store.open(file.url)
        }

        Button("Copy Path") {
            store.copyPath(file.url)
        }

        Button("Reveal in Finder") {
            store.revealInFinder(file.url)
        }

        Divider()

        Button("Remove from Recents", role: .destructive) {
            store.removeRecent(file.url)
        }
    }

    private func emptySidebarState(icon: String, title: String, hint: String) -> some View {
        VStack(spacing: 8) {
            Image(systemName: icon)
                .font(.system(size: 24))
                .foregroundStyle(MdowStyle.mutedForeground.opacity(0.6))
            Text(title)
                .font(.system(size: 13, weight: .medium))
            Text(hint)
                .font(.system(size: 12))
                .foregroundStyle(MdowStyle.mutedForeground)
                .multilineTextAlignment(.center)
                .lineLimit(3)
        }
        .frame(maxWidth: .infinity)
        .padding(.horizontal, 14)
        .padding(.vertical, 28)
    }

    private func openDroppedFile(_ providers: [NSItemProvider]) -> Bool {
        guard let provider = providers.first(where: {
            $0.hasItemConformingToTypeIdentifier(UTType.fileURL.identifier)
        }) else {
            return false
        }

        provider.loadItem(forTypeIdentifier: UTType.fileURL.identifier, options: nil) { item, _ in
            let url = droppedURL(from: item)
            Task { @MainActor in
                if let url {
                    isSettingsPresented = false
                    if isDirectory(url) {
                        store.openFolder(url)
                    } else {
                        store.open(url)
                    }
                }
            }
        }

        return true
    }

    private func folderTree() -> [FolderTreeNode] {
        guard let folderURL = store.folderURL else { return [] }
        var buckets: [String: [MarkdownFileSummary]] = [:]

        for file in store.folderFiles {
            let directory = file.url.deletingLastPathComponent().standardizedFileURL.path
            buckets[directory, default: []].append(file)
        }

        func makeNodes(in directory: URL, depth: Int) -> [FolderTreeNode] {
            let childDirectories = childFolderURLs(in: directory, bucketPaths: Array(buckets.keys))
            let folders = childDirectories.map { child in
                FolderTreeNode(
                    url: child,
                    title: child.lastPathComponent,
                    depth: depth,
                    isFolder: true,
                    children: makeNodes(in: child, depth: depth + 1)
                )
            }

            let files = (buckets[directory.standardizedFileURL.path] ?? []).map { file in
                FolderTreeNode(
                    url: file.url,
                    title: file.title,
                    depth: depth,
                    isFolder: false,
                    children: []
                )
            }

            return folders + files.sorted {
                $0.title.localizedStandardCompare($1.title) == .orderedAscending
            }
        }

        return makeNodes(in: folderURL, depth: 0)
    }

    private func childFolderURLs(in directory: URL, bucketPaths: [String]) -> [URL] {
        let directoryPath = directory.standardizedFileURL.path
        let children = Set(bucketPaths.compactMap { bucketPath -> URL? in
            guard bucketPath != directoryPath,
                  bucketPath.hasPrefix(directoryPath + "/") else {
                return nil
            }

            let suffix = String(bucketPath.dropFirst(directoryPath.count + 1))
            guard let firstComponent = suffix.split(separator: "/").first else { return nil }
            return directory.appendingPathComponent(String(firstComponent))
        })

        return children.sorted {
            $0.lastPathComponent.localizedStandardCompare($1.lastPathComponent) == .orderedAscending
        }
    }

    private func defaultExpandedFolderPaths() -> Set<String> {
        guard let folderURL = store.folderURL else { return [] }
        var expanded = Set<String>()
        expanded.insert(folderURL.standardizedFileURL.path)

        let rootPath = folderURL.standardizedFileURL.path
        let directFolders = store.folderFiles.compactMap { file -> String? in
            let directory = file.url.deletingLastPathComponent().standardizedFileURL
            guard directory.path != rootPath,
                  directory.path.hasPrefix(rootPath + "/"),
                  directory.deletingLastPathComponent().path == rootPath else {
                return nil
            }
            return directory.path
        }

        expanded.formUnion(directFolders.sorted().prefix(12))
        if let activeURL = store.activeURL {
            expanded.formUnion(folderAncestorPaths(for: activeURL))
        }
        return expanded
    }

    private func expandActiveDocumentAncestors() {
        guard let activeURL = store.activeURL else { return }
        expandedFolderPaths.formUnion(folderAncestorPaths(for: activeURL))
    }

    private func folderAncestorPaths(for fileURL: URL) -> Set<String> {
        guard let folderURL = store.folderURL else { return [] }
        let rootPath = folderURL.standardizedFileURL.path
        var url = fileURL.deletingLastPathComponent().standardizedFileURL
        var paths: Set<String> = []

        while url.path.hasPrefix(rootPath) {
            paths.insert(url.path)
            if url.path == rootPath { break }
            url.deleteLastPathComponent()
        }

        return paths
    }
}

private func isDirectory(_ url: URL) -> Bool {
    (try? url.resourceValues(forKeys: [.isDirectoryKey]).isDirectory) == true
}

struct SegmentedModeButtonStyle: ButtonStyle {
    let isSelected: Bool

    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .foregroundStyle(isSelected ? MdowStyle.foreground : MdowStyle.mutedForeground)
            .background(
                isSelected
                    ? MdowStyle.elevatedSurface
                    : configuration.isPressed ? MdowStyle.sidebarHover : Color.clear,
                in: RoundedRectangle(cornerRadius: 6)
            )
            .contentShape(RoundedRectangle(cornerRadius: 6))
            .overlay {
                if isSelected {
                    RoundedRectangle(cornerRadius: 6)
                        .stroke(MdowStyle.borderSubtle.opacity(0.50), lineWidth: 1)
                }
            }
            .shadow(color: .black.opacity(isSelected ? 0.018 : 0), radius: 1, y: 1)
            .scaleEffect(configuration.isPressed ? 0.985 : 1)
    }
}

struct SidebarRowButtonStyle: ButtonStyle {
    let isActive: Bool
    var fontSize: CGFloat = 12

    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .font(.system(size: fontSize))
            .foregroundStyle(isActive ? MdowStyle.foreground : MdowStyle.mutedForeground)
            .padding(.horizontal, 7)
            .padding(.vertical, 5)
            .background(
                isActive ? MdowStyle.sidebarAccent : configuration.isPressed ? MdowStyle.sidebarHover : Color.clear,
                in: RoundedRectangle(cornerRadius: 6)
            )
            .contentShape(RoundedRectangle(cornerRadius: 6))
            .overlay(alignment: .leading) {
                if isActive {
                    RoundedRectangle(cornerRadius: 1)
                        .fill(MdowStyle.accent.opacity(0.78))
                        .frame(width: 2)
                        .padding(.vertical, 4)
                        .offset(x: -2)
                }
            }
    }
}

private struct BreadcrumbButtonStyle: ButtonStyle {
    let isCurrentDocument: Bool
    let fontSize: CGFloat

    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .font(.system(size: fontSize, weight: isCurrentDocument ? .medium : .regular))
            .foregroundStyle(isCurrentDocument ? MdowStyle.foreground.opacity(0.9) : MdowStyle.mutedForeground.opacity(0.85))
            .padding(.horizontal, isCurrentDocument ? 5 : 3)
            .padding(.vertical, 2)
            .background(
                configuration.isPressed ? MdowStyle.muted.opacity(0.75) : Color.clear,
                in: RoundedRectangle(cornerRadius: 4)
            )
            .contentShape(RoundedRectangle(cornerRadius: 4))
    }
}

private struct FolderTreeRowButtonStyle: ButtonStyle {
    let isActive: Bool
    let fontSize: CGFloat

    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .font(.system(size: fontSize))
            .foregroundStyle(isActive ? MdowStyle.foreground : MdowStyle.mutedForeground)
            .padding(.horizontal, 7)
            .padding(.vertical, 5)
            .background(
                isActive ? MdowStyle.sidebarAccent : configuration.isPressed ? MdowStyle.sidebarHover : Color.clear,
                in: RoundedRectangle(cornerRadius: 5)
            )
            .contentShape(RoundedRectangle(cornerRadius: 5))
    }
}

struct ChromeIconButtonStyle: ButtonStyle {
    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .foregroundStyle(MdowStyle.mutedForeground)
            .background(
                configuration.isPressed ? MdowStyle.sidebarHover : Color.clear,
                in: RoundedRectangle(cornerRadius: 6)
            )
            .contentShape(RoundedRectangle(cornerRadius: 6))
            .scaleEffect(configuration.isPressed ? 0.97 : 1)
    }
}

private struct FolderTreeRow: View {
    let node: FolderTreeNode
    let activeURL: URL?
    let interfaceScale: InterfaceScale
    @Binding var expandedFolderPaths: Set<String>
    let open: (URL) -> Void
    let copyPath: (URL) -> Void
    let revealInFinder: (URL) -> Void

    var body: some View {
        if node.isFolder {
            Button {
                toggleFolder()
            } label: {
                HStack(spacing: 5) {
                    Image(systemName: isExpanded ? "chevron.down" : "chevron.right")
                        .font(.system(size: 9, weight: .semibold))
                        .foregroundStyle(MdowStyle.mutedForeground.opacity(0.75))
                        .frame(width: 10)
                    Text(node.title)
                        .lineLimit(1)
                    Spacer(minLength: 0)
                }
                .padding(.leading, CGFloat(node.depth) * 11)
                .frame(maxWidth: .infinity, alignment: .leading)
            }
            .buttonStyle(FolderTreeRowButtonStyle(
                isActive: false,
                fontSize: CGFloat(interfaceScale.controlFontSize)
            ))
            .contextMenu {
                folderTreeContextMenu()
            }
            .help(node.url.path)
            .accessibilityLabel("\(node.title), folder")
            .accessibilityValue(isExpanded ? "Expanded" : "Collapsed")
            .accessibilityHint(isExpanded ? "Collapses this folder" : "Expands this folder")

            if isExpanded {
                ForEach(node.children, id: \.id) { child in
                    FolderTreeRow(
                        node: child,
                        activeURL: activeURL,
                        interfaceScale: interfaceScale,
                        expandedFolderPaths: $expandedFolderPaths,
                        open: open,
                        copyPath: copyPath,
                        revealInFinder: revealInFinder
                    )
                }
            }
        } else {
            Button {
                open(node.url)
            } label: {
                HStack(spacing: 6) {
                    Image(systemName: "doc.text")
                        .font(.system(size: 12))
                        .foregroundStyle(MdowStyle.mutedForeground)
                        .frame(width: 12)
                    Text(node.title)
                        .lineLimit(1)
                    Spacer(minLength: 0)
                }
                .padding(.leading, CGFloat(node.depth) * 11 + 15)
                .frame(maxWidth: .infinity, alignment: .leading)
            }
            .buttonStyle(FolderTreeRowButtonStyle(
                isActive: node.url == activeURL,
                fontSize: CGFloat(interfaceScale.controlFontSize)
            ))
            .contextMenu {
                folderTreeContextMenu()
            }
            .help(node.url.path)
            .accessibilityLabel(node.title)
            .accessibilityValue(node.url == activeURL ? "Selected" : "")
            .accessibilityHint("Opens this Markdown file")
        }
    }

    @ViewBuilder
    private func folderTreeContextMenu() -> some View {
        ForEach(FolderTreeContextActions.actions(isFolder: node.isFolder), id: \.rawValue) { action in
            switch action {
            case .open:
                Button(action.title) {
                    open(node.url)
                }
            case .copyPath:
                Button(action.title) {
                    copyPath(node.url)
                }
            case .revealInFinder:
                Button(action.title) {
                    revealInFinder(node.url)
                }
            }
        }
    }

    private var isExpanded: Bool {
        expandedFolderPaths.contains(node.id)
    }

    private func toggleFolder() {
        if expandedFolderPaths.contains(node.id) {
            expandedFolderPaths.remove(node.id)
        } else {
            expandedFolderPaths.insert(node.id)
        }
    }
}

private struct FolderTreeNode: Identifiable {
    let url: URL
    let title: String
    let depth: Int
    let isFolder: Bool
    let children: [FolderTreeNode]

    var id: String {
        url.standardizedFileURL.path
    }
}

private func droppedURL(from item: NSSecureCoding?) -> URL? {
    if let url = item as? URL {
        return url
    }

    if let data = item as? Data,
       let path = String(data: data, encoding: .utf8),
       let url = URL(string: path) {
        return url
    }

    return nil
}

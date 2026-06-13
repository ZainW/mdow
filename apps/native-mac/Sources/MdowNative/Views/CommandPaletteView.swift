import AppKit
import MdowNativeCore
import SwiftUI

struct CommandPaletteView: View {
    @EnvironmentObject private var store: DocumentStore
    @Binding var isPresented: Bool

    @State private var query = ""
    @State private var selectedIndex = 0
    @State private var isSearchFocused = true

    private var allFiles: [MarkdownFileSummary] {
        let files = store.openDocuments.map { MarkdownFileSummary(url: $0.url) }
            + store.folderFiles
            + store.recents
        return QuickOpenSearch.deduped(files)
    }

    private var normalizedQuery: String {
        query.trimmingCharacters(in: .whitespacesAndNewlines)
    }

    private var items: [PaletteItem] {
        let commands = PaletteCommand.allCases
            .filter { $0.isAvailable(in: store) }
            .filter { normalizedQuery.isEmpty || $0.matches(normalizedQuery) }
            .map(PaletteItem.command)
        let files = QuickOpenSearch.results(query: normalizedQuery, files: allFiles)
            .map(PaletteItem.file)
        return commands + files
    }

    var body: some View {
        VStack(spacing: 0) {
            searchHeader

            Rectangle()
                .fill(MdowStyle.borderSubtle.opacity(0.55))
                .frame(height: 1)

            ScrollViewReader { proxy in
                ScrollView {
                    if items.isEmpty {
                        emptyResults
                    } else {
                        LazyVStack(spacing: 2) {
                            ForEach(Array(items.enumerated()), id: \.element.id) { index, item in
                                paletteRow(item: item, index: index)
                                    .id(item.id)
                            }
                        }
                        .padding(.horizontal, 8)
                        .padding(.vertical, 8)
                    }
                }
                .scrollContentBackground(.hidden)
                .onChange(of: selectedIndex) { _, index in
                    guard items.indices.contains(index) else { return }
                    withAnimation(.easeInOut(duration: 0.12)) {
                        proxy.scrollTo(items[index].id, anchor: .center)
                    }
                }
            }
        }
        .background(MdowStyle.background)
        .overlay {
            RoundedRectangle(cornerRadius: 10)
                .stroke(MdowStyle.border.opacity(0.72), lineWidth: 1)
        }
        .frame(width: 620, height: 460)
        .onAppear {
            selectedIndex = 0
            isSearchFocused = true
        }
        .onChange(of: query) { _, _ in
            selectedIndex = 0
        }
        .onChange(of: items.count) { _, count in
            selectedIndex = min(selectedIndex, max(0, count - 1))
        }
        .onExitCommand {
            close()
        }
        .accessibilityLabel("Command Palette")
    }

    private var searchHeader: some View {
        HStack(spacing: 11) {
            Image(systemName: "magnifyingglass")
                .foregroundStyle(MdowStyle.mutedForeground)
                .font(.system(size: 14, weight: .medium))
                .accessibilityHidden(true)

            QuickOpenSearchField(
                text: $query,
                isFocused: $isSearchFocused,
                placeholder: "Search commands or files",
                onSubmit: openSelectedItem,
                onMoveUp: selectPreviousItem,
                onMoveDown: selectNextItem,
                onClose: close
            )
            .frame(height: 24)

            Text(resultSummary)
                .font(.system(size: 11, weight: .medium))
                .foregroundStyle(MdowStyle.mutedForeground)
                .monospacedDigit()
                .frame(minWidth: 58, alignment: .trailing)
                .accessibilityHidden(true)
        }
        .padding(.horizontal, 16)
        .frame(height: 50)
    }

    @ViewBuilder
    private func paletteRow(item: PaletteItem, index: Int) -> some View {
        let isSelected = index == selectedIndex

        Button {
            open(item)
        } label: {
            HStack(spacing: 10) {
                Image(systemName: item.systemImage)
                    .font(.system(size: 13, weight: .medium))
                    .foregroundStyle(isSelected ? MdowStyle.foreground.opacity(0.82) : MdowStyle.mutedForeground)
                    .frame(width: 17)
                    .accessibilityHidden(true)

                VStack(alignment: .leading, spacing: 2) {
                    Text(item.title)
                        .font(.system(size: 13, weight: .medium))
                        .foregroundStyle(MdowStyle.foreground)
                        .lineLimit(1)

                    if let subtitle = item.subtitle {
                        Text(subtitle)
                            .font(.system(size: 11))
                            .foregroundStyle(MdowStyle.mutedForeground.opacity(0.78))
                            .lineLimit(1)
                    }
                }

                Spacer(minLength: 12)

                if let badge = item.badge(activeURL: store.activeURL) {
                    Text(badge)
                        .font(.system(size: 10, weight: .medium, design: .monospaced))
                        .foregroundStyle(MdowStyle.mutedForeground)
                        .padding(.horizontal, 6)
                        .padding(.vertical, 2)
                        .background(MdowStyle.muted.opacity(0.72), in: RoundedRectangle(cornerRadius: 4))
                }
            }
            .padding(.horizontal, 10)
            .padding(.vertical, 7)
            .frame(maxWidth: .infinity, minHeight: 42, alignment: .leading)
            .contentShape(RoundedRectangle(cornerRadius: 7))
        }
        .buttonStyle(.plain)
        .background(
            isSelected ? MdowStyle.sidebarAccent : Color.clear,
            in: RoundedRectangle(cornerRadius: 7)
        )
        .overlay(alignment: .leading) {
            if isSelected {
                RoundedRectangle(cornerRadius: 1)
                    .fill(MdowStyle.primary.opacity(0.78))
                    .frame(width: 2)
                    .padding(.vertical, 7)
                    .offset(x: 1)
            }
        }
        .accessibilityElement(children: .ignore)
        .accessibilityLabel(accessibilityLabel(for: item, isSelected: isSelected))
        .accessibilityHint(item.accessibilityHint)
    }

    private var emptyResults: some View {
        VStack(spacing: 9) {
            Image(systemName: "command")
                .font(.system(size: 24, weight: .medium))
                .foregroundStyle(MdowStyle.mutedForeground.opacity(0.55))
                .accessibilityHidden(true)

            Text(normalizedQuery.isEmpty ? "No commands available" : "No matching commands or files")
                .font(.system(size: 13, weight: .medium))
                .foregroundStyle(MdowStyle.foreground)

            Text(normalizedQuery.isEmpty ? "Open a document or folder to unlock more actions." : "Try a command, filename, folder, or acronym.")
                .font(.system(size: 12))
                .foregroundStyle(MdowStyle.mutedForeground)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .padding(.top, 108)
        .accessibilityElement(children: .combine)
    }

    private var resultSummary: String {
        if items.isEmpty { return "No results" }
        return "\(selectedIndex + 1) of \(items.count)"
    }

    private func selectPreviousItem() {
        guard !items.isEmpty else { return }
        selectedIndex = selectedIndex == 0 ? items.count - 1 : selectedIndex - 1
    }

    private func selectNextItem() {
        guard !items.isEmpty else { return }
        selectedIndex = selectedIndex >= items.count - 1 ? 0 : selectedIndex + 1
    }

    private func openSelectedItem() {
        guard items.indices.contains(selectedIndex) else { return }
        open(items[selectedIndex])
    }

    private func open(_ item: PaletteItem) {
        switch item {
        case .command(let command):
            run(command)
        case .file(let result):
            store.open(result.file.url)
            close()
        }
    }

    private func run(_ command: PaletteCommand) {
        close()

        switch command {
        case .openFile:
            runAfterDismissal { store.openWithPanel() }
        case .openFolder:
            runAfterDismissal { store.openFolderWithPanel() }
        case .find:
            runAfterDismissal {
                NotificationCenter.default.post(name: AppDelegate.focusSearchNotification, object: nil)
            }
        case .toggleSidebar:
            store.toggleSidebar()
        case .toggleFullWidth:
            store.toggleWideMode()
        case .keyboardShortcuts:
            runAfterDismissal {
                NotificationCenter.default.post(name: AppDelegate.showShortcutsNotification, object: nil)
            }
        case .settings:
            runAfterDismissal { openNativeSettings() }
        case .closeTab:
            store.closeActiveDocument()
        case .nextTab:
            store.activateNextDocument()
        case .previousTab:
            store.activatePreviousDocument()
        case .copyPath:
            if let activeURL = store.activeURL {
                store.copyPath(activeURL)
            }
        case .revealInFinder:
            if let activeURL = store.activeURL {
                store.revealInFinder(activeURL)
            }
        }
    }

    private func close() {
        isPresented = false
    }

    private func openNativeSettings() {
        NSApp.sendAction(Selector(("showSettingsWindow:")), to: nil, from: nil)
    }

    private func runAfterDismissal(_ action: @escaping @MainActor () -> Void) {
        Task { @MainActor in
            try? await Task.sleep(nanoseconds: 160_000_000)
            action()
        }
    }

    private func accessibilityLabel(for item: PaletteItem, isSelected: Bool) -> String {
        let state = isSelected ? "selected, " : ""
        if let subtitle = item.subtitle {
            return "\(state)\(item.title), \(subtitle)"
        }
        return "\(state)\(item.title)"
    }
}

private enum PaletteItem: Identifiable {
    case command(PaletteCommand)
    case file(QuickOpenSearchResult)

    var id: String {
        switch self {
        case .command(let command):
            "command-\(command.rawValue)"
        case .file(let result):
            "file-\(result.file.url.standardizedFileURL.path)"
        }
    }

    var title: String {
        switch self {
        case .command(let command):
            command.title
        case .file(let result):
            result.file.title
        }
    }

    var subtitle: String? {
        switch self {
        case .command(let command):
            command.subtitle
        case .file(let result):
            compactParentPath(result.file.url)
        }
    }

    var systemImage: String {
        switch self {
        case .command(let command):
            command.systemImage
        case .file:
            "doc.text"
        }
    }

    var accessibilityHint: String {
        switch self {
        case .command:
            "Runs this command"
        case .file:
            "Opens this Markdown file"
        }
    }

    func badge(activeURL: URL?) -> String? {
        switch self {
        case .command(let command):
            command.shortcut
        case .file(let result):
            result.file.url == activeURL ? "Open" : nil
        }
    }

    private func compactParentPath(_ url: URL) -> String? {
        let components = url.deletingLastPathComponent().pathComponents
            .filter { $0 != "/" }
        guard !components.isEmpty else { return nil }

        return components.suffix(2).joined(separator: "/")
    }
}

private enum PaletteCommand: String, CaseIterable, Identifiable {
    case openFile
    case openFolder
    case find
    case toggleSidebar
    case toggleFullWidth
    case keyboardShortcuts
    case settings
    case closeTab
    case nextTab
    case previousTab
    case copyPath
    case revealInFinder

    var id: String { rawValue }

    var title: String {
        switch self {
        case .openFile: "Open File"
        case .openFolder: "Open Folder"
        case .find: "Find in Document"
        case .toggleSidebar: "Toggle Sidebar"
        case .toggleFullWidth: "Toggle Full Width"
        case .keyboardShortcuts: "Keyboard Shortcuts"
        case .settings: "Settings"
        case .closeTab: "Close Tab"
        case .nextTab: "Next Tab"
        case .previousTab: "Previous Tab"
        case .copyPath: "Copy Path"
        case .revealInFinder: "Reveal in Finder"
        }
    }

    var subtitle: String {
        switch self {
        case .openFile: "Choose a Markdown file"
        case .openFolder: "Choose a folder of Markdown files"
        case .find: "Search within the active document"
        case .toggleSidebar: "Show or hide the sidebar"
        case .toggleFullWidth: "Use or constrain the reader width"
        case .keyboardShortcuts: "Show available keyboard shortcuts"
        case .settings: "Open native app settings"
        case .closeTab: "Close the active document tab"
        case .nextTab: "Move to the next open document"
        case .previousTab: "Move to the previous open document"
        case .copyPath: "Copy the active document path"
        case .revealInFinder: "Show the active document in Finder"
        }
    }

    var systemImage: String {
        switch self {
        case .openFile: "doc"
        case .openFolder: "folder"
        case .find: "magnifyingglass"
        case .toggleSidebar: "sidebar.left"
        case .toggleFullWidth: "arrow.left.and.right"
        case .keyboardShortcuts: "keyboard"
        case .settings: "gearshape"
        case .closeTab: "xmark.square"
        case .nextTab: "arrow.right.square"
        case .previousTab: "arrow.left.square"
        case .copyPath: "doc.on.doc"
        case .revealInFinder: "finder"
        }
    }

    var shortcut: String? {
        switch self {
        case .openFile: "⌘O"
        case .openFolder: "⇧⌘O"
        case .find: "⌘F"
        case .toggleSidebar: "⌘B"
        case .toggleFullWidth: "⇧⌘W"
        case .keyboardShortcuts: "⌘/"
        case .settings: "⌘,"
        case .closeTab: "⌘W"
        case .nextTab: "⇧⌘]"
        case .previousTab: "⇧⌘["
        case .copyPath, .revealInFinder:
            nil
        }
    }

    @MainActor
    func isAvailable(in store: DocumentStore) -> Bool {
        switch self {
        case .find, .closeTab, .copyPath, .revealInFinder:
            store.activeURL != nil
        case .nextTab, .previousTab:
            store.openDocuments.count > 1
        default:
            true
        }
    }

    func matches(_ query: String) -> Bool {
        let terms = query.lowercased().split(whereSeparator: \.isWhitespace)
        guard !terms.isEmpty else { return true }
        let haystack = "\(title) \(subtitle) \(rawValue)".lowercased()
        return terms.allSatisfy { haystack.contains($0) }
    }
}

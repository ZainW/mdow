import MdowNativeCore
import SwiftUI

struct CommandPaletteView: View {
    @EnvironmentObject private var store: DocumentStore
    @Binding var isPresented: Bool
    var onOpen: () -> Void = {}

    @State private var query = ""
    @State private var selectedIndex = 0
    @State private var isSearchFocused = true

    private var allFiles: [MarkdownFileSummary] {
        let files = store.openDocuments.map { MarkdownFileSummary(url: $0.url) }
            + store.folderFiles
            + store.recents
        return QuickOpenSearch.deduped(files)
    }

    private var results: [QuickOpenSearchResult] {
        QuickOpenSearch.results(query: query, files: allFiles)
    }

    var body: some View {
        VStack(spacing: 0) {
            searchHeader

            Rectangle()
                .fill(MdowStyle.borderSubtle.opacity(0.55))
                .frame(height: 1)

            ScrollViewReader { proxy in
                ScrollView {
                    if results.isEmpty {
                        emptyResults
                    } else {
                        LazyVStack(spacing: 2) {
                            ForEach(Array(results.enumerated()), id: \.element.file.url) { index, result in
                                quickOpenRow(result: result, index: index)
                                    .id(result.file.url)
                            }
                        }
                        .padding(.horizontal, 8)
                        .padding(.vertical, 8)
                    }
                }
                .scrollContentBackground(.hidden)
                .onChange(of: selectedIndex) { _, index in
                    guard results.indices.contains(index) else { return }
                    withAnimation(.easeInOut(duration: 0.12)) {
                        proxy.scrollTo(results[index].file.url, anchor: .center)
                    }
                }
            }
        }
        .background(MdowStyle.background)
        .overlay {
            RoundedRectangle(cornerRadius: 10)
                .stroke(MdowStyle.border.opacity(0.72), lineWidth: 1)
        }
        .frame(width: 580, height: 430)
        .onAppear {
            selectedIndex = 0
            isSearchFocused = true
        }
        .onChange(of: query) { _, _ in
            selectedIndex = 0
        }
        .onChange(of: results.count) { _, count in
            selectedIndex = min(selectedIndex, max(0, count - 1))
        }
        .onExitCommand {
            close()
        }
        .accessibilityLabel("Quick Open")
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
                placeholder: "Open file",
                onSubmit: openSelectedResult,
                onMoveUp: selectPreviousResult,
                onMoveDown: selectNextResult,
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
    private func quickOpenRow(result: QuickOpenSearchResult, index: Int) -> some View {
        let file = result.file
        let isSelected = index == selectedIndex

        Button {
            open(file)
        } label: {
            HStack(spacing: 10) {
                Image(systemName: "doc.text")
                    .font(.system(size: 13, weight: .medium))
                    .foregroundStyle(isSelected ? MdowStyle.foreground.opacity(0.82) : MdowStyle.mutedForeground)
                    .frame(width: 17)
                    .accessibilityHidden(true)

                VStack(alignment: .leading, spacing: 2) {
                    Text(file.title)
                        .font(.system(size: 13, weight: .medium))
                        .foregroundStyle(MdowStyle.foreground)
                        .lineLimit(1)

                    if let parent = compactParentPath(file.url) {
                        Text(parent)
                            .font(.system(size: 11))
                            .foregroundStyle(MdowStyle.mutedForeground.opacity(0.78))
                            .lineLimit(1)
                    }
                }

                Spacer(minLength: 12)

                if file.url == store.activeURL {
                    Text("Open")
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
        .accessibilityLabel(accessibilityLabel(for: file, isSelected: isSelected))
        .accessibilityHint("Opens this Markdown file")
    }

    private var emptyResults: some View {
        VStack(spacing: 9) {
            Image(systemName: "doc.text.magnifyingglass")
                .font(.system(size: 24, weight: .medium))
                .foregroundStyle(MdowStyle.mutedForeground.opacity(0.55))
                .accessibilityHidden(true)

            Text(query.isEmpty ? "No files available" : "No matching files")
                .font(.system(size: 13, weight: .medium))
                .foregroundStyle(MdowStyle.foreground)

            Text(query.isEmpty ? "Open a file or folder first." : "Try a filename, folder, or acronym.")
                .font(.system(size: 12))
                .foregroundStyle(MdowStyle.mutedForeground)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .padding(.top, 96)
        .accessibilityElement(children: .combine)
    }

    private func compactParentPath(_ url: URL) -> String? {
        let components = url.deletingLastPathComponent().pathComponents
            .filter { $0 != "/" }
        guard !components.isEmpty else { return nil }

        return components.suffix(2).joined(separator: "/")
    }

    private var resultSummary: String {
        if results.isEmpty { return "No results" }
        return "\(selectedIndex + 1) of \(results.count)"
    }

    private func selectPreviousResult() {
        guard !results.isEmpty else { return }
        selectedIndex = selectedIndex == 0 ? results.count - 1 : selectedIndex - 1
    }

    private func selectNextResult() {
        guard !results.isEmpty else { return }
        selectedIndex = selectedIndex >= results.count - 1 ? 0 : selectedIndex + 1
    }

    private func openSelectedResult() {
        guard results.indices.contains(selectedIndex) else { return }
        open(results[selectedIndex].file)
    }

    private func open(_ file: MarkdownFileSummary) {
        onOpen()
        store.open(file.url)
        close()
    }

    private func close() {
        isPresented = false
    }

    private func accessibilityLabel(for file: MarkdownFileSummary, isSelected: Bool) -> String {
        let state = isSelected ? "selected, " : ""
        return "\(state)\(file.title), \(file.url.path)"
    }
}

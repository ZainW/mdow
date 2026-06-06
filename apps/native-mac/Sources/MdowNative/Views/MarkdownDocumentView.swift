import MdowNativeCore
import SwiftUI

struct MarkdownDocumentView: View {
    let document: MarkdownDocument
    let blocks: [MarkdownBlock]
    let searchQuery: String
    @Binding var scrollTarget: Int?
    let isWide: Bool
    let zoomLevel: Int
    let readingWidth: ReadingWidth
    let contentFont: ContentFontPreset
    let codeFont: CodeFontPreset
    @State private var copyFeedback = CodeCopyFeedbackState()

    var body: some View {
        let layout = DocumentReadingLayout(readingWidth: readingWidth, isWide: isWide)
        let maxContentWidth = layout.maxContentWidth.map { CGFloat($0) } ?? .infinity

        ScrollViewReader { proxy in
            ScrollView {
                VStack(alignment: .leading, spacing: 0) {
                    ForEach(Array(blocks.enumerated()), id: \.offset) { index, block in
                        blockView(block, index: index)
                            .id(index)
                            .padding(.bottom, bottomSpacing(after: block, before: nextBlock(after: index)))
                    }
                }
                .frame(
                    maxWidth: maxContentWidth,
                    alignment: .leading
                )
                .padding(.horizontal, CGFloat(layout.horizontalPadding))
                .padding(.top, 22)
                .padding(.bottom, 40)
                .frame(maxWidth: .infinity, alignment: layout.swiftUIAlignment)
            }
            .onChange(of: scrollTarget) { _, target in
                guard let target else { return }
                withAnimation(.easeInOut(duration: 0.18)) {
                    proxy.scrollTo(target, anchor: .top)
                }
                scrollTarget = nil
            }
        }
    }

    @ViewBuilder
    private func blockView(_ block: MarkdownBlock, index: Int) -> some View {
        switch block {
        case .heading(let level, let text):
            let presentation = MarkdownHeadingPresentation(level: level)
            markdownText(text)
                .font(headingFont(presentation))
                .foregroundStyle(presentation.isMuted ? MdowStyle.mutedForeground : MdowStyle.foreground)
                .textCase(presentation.isUppercase ? .uppercase : nil)
                .padding(.top, scaled(CGFloat(presentation.topPadding)))

        case .paragraph(let text):
            NativeMarkdownInlineText(
                attributedText: markdownAttributedText(text),
                font: contentFont.nsFont(size: scaled(15.5)),
                codeFont: codeFont.nsFont(size: scaled(15.5)),
                lineSpacing: scaled(5)
            )
            .frame(maxWidth: .infinity, alignment: .leading)

        case .unorderedListItem(let text):
            HStack(alignment: .firstTextBaseline, spacing: 8) {
                Text("•")
                markdownText(text)
            }
            .font(contentFont.font(size: scaled(15.5)))

        case .orderedListItem(let number, let text):
            HStack(alignment: .firstTextBaseline, spacing: 8) {
                Text("\(number).")
                    .foregroundStyle(.secondary)
                markdownText(text)
            }
            .font(contentFont.font(size: scaled(15.5)))

        case .taskListItem(let isComplete, let text):
            HStack(alignment: .firstTextBaseline, spacing: 8) {
                Image(systemName: isComplete ? "checkmark.square.fill" : "square")
                    .foregroundStyle(isComplete ? Color.accentColor : Color.secondary)
                    .accessibilityLabel(isComplete ? "Completed task" : "Incomplete task")
                markdownText(text)
            }
            .font(contentFont.font(size: scaled(15.5)))

        case .blockquote(let text):
            let style = MarkdownBlockquoteStyle.standard
            HStack(alignment: .top, spacing: 0) {
                Rectangle()
                    .fill(MdowStyle.border)
                    .frame(width: CGFloat(style.borderWidth))
                quoteText(text)
                    .foregroundStyle(style.textIsMuted ? MdowStyle.mutedForeground : MdowStyle.foreground)
                    .font(contentFont.font(size: scaled(15.5)))
                    .lineSpacing(scaled(5))
                    .padding(.horizontal, CGFloat(style.horizontalPadding))
                    .padding(.vertical, CGFloat(style.verticalPadding))
            }

        case .callout(let kind, let title, let text):
            calloutView(kind: kind, title: title, text: text)

        case .footnoteDefinition(let label, let text):
            footnoteDefinitionView(label: label, text: text)

        case .horizontalRule:
            Divider()
                .padding(.vertical, 6)

        case .codeBlock(let language, let code):
            let codeID = "code-\(index)"
            let isCopied = copyFeedback.isCopied(codeID)
            let style = MarkdownCodeBlockStyle.standard
            let codeTextFont = codeFont.nsFont(size: scaled(15.5 * style.codeFontScale))
            let codeLineSpacing = scaled(4)
            VStack(alignment: .leading, spacing: 10) {
                HStack(spacing: 8) {
                    Spacer(minLength: 0)
                    if let language {
                        Text(language)
                            .font(codeFont.font(size: scaled(CGFloat(style.languageFontSize)), weight: .medium))
                            .foregroundStyle(MdowStyle.mutedForeground)
                            .textCase(.lowercase)
                    }
                    Button {
                        NSPasteboard.general.clearContents()
                        NSPasteboard.general.setString(code, forType: .string)
                        copyFeedback = copyFeedback.copying(codeID)
                        Task { @MainActor in
                            try? await Task.sleep(nanoseconds: 2_000_000_000)
                            copyFeedback = copyFeedback.resetting(codeID)
                        }
                    } label: {
                        HStack(spacing: 5) {
                            Image(systemName: isCopied ? "checkmark" : "doc.on.doc")
                                .contentTransition(.symbolEffect(.replace))
                            if isCopied {
                                Text("Copied")
                                    .font(codeFont.font(size: scaled(11), weight: .medium))
                            }
                        }
                        .foregroundStyle(isCopied ? MdowStyle.primary : MdowStyle.mutedForeground)
                    }
                    .buttonStyle(.borderless)
                    .help(isCopied ? "Copied" : "Copy code")
                    .accessibilityLabel(isCopied ? "Copied" : "Copy code")
                }
                .frame(height: 18)

                NativeCodeBlockText(
                    code: code,
                    language: language,
                    searchQuery: searchQuery,
                    font: codeTextFont,
                    lineSpacing: codeLineSpacing
                )
                .frame(
                    maxWidth: .infinity,
                    minHeight: NativeCodeBlockText.height(
                        for: code,
                        font: codeTextFont,
                        lineSpacing: codeLineSpacing
                    ),
                    alignment: .leading
                )
            }
            .padding(.horizontal, CGFloat(style.horizontalPadding))
            .padding(.vertical, CGFloat(style.verticalPadding))
            .background(NativeCodeTheme.surface, in: RoundedRectangle(cornerRadius: CGFloat(style.cornerRadius)))
            .overlay {
                RoundedRectangle(cornerRadius: CGFloat(style.cornerRadius))
                    .stroke(NativeCodeTheme.border, lineWidth: 1)
            }
            .shadow(color: .black.opacity(0.025), radius: 3, y: CGFloat(style.shadowOffsetY))

        case .mermaidDiagram(let diagram):
            mermaidDiagramView(diagram)

        case .mathBlock(let formula):
            mathBlockView(formula)

        case .table(let headers, let rows):
            tableView(headers: headers, rows: rows)

        case .image(let alt, let source):
            imageView(alt: alt, source: source)
        }
    }

    private func footnoteDefinitionView(label: String, text: String) -> some View {
        HStack(alignment: .firstTextBaseline, spacing: 10) {
            Text("[\(label)]")
                .font(codeFont.font(size: scaled(12), weight: .medium))
                .foregroundStyle(MdowStyle.mutedForeground)
                .textSelection(.enabled)
                .frame(minWidth: 36, alignment: .trailing)

            NativeMarkdownInlineText(
                attributedText: markdownAttributedText(text),
                font: contentFont.nsFont(size: scaled(14)),
                codeFont: codeFont.nsFont(size: scaled(14)),
                lineSpacing: scaled(4)
            )
            .foregroundStyle(MdowStyle.mutedForeground)
            .frame(maxWidth: .infinity, alignment: .leading)
        }
        .padding(.top, 4)
    }

    private func mermaidDiagramView(_ diagram: MermaidDiagram) -> some View {
        let isHorizontal = diagram.direction == .leftToRight || diagram.direction == .rightToLeft
        let displayNodes = displayedMermaidNodes(diagram)

        return VStack(alignment: .leading, spacing: 14) {
            HStack(spacing: 8) {
                Image(systemName: "point.3.connected.trianglepath.dotted")
                    .foregroundStyle(MdowStyle.primary)
                    .accessibilityHidden(true)
                Text("Mermaid")
                    .font(codeFont.font(size: scaled(11), weight: .medium))
                    .foregroundStyle(MdowStyle.mutedForeground)
                    .textCase(.lowercase)
                Spacer()
            }

            if isHorizontal {
                ScrollView(.horizontal, showsIndicators: false) {
                    HStack(spacing: 10) {
                        ForEach(Array(displayNodes.enumerated()), id: \.element.id) { index, node in
                            mermaidNodeView(node)
                            if index < displayNodes.count - 1 {
                                Image(systemName: "arrow.right")
                                    .foregroundStyle(MdowStyle.mutedForeground)
                                    .accessibilityHidden(true)
                            }
                        }
                    }
                    .padding(.vertical, 2)
                }
            } else {
                VStack(alignment: .center, spacing: 8) {
                    ForEach(Array(displayNodes.enumerated()), id: \.element.id) { index, node in
                        mermaidNodeView(node)
                        if index < displayNodes.count - 1 {
                            Image(systemName: "arrow.down")
                                .foregroundStyle(MdowStyle.mutedForeground)
                                .accessibilityHidden(true)
                        }
                    }
                }
                .frame(maxWidth: .infinity)
            }
        }
        .padding(.horizontal, 18)
        .padding(.vertical, 15)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(MdowStyle.muted.opacity(0.62), in: RoundedRectangle(cornerRadius: 8))
        .overlay {
            RoundedRectangle(cornerRadius: 8)
                .stroke(MdowStyle.border.opacity(0.72), lineWidth: 1)
        }
    }

    private func mermaidNodeView(_ node: MermaidNode) -> some View {
        Text(node.label)
            .font(contentFont.font(size: scaled(13.5), weight: .medium))
            .foregroundStyle(MdowStyle.foreground)
            .lineLimit(2)
            .multilineTextAlignment(.center)
            .padding(.horizontal, 14)
            .padding(.vertical, 9)
            .frame(minWidth: 86)
            .background(MdowStyle.elevatedSurface, in: RoundedRectangle(cornerRadius: 7))
            .overlay {
                RoundedRectangle(cornerRadius: 7)
                    .stroke(MdowStyle.borderSubtle.opacity(0.72), lineWidth: 1)
            }
            .textSelection(.enabled)
    }

    private func mathBlockView(_ formula: String) -> some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack(spacing: 8) {
                Image(systemName: "function")
                    .foregroundStyle(MdowStyle.primary)
                    .accessibilityHidden(true)
                Text("Math")
                    .font(codeFont.font(size: scaled(11), weight: .medium))
                    .foregroundStyle(MdowStyle.mutedForeground)
                    .textCase(.lowercase)
                Spacer()
            }

            Text(formula)
                .font(codeFont.font(size: scaled(14)))
                .foregroundStyle(MdowStyle.foreground)
                .lineSpacing(scaled(5))
                .textSelection(.enabled)
                .frame(maxWidth: .infinity, alignment: .center)
                .padding(.vertical, 4)
        }
        .padding(.horizontal, 18)
        .padding(.vertical, 15)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(MdowStyle.muted.opacity(0.62), in: RoundedRectangle(cornerRadius: 8))
        .overlay {
            RoundedRectangle(cornerRadius: 8)
                .stroke(MdowStyle.border.opacity(0.72), lineWidth: 1)
        }
    }

    private func displayedMermaidNodes(_ diagram: MermaidDiagram) -> [MermaidNode] {
        switch diagram.direction {
        case .rightToLeft, .bottomTop:
            Array(diagram.nodes.reversed())
        case .leftToRight, .topDown:
            diagram.nodes
        }
    }

    private func calloutView(kind: String, title: String, text: String) -> some View {
        let style = calloutStyle(kind: kind)

        return HStack(alignment: .top, spacing: 12) {
            Image(systemName: style.symbol)
                .font(.system(size: scaled(14), weight: .semibold))
                .foregroundStyle(style.color)
                .frame(width: 18)
                .padding(.top, 2)
                .accessibilityHidden(true)

            VStack(alignment: .leading, spacing: text.isEmpty ? 0 : 7) {
                markdownText(title)
                    .font(contentFont.font(size: scaled(14.5), weight: .semibold))
                    .foregroundStyle(style.color)

                if !text.isEmpty {
                    quoteText(text)
                        .font(contentFont.font(size: scaled(15.5)))
                        .foregroundStyle(MdowStyle.foreground)
                        .lineSpacing(scaled(5))
                }
            }
        }
        .padding(.horizontal, 15)
        .padding(.vertical, 13)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(MdowStyle.muted.opacity(0.68), in: RoundedRectangle(cornerRadius: 8))
        .overlay(alignment: .leading) {
            RoundedRectangle(cornerRadius: 8)
                .fill(style.color)
                .frame(width: 4)
        }
        .overlay {
            RoundedRectangle(cornerRadius: 8)
                .stroke(MdowStyle.border.opacity(0.78), lineWidth: 1)
        }
    }

    private func calloutStyle(kind: String) -> (symbol: String, color: Color) {
        switch kind {
        case "tip":
            ("lightbulb", Color(nsColor: NSColor(red: 0.137, green: 0.525, blue: 0.212, alpha: 1)))
        case "important":
            ("sparkles", Color(nsColor: NSColor(red: 0.537, green: 0.341, blue: 0.898, alpha: 1)))
        case "warning":
            ("exclamationmark.triangle", Color(nsColor: NSColor(red: 0.620, green: 0.416, blue: 0.012, alpha: 1)))
        case "caution":
            ("xmark.octagon", Color(nsColor: NSColor(red: 0.855, green: 0.212, blue: 0.200, alpha: 1)))
        default:
            ("info.circle", Color(nsColor: NSColor(red: 0.122, green: 0.435, blue: 0.922, alpha: 1)))
        }
    }

    private func headingFont(_ presentation: MarkdownHeadingPresentation) -> Font {
        contentFont.font(
            size: scaled(15.5 * presentation.fontScale),
            weight: presentation.fontWeight
        )
    }

    private func scaled(_ value: CGFloat) -> CGFloat {
        value * CGFloat(zoomLevel) / 100
    }

    private func nextBlock(after index: Int) -> MarkdownBlock? {
        let nextIndex = index + 1
        guard blocks.indices.contains(nextIndex) else { return nil }
        return blocks[nextIndex]
    }

    private func bottomSpacing(after block: MarkdownBlock, before nextBlock: MarkdownBlock?) -> CGFloat {
        guard let nextBlock else { return 0 }

        if block.isListItem, nextBlock.isListItem {
            return 4
        }

        if block.isListItem {
            return 12
        }

        switch block {
        case .heading:
            return nextBlock.isListItem ? 12 : 14
        case .paragraph, .blockquote, .callout:
            return nextBlock.isListItem ? 10 : 16
        case .horizontalRule:
            return 18
        case .codeBlock, .mermaidDiagram, .mathBlock, .table, .image:
            return 18
        case .footnoteDefinition:
            return 8
        case .unorderedListItem, .orderedListItem, .taskListItem:
            return 12
        }
    }

    private func markdownText(_ source: String) -> Text {
        Text(markdownAttributedText(source))
    }

    private func markdownAttributedText(_ source: String) -> AttributedString {
        let normalizedSource = MarkdownInline.normalizedMarkdown(source)
        if let attributed = try? AttributedString(
            markdown: normalizedSource,
            options: AttributedString.MarkdownParsingOptions(
                interpretedSyntax: .inlineOnlyPreservingWhitespace
            )
        ) {
            return highlightedSearchMatches(in: attributed, query: searchQuery)
        }

        return highlightedPlainText(source, query: searchQuery)
    }

    private func quoteText(_ source: String) -> Text {
        let normalizedSource = MarkdownInline.normalizedMarkdown(source)
        if let attributed = try? AttributedString(
            markdown: normalizedSource,
            options: AttributedString.MarkdownParsingOptions(
                interpretedSyntax: .inlineOnlyPreservingWhitespace
            )
        ) {
            return Text(highlightedSearchMatches(in: attributed, query: searchQuery))
        }

        return Text(highlightedPlainText(source, query: searchQuery))
    }

    private func tableView(headers: [String], rows: [[String]]) -> some View {
        let style = MarkdownTableGridStyle.standard
        let allRows = [headers] + rows

        return VStack(alignment: .leading, spacing: 0) {
            ForEach(Array(allRows.enumerated()), id: \.offset) { rowIndex, row in
                tableRow(
                    cells: row,
                    isHeader: rowIndex == 0,
                    showsBottomBorder: rowIndex < allRows.count - 1,
                    style: style
                )
            }
        }
        .font(contentFont.font(size: scaled(15.5 * style.bodyFontScale)))
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(MdowStyle.elevatedSurface, in: RoundedRectangle(cornerRadius: CGFloat(style.cornerRadius)))
        .overlay {
            RoundedRectangle(cornerRadius: CGFloat(style.cornerRadius))
                .stroke(MdowStyle.border.opacity(0.82), lineWidth: 1)
        }
        .clipShape(RoundedRectangle(cornerRadius: CGFloat(style.cornerRadius)))
    }

    private func tableRow(
        cells: [String],
        isHeader: Bool,
        showsBottomBorder: Bool,
        style: MarkdownTableGridStyle
    ) -> some View {
        HStack(alignment: .top, spacing: 0) {
            ForEach(Array(cells.enumerated()), id: \.offset) { index, cell in
                tableCell(
                    cell,
                    isHeader: isHeader,
                    showsTrailingBorder: index < cells.count - 1,
                    style: style
                )
            }
        }
        .background(isHeader ? MdowStyle.muted.opacity(0.74) : Color.clear)
        .overlay(alignment: .bottom) {
            if showsBottomBorder {
                Rectangle()
                    .fill(isHeader ? MdowStyle.border.opacity(0.8) : MdowStyle.borderSubtle.opacity(0.72))
                    .frame(height: 1)
            }
        }
    }

    private func tableCell(
        _ text: String,
        isHeader: Bool,
        showsTrailingBorder: Bool,
        style: MarkdownTableGridStyle
    ) -> some View {
        markdownText(text)
            .font(contentFont.font(
                size: scaled(15.5 * (isHeader ? style.headerFontScale : style.bodyFontScale)),
                weight: isHeader ? .semibold : .regular
            ))
            .foregroundStyle(isHeader ? MdowStyle.mutedForeground : MdowStyle.foreground)
            .textCase(isHeader ? .uppercase : nil)
            .frame(maxWidth: .infinity, alignment: .leading)
            .padding(.horizontal, CGFloat(style.horizontalPadding))
            .padding(.vertical, CGFloat(style.verticalPadding))
            .overlay(alignment: .trailing) {
                if showsTrailingBorder {
                    Rectangle()
                        .fill(MdowStyle.borderSubtle.opacity(0.72))
                        .frame(width: 1)
                }
            }
    }

    @ViewBuilder
    private func imageView(alt: String, source: String) -> some View {
        let style = MarkdownImageStyle.standard
        let url = resolvedImageURL(source)
        if let image = NSImage(contentsOf: url) {
            VStack(alignment: .leading, spacing: 6) {
                Image(nsImage: image)
                    .resizable()
                    .scaledToFit()
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .clipShape(RoundedRectangle(cornerRadius: CGFloat(style.cornerRadius)))
                    .accessibilityLabel(alt.isEmpty ? source : alt)

                if !alt.isEmpty {
                    Text(alt)
                        .font(contentFont.font(size: scaled(CGFloat(style.captionFontSize))))
                        .foregroundStyle(MdowStyle.mutedForeground)
                        .textSelection(.enabled)
                }
            }
        } else {
            HStack(spacing: 8) {
                Image(systemName: "photo")
                    .foregroundStyle(MdowStyle.mutedForeground)
                    .accessibilityHidden(true)
                Text(alt.isEmpty ? source : alt)
                    .foregroundStyle(.secondary)
                    .font(contentFont.font(size: scaled(13)))
                    .textSelection(.enabled)
            }
            .padding(12)
            .background(
                MdowStyle.muted.opacity(0.55),
                in: RoundedRectangle(cornerRadius: CGFloat(style.cornerRadius))
            )
        }
    }

    private func resolvedImageURL(_ source: String) -> URL {
        if let url = URL(string: source), url.isFileURL {
            return url
        }

        if source.hasPrefix("/") {
            return URL(fileURLWithPath: source)
        }

        return document.url.deletingLastPathComponent().appendingPathComponent(source)
    }

    private func highlightedPlainText(_ source: String, query: String) -> AttributedString {
        highlightedSearchMatches(in: AttributedString(source), query: query)
    }

    private func highlightedSearchMatches(in source: AttributedString, query: String) -> AttributedString {
        guard !query.isEmpty else { return source }

        var attributed = source
        let plainText = String(attributed.characters)
        let matches = MarkdownSearch.matches(in: plainText, query: query)

        for match in matches {
            guard let range = attributedRange(for: match, in: attributed) else { continue }
            attributed[range].backgroundColor = MdowStyle.searchHighlight
            attributed[range].foregroundColor = MdowStyle.searchHighlightForeground
        }

        return attributed
    }

    private func attributedRange(
        for match: MarkdownSearchMatch,
        in attributed: AttributedString
    ) -> Range<AttributedString.Index>? {
        guard let lowerBound = attributed.characters.index(
            attributed.startIndex,
            offsetBy: match.offset,
            limitedBy: attributed.endIndex
        ),
              let upperBound = attributed.characters.index(
                  lowerBound,
                  offsetBy: match.length,
                  limitedBy: attributed.endIndex
              ) else {
            return nil
        }

        return lowerBound..<upperBound
    }
}

private extension MarkdownBlock {
    var isListItem: Bool {
        switch self {
        case .unorderedListItem, .orderedListItem, .taskListItem:
            true
        default:
            false
        }
    }
}

private extension DocumentReadingLayout {
    var swiftUIAlignment: Alignment {
        switch alignment {
        case .leading: .leading
        case .center: .center
        }
    }
}

private extension MarkdownHeadingPresentation {
    var fontWeight: Font.Weight {
        switch emphasis {
        case .bold: .bold
        case .semibold: .semibold
        }
    }
}

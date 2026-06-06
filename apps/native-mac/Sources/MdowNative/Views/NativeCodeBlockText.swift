import AppKit
import MdowNativeCore
import SwiftUI

struct NativeCodeBlockText: NSViewRepresentable {
    let code: String
    let language: String?
    let searchQuery: String
    let font: NSFont
    let lineSpacing: CGFloat

    static func height(for code: String, font: NSFont, lineSpacing: CGFloat) -> CGFloat {
        let lineCount = max(1, code.split(separator: "\n", omittingEmptySubsequences: false).count)
        let lineHeight = ceil(font.ascender - font.descender + font.leading + lineSpacing)
        return CGFloat(lineCount) * lineHeight + 2
    }

    func makeNSView(context: Context) -> NSScrollView {
        let scrollView = NSScrollView()
        scrollView.drawsBackground = false
        scrollView.borderType = .noBorder
        scrollView.hasVerticalScroller = false
        scrollView.hasHorizontalScroller = true
        scrollView.autohidesScrollers = true
        scrollView.scrollerStyle = .overlay

        let textView = NSTextView()
        textView.isEditable = false
        textView.isSelectable = true
        textView.drawsBackground = false
        textView.textContainerInset = .zero
        textView.textContainer?.lineFragmentPadding = 0
        textView.textContainer?.widthTracksTextView = false
        textView.textContainer?.containerSize = NSSize(
            width: CGFloat.greatestFiniteMagnitude,
            height: CGFloat.greatestFiniteMagnitude
        )
        textView.isHorizontallyResizable = true
        textView.isVerticallyResizable = false
        textView.minSize = .zero
        textView.maxSize = NSSize(width: CGFloat.greatestFiniteMagnitude, height: CGFloat.greatestFiniteMagnitude)
        textView.setAccessibilityLabel(languageLabel)

        scrollView.documentView = textView
        return scrollView
    }

    func updateNSView(_ scrollView: NSScrollView, context: Context) {
        guard let textView = scrollView.documentView as? NSTextView else { return }

        let attributedCode = NativeCodeHighlighter.highlightedCode(
            code,
            language: language,
            font: font,
            lineSpacing: lineSpacing,
            searchQuery: searchQuery
        )
        textView.textStorage?.setAttributedString(attributedCode)
        textView.frame = NSRect(origin: .zero, size: textSize(for: attributedCode))
        textView.setAccessibilityLabel(languageLabel)
    }

    private var languageLabel: String {
        if let language, !language.isEmpty {
            return "\(language) code block"
        }
        return "Code block"
    }

    private func textSize(for attributedCode: NSAttributedString) -> NSSize {
        let storage = NSTextStorage(attributedString: attributedCode)
        let container = NSTextContainer(size: NSSize(
            width: CGFloat.greatestFiniteMagnitude,
            height: CGFloat.greatestFiniteMagnitude
        ))
        container.lineFragmentPadding = 0
        container.widthTracksTextView = false

        let layoutManager = NSLayoutManager()
        layoutManager.addTextContainer(container)
        storage.addLayoutManager(layoutManager)
        layoutManager.ensureLayout(for: container)

        let usedRect = layoutManager.usedRect(for: container)
        return NSSize(width: ceil(usedRect.width) + 2, height: ceil(usedRect.height) + 2)
    }
}

enum NativeCodeHighlighter {
    static func highlightedCode(
        _ code: String,
        language: String?,
        font: NSFont,
        lineSpacing: CGFloat,
        searchQuery: String
    ) -> NSAttributedString {
        let result = NSMutableAttributedString(string: code)
        let fullRange = NSRange(location: 0, length: result.length)
        let paragraphStyle = NSMutableParagraphStyle()
        paragraphStyle.lineSpacing = lineSpacing
        paragraphStyle.paragraphSpacing = 0
        paragraphStyle.defaultTabInterval = max(24, font.maximumAdvancement.width * 4)

        result.addAttributes([
            .font: font,
            .foregroundColor: NativeCodeTheme.foreground,
            .paragraphStyle: paragraphStyle,
            .ligature: 0,
        ], range: fullRange)

        applySyntaxHighlighting(to: result, language: language)
        applySearchHighlighting(to: result, query: searchQuery)
        return result
    }

    private static func applySyntaxHighlighting(to result: NSMutableAttributedString, language: String?) {
        guard let highlights = try? TreeSitterSemanticHighlighter.highlightRanges(
            in: result.string,
            language: language
        ) else {
            return
        }

        for highlight in highlights {
            result.addAttribute(
                .foregroundColor,
                value: tokenColor(for: highlight.name),
                range: highlight.range
            )
        }
    }

    private static func tokenColor(for name: TreeSitterSemanticTokenName) -> NSColor {
        switch name {
        case .comment:
            NativeCodeTheme.comment
        case .function:
            NativeCodeTheme.function
        case .keyword:
            NativeCodeTheme.keyword
        case .number:
            NativeCodeTheme.number
        case .parameter:
            NativeCodeTheme.foreground
        case .string:
            NativeCodeTheme.string
        case .type:
            NativeCodeTheme.type
        }
    }

    private static func applySearchHighlighting(to result: NSMutableAttributedString, query: String) {
        guard !query.isEmpty else { return }
        let matches = MarkdownSearch.matches(in: result.string, query: query)
        for match in matches {
            let range = NSRange(location: match.offset, length: match.length)
            result.addAttributes([
                .backgroundColor: NSColor(MdowStyle.searchHighlight),
                .foregroundColor: NSColor(MdowStyle.searchHighlightForeground),
            ], range: range)
        }
    }
}

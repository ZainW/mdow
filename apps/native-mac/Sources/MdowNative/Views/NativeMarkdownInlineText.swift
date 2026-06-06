import AppKit
import MdowNativeCore
import SwiftUI

struct NativeMarkdownInlineText: NSViewRepresentable {
    @Environment(\.openURL) private var openURL

    let attributedText: AttributedString
    let font: NSFont
    let codeFont: NSFont
    let lineSpacing: CGFloat

    func makeNSView(context: Context) -> IntrinsicMarkdownTextView {
        let textView = IntrinsicMarkdownTextView()
        textView.delegate = context.coordinator
        textView.onOpenLink = context.coordinator.open(link:)
        textView.isEditable = false
        textView.isSelectable = true
        textView.drawsBackground = false
        textView.textContainerInset = .zero
        textView.textContainer?.lineFragmentPadding = 0
        textView.textContainer?.widthTracksTextView = true
        textView.isHorizontallyResizable = false
        textView.isVerticallyResizable = true
        textView.linkTextAttributes = [
            .foregroundColor: NSColor.linkColor,
            .underlineStyle: NSUnderlineStyle.single.rawValue,
        ]
        return textView
    }

    func updateNSView(_ textView: IntrinsicMarkdownTextView, context: Context) {
        context.coordinator.openURL = openURL
        textView.onOpenLink = context.coordinator.open(link:)
        textView.textStorage?.setAttributedString(nativeAttributedString())
        textView.invalidateIntrinsicContentSize()
    }

    func makeCoordinator() -> Coordinator {
        Coordinator(openURL: openURL)
    }

    private func nativeAttributedString() -> NSAttributedString {
        let result = NSMutableAttributedString(attributedString: NSAttributedString(attributedText))
        let fullRange = NSRange(location: 0, length: result.length)
        let paragraphStyle = NSMutableParagraphStyle()
        paragraphStyle.lineSpacing = lineSpacing
        result.addAttributes([
            .font: font,
            .paragraphStyle: paragraphStyle,
            .foregroundColor: NSColor.labelColor,
        ], range: fullRange)
        result.enumerateAttribute(
            .inlinePresentationIntent,
            in: fullRange
        ) { value, range, _ in
            guard containsInlineIntent(value, .code) else {
                return
            }

            let inlineCodeStyle = MarkdownInlineCodeStyle.standard
            let scaledCodeFont = NSFontManager.shared.convert(
                codeFont,
                toSize: max(10, font.pointSize * inlineCodeStyle.fontScale)
            )
            result.addAttributes([
                .font: scaledCodeFont,
                .backgroundColor: inlineCodeBackgroundColor(),
                .foregroundColor: NSColor.labelColor,
            ], range: range)
        }
        result.enumerateAttribute(
            .inlinePresentationIntent,
            in: fullRange
        ) { value, range, _ in
            guard containsInlineIntent(value, .strikethrough) else {
                return
            }

            result.addAttribute(
                .strikethroughStyle,
                value: NSUnderlineStyle.single.rawValue,
                range: range
            )
        }
        result.enumerateAttribute(
            .link,
            in: fullRange
        ) { value, range, _ in
            guard isFootnoteLink(value) else {
                return
            }

            result.addAttributes([
                .font: NSFont.systemFont(ofSize: max(9, font.pointSize * 0.72)),
                .baselineOffset: max(2, font.pointSize * 0.34),
            ], range: range)
        }
        return result
    }

    private func containsInlineIntent(_ value: Any?, _ intent: InlinePresentationIntent) -> Bool {
        if let presentationIntent = value as? InlinePresentationIntent {
            return presentationIntent.contains(intent)
        }
        let rawIntent = Int(intent.rawValue)
        if let number = value as? NSNumber {
            return number.intValue & rawIntent != 0
        }
        if let rawValue = value as? Int {
            return rawValue & rawIntent != 0
        }
        return false
    }

    private func isFootnoteLink(_ value: Any?) -> Bool {
        if let url = value as? URL {
            return url.fragment?.hasPrefix("fn-") == true
                || url.absoluteString.hasPrefix("#fn-")
                || url.absoluteString.contains("#fn-")
        }
        if let href = value as? String {
            return href.hasPrefix("#fn-") || href.contains("#fn-")
        }
        return false
    }

    private func inlineCodeBackgroundColor() -> NSColor {
        NSColor(name: nil) { appearance in
            let best = appearance.bestMatch(from: [.darkAqua, .aqua])
            if best == .darkAqua {
                return NSColor(red: 0.200, green: 0.200, blue: 0.200, alpha: 1)
            }
            return NSColor(red: 0.945, green: 0.935, blue: 0.900, alpha: 1)
        }
    }

    final class Coordinator: NSObject, NSTextViewDelegate {
        var openURL: OpenURLAction

        init(openURL: OpenURLAction) {
            self.openURL = openURL
        }

        func textView(
            _ textView: NSTextView,
            clickedOnLink link: Any,
            at charIndex: Int
        ) -> Bool {
            open(link: link)
        }

        func open(link: Any) -> Bool {
            if let url = link as? URL {
                dispatch(url)
                return true
            }

            if let href = link as? String, let url = URL(string: href) {
                dispatch(url)
                return true
            }

            return false
        }

        private func dispatch(_ url: URL) {
            Task { @MainActor [openURL] in
                openURL(url)
            }
        }
    }
}

final class IntrinsicMarkdownTextView: NSTextView {
    var onOpenLink: ((Any) -> Bool)?

    override var intrinsicContentSize: NSSize {
        guard let layoutManager, let textContainer else {
            return NSSize(width: NSView.noIntrinsicMetric, height: 0)
        }

        layoutManager.ensureLayout(for: textContainer)
        let usedRect = layoutManager.usedRect(for: textContainer)
        return NSSize(width: NSView.noIntrinsicMetric, height: ceil(usedRect.height))
    }

    override func layout() {
        super.layout()
        invalidateIntrinsicContentSize()
    }

    override func mouseDown(with event: NSEvent) {
        guard openLink(at: event) else {
            super.mouseDown(with: event)
            return
        }
    }

    private func openLink(at event: NSEvent) -> Bool {
        guard let textContainer,
              let layoutManager else {
            return false
        }

        let localPoint = convert(event.locationInWindow, from: nil)
        let containerOrigin = textContainerOrigin
        let containerPoint = NSPoint(
            x: localPoint.x - containerOrigin.x,
            y: localPoint.y - containerOrigin.y
        )
        let glyphIndex = layoutManager.glyphIndex(
            for: containerPoint,
            in: textContainer,
            fractionOfDistanceThroughGlyph: nil
        )
        guard glyphIndex < layoutManager.numberOfGlyphs else {
            return false
        }

        let glyphRect = layoutManager.boundingRect(
            forGlyphRange: NSRange(location: glyphIndex, length: 1),
            in: textContainer
        )
        guard glyphRect.insetBy(dx: -3, dy: -4).contains(containerPoint) else {
            return false
        }

        let characterIndex = layoutManager.characterIndexForGlyph(at: glyphIndex)
        guard characterIndex < string.utf16.count,
              let link = textStorage?.attribute(.link, at: characterIndex, effectiveRange: nil) else {
            return false
        }

        return onOpenLink?(link) == true
    }
}

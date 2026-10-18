import AppKit
import Markdown

public struct OutlineHeading: Sendable, Equatable {
  public var level: Int
  public var title: String
  public var anchor: String
  /// Where the heading starts in the rendered text.
  public var location: Int
}

/// What the renderer knows about an image or diagram when it reaches it.
public enum AssetState: @unchecked Sendable {
  case ready(NSImage)
  case pending
  case failed
}

/// Where the renderer gets images and diagrams. Anything `.pending` is listed in the result,
/// and the document is rendered again once it arrives.
public struct RenderAssets: Sendable {
  public var image: @Sendable (URL) -> AssetState
  public var diagram: @Sendable (String) -> AssetState

  public init(
    image: @escaping @Sendable (URL) -> AssetState,
    diagram: @escaping @Sendable (String) -> AssetState
  ) {
    self.image = image
    self.diagram = diagram
  }

  public static let none = RenderAssets(image: { _ in .pending }, diagram: { _ in .pending })
}

public struct RenderedDocument: @unchecked Sendable {
  public var text: NSAttributedString
  public var headings: [OutlineHeading]
  /// The frontmatter title, else the first top-level heading.
  public var title: String?
  public var pendingImages: [URL]
  public var pendingDiagrams: [String]
}

/// Custom link scheme for in-document anchors, so they can't be confused with relative paths.
public let anchorScheme = "x-mdow-anchor"

public func anchorURL(_ anchor: String) -> URL? {
  var components = URLComponents()
  components.scheme = anchorScheme
  components.path = anchor
  return components.url
}

/// Renders markdown into an attributed string laid out with TextKit's own building blocks:
/// paragraph styles, `NSTextBlock`s for code and quotes, `NSTextTable` for tables and
/// `NSTextAttachment` for images, so selection, find, copy and printing all work natively.
public final class MarkdownRenderer {
  public static func render(
    _ source: MarkdownSource, style: ReaderStyle, baseURL: URL?, assets: RenderAssets = .none
  ) -> RenderedDocument {
    // Math first, so footnote definitions keep their formulas and TeX can't look like a note.
    let (withoutMath, math) = MathSpans.extract(from: source.body)
    let (body, footnotes) = Footnotes.extract(from: withoutMath)
    let document = Document(parsing: body, options: [.disableSmartOpts])
    let renderer = MarkdownRenderer(
      style: style, baseURL: baseURL, assets: assets, footnotes: footnotes, math: math)
    renderer.renderBlocks(document.children, renderer.rootContext)
    renderer.renderFootnotes()
    return RenderedDocument(
      text: renderer.out, headings: renderer.headings, title: source.title ?? renderer.firstTitle,
      pendingImages: renderer.pendingImages, pendingDiagrams: renderer.pendingDiagrams)
  }

  struct Context {
    var blocks: [NSTextBlock] = []
    var indent: CGFloat = 0
    var spacing: CGFloat
    var color: NSColor = ReaderColors.text
    var listDepth = 0
  }

  struct Inline {
    var size: CGFloat
    var weight: NSFont.Weight
    var italic = false
    var code = false
    var strike = false
    var underline = false
    var superscript = 0
    var link: URL?
    var color: NSColor
  }

  struct Marker {
    var text: NSAttributedString
    var start: CGFloat
    var indent: CGFloat
  }

  let style: ReaderStyle
  let baseURL: URL?
  let assets: RenderAssets
  let footnotes: Footnotes
  /// The TeX behind each math placeholder in the parsed text.
  let math: MathSpans
  let out = NSMutableAttributedString()
  var headings: [OutlineHeading] = []
  var firstTitle: String?
  var pendingImages: [URL] = []
  var pendingDiagrams: [String] = []
  private var slugger = Slugger()
  private var marker: Marker?
  private var fonts: [FontKey: NSFont] = [:]
  private var assetSlots = 0
  // Looking fonts up by descriptor is slow (Charter especially), so the common ones are made once.
  lazy var bodyFont = style.body()
  lazy var codeFont = style.code()
  lazy var boldCodeFont = style.code(weight: .semibold)
  lazy var digitFont = NSFont.monospacedDigitSystemFont(ofSize: style.bodySize, weight: .regular)

  init(
    style: ReaderStyle, baseURL: URL?, assets: RenderAssets, footnotes: Footnotes,
    math: MathSpans = MathSpans()
  ) {
    self.style = style
    self.baseURL = baseURL
    self.assets = assets
    self.footnotes = footnotes
    self.math = math
  }

  var rootContext: Context { Context(spacing: style.blockSpacing) }

  // MARK: - Blocks

  func renderBlocks(_ children: MarkupChildren, _ ctx: Context) {
    for child in children { renderBlock(child, ctx) }
  }

  func renderBlock(_ markup: Markup, _ ctx: Context) {
    switch markup {
    case let heading as Heading:
      renderHeading(heading, ctx)
    case let paragraph as Paragraph:
      renderParagraph(paragraph, ctx)
    case let code as CodeBlock:
      renderCode(math.restoringSource(code.code), info: code.language, ctx)
    case let quote as BlockQuote:
      renderQuote(quote, ctx)
    case let list as UnorderedList:
      renderList(Array(list.listItems), ordered: false, start: 1, ctx)
    case let list as OrderedList:
      renderList(Array(list.listItems), ordered: true, start: Int(list.startIndex), ctx)
    case let table as Table:
      renderTable(table, ctx)
    case is ThematicBreak:
      renderRule(ctx)
    case let html as HTMLBlock:
      renderHTMLBlock(math.restoringSource(html.rawHTML), ctx)
    default:
      renderBlocks(markup.children, ctx)
    }
  }

  func baseInline(_ ctx: Context) -> Inline {
    Inline(size: style.bodySize, weight: .regular, color: ctx.color)
  }

  func paragraphStyle(_ ctx: Context, spacing: CGFloat? = nil) -> NSMutableParagraphStyle {
    let paragraph = NSMutableParagraphStyle()
    paragraph.textBlocks = ctx.blocks
    paragraph.lineHeightMultiple = style.lineSpacing.multiple
    paragraph.paragraphSpacing = spacing ?? ctx.spacing
    paragraph.headIndent = ctx.indent
    paragraph.firstLineHeadIndent = ctx.indent
    return paragraph
  }

  /// Appends one paragraph, putting the pending list marker in front of it if there is one.
  func append(_ content: NSAttributedString, _ paragraph: NSMutableParagraphStyle) {
    let line = NSMutableAttributedString()
    if let marker {
      paragraph.firstLineHeadIndent = marker.start
      paragraph.tabStops = [
        NSTextTab(textAlignment: .right, location: marker.indent - 7),
        NSTextTab(textAlignment: .left, location: marker.indent),
      ]
      paragraph.defaultTabInterval = 28
      line.append(NSAttributedString(string: "\t"))
      line.append(marker.text)
      line.append(NSAttributedString(string: "\t"))
      self.marker = nil
    }
    line.append(content)
    var newlineAttributes =
      content.length > 0 ? content.attributes(at: content.length - 1, effectiveRange: nil) : [:]
    // Run-level styling on the newline would paint across the rest of the line.
    for key: NSAttributedString.Key in [
      .backgroundColor, .link, .underlineStyle, .strikethroughStyle, .toolTip, .attachment,
      .baselineOffset,
    ] {
      newlineAttributes[key] = nil
    }
    line.append(NSAttributedString(string: "\n", attributes: newlineAttributes))
    if line.length > 0, line.attribute(.font, at: 0, effectiveRange: nil) == nil {
      line.addAttribute(.font, value: bodyFont, range: NSRange(location: 0, length: 1))
    }
    line.addAttribute(.paragraphStyle, value: paragraph, range: NSRange(location: 0, length: line.length))
    out.append(line)
  }

  /// Emits a list marker on its own line when an item starts with something that can't carry it.
  func flushMarker(_ ctx: Context) {
    guard marker != nil else { return }
    append(NSAttributedString(), paragraphStyle(ctx, spacing: 2))
  }

  /// Sets the space after the last paragraph since `start`, unless that paragraph sits inside a
  /// nested block, whose own margin already spaces it.
  func setTrailingSpacing(from start: Int, _ spacing: CGFloat, depth: Int) {
    guard out.length > start else { return }
    let last = out.length - 1
    guard
      let current = out.attribute(.paragraphStyle, at: last, effectiveRange: nil)
        as? NSParagraphStyle, current.textBlocks.count == depth,
      let paragraph = current.mutableCopy() as? NSMutableParagraphStyle
    else { return }
    paragraph.paragraphSpacing = spacing
    let range = out.mutableString.paragraphRange(for: NSRange(location: last, length: 0))
    out.addAttribute(.paragraphStyle, value: paragraph, range: range)
  }

  func renderHeading(_ heading: Heading, _ ctx: Context) {
    let level = min(max(heading.level, 1), 6)
    let size = style.headingSize(level)
    var inline = baseInline(ctx)
    inline.size = size
    inline.weight = level <= 2 ? .bold : .semibold
    if level == 6 { inline.color = ReaderColors.secondary }
    let text = NSMutableAttributedString()
    renderInlines(heading.children, inline, into: text)

    let title = math.replacingWithTeX(heading.plainText).trimmingCharacters(in: .whitespaces)
    let anchor = slugger.next(title)
    if level == 1, firstTitle == nil, ctx.blocks.isEmpty { firstTitle = title }
    text.addAttribute(.mdowAnchor, value: anchor, range: NSRange(location: 0, length: text.length))
    if ctx.blocks.isEmpty, ctx.listDepth == 0 {
      headings.append(OutlineHeading(level: level, title: title, anchor: anchor, location: out.length))
    }

    let paragraph = paragraphStyle(ctx, spacing: (size * 0.4).rounded())
    paragraph.paragraphSpacingBefore = out.length == 0 ? 0 : (size * (level <= 2 ? 0.95 : 0.75)).rounded()
    paragraph.lineHeightMultiple = 1.12
    append(text, paragraph)
  }

  func renderParagraph(_ paragraph: Paragraph, _ ctx: Context) {
    let text = NSMutableAttributedString()
    renderInlines(paragraph.children, baseInline(ctx), into: text, indent: ctx.indent)
    if !math.spans.isEmpty, splitDisplayMath(text, ctx) { return }
    append(text, paragraphStyle(ctx))
  }

  // MARK: Code and diagrams

  func renderCode(_ code: String, info: String?, _ ctx: Context) {
    flushMarker(ctx)
    var code = code
    if code.hasSuffix("\n") { code.removeLast() }
    let language = SyntaxHighlighter.language(for: info)
    let infoWord = info?.split(separator: " ").first.map { $0.lowercased() }
    if infoWord == "mermaid" {
      // Whatever the diagram turns out as (drawn, pending or shown as code) is one asset slot.
      let slot = nextAssetSlot()
      let start = out.length
      defer {
        out.addAttribute(
          .mdowAssetSlot, value: slot, range: NSRange(location: start, length: out.length - start))
      }
      if renderDiagram(code, ctx) { return }
    }
    if infoWord == "math", renderDisplayMath(code, ctx) { return }

    let block = CodeTextBlock()
    block.setWidth(14, type: .absoluteValueType, for: .padding)
    block.setWidth(12, type: .absoluteValueType, for: .padding, edge: .minY)
    block.setWidth(12, type: .absoluteValueType, for: .padding, edge: .maxY)
    block.setWidth(style.blockSpacing, type: .absoluteValueType, for: .margin, edge: .maxY)
    block.setWidth(ctx.indent, type: .absoluteValueType, for: .margin, edge: .minX)
    if let language {
      block.label = SyntaxHighlighter.displayName(for: language)
    } else if let infoWord, !infoWord.isEmpty {
      block.label = infoWord
    }

    let font = codeFont
    let text = NSMutableAttributedString(
      string: code.isEmpty ? " " : code,
      attributes: [.font: font, .foregroundColor: ReaderColors.text])
    if code.utf16.count < 400_000 {
      let bold = boldCodeFont
      for token in SyntaxHighlighter.tokens(in: code, language: language) {
        guard NSMaxRange(token.range) <= text.length else { continue }
        text.addAttribute(.foregroundColor, value: CodeTheme.color(for: token.kind), range: token.range)
        if CodeTheme.isBold(token.kind) { text.addAttribute(.font, value: bold, range: token.range) }
      }
    }
    text.append(NSAttributedString(string: "\n", attributes: [.font: font]))
    let paragraph = NSMutableParagraphStyle()
    paragraph.textBlocks = ctx.blocks + [block]
    paragraph.lineHeightMultiple = 1.18
    paragraph.lineBreakMode = .byWordWrapping
    let whole = NSRange(location: 0, length: text.length)
    text.addAttribute(.paragraphStyle, value: paragraph, range: whole)
    text.addAttribute(.mdowCodeBlock, value: CodeBlockInfo(code: code, language: language), range: whole)
    out.append(text)
  }

  /// Draws a Mermaid diagram, or returns false to fall back to showing its source.
  func renderDiagram(_ source: String, _ ctx: Context) -> Bool {
    let paragraph = paragraphStyle(ctx, spacing: style.blockSpacing * 1.4)
    paragraph.alignment = .center
    paragraph.lineHeightMultiple = 1
    switch assets.diagram(source) {
    case .failed:
      return false
    case .pending:
      pendingDiagrams.append(source)
      let placeholder = NSAttributedString(
        string: "Drawing diagram…",
        attributes: [.font: bodyFont, .foregroundColor: ReaderColors.tertiary])
      append(placeholder, paragraph)
    case .ready(let image):
      let attachment = NSTextAttachment()
      attachment.image = image
      let size = fitted(image.size, maxWidth: style.maxContentWidth - ctx.indent)
      attachment.bounds = CGRect(origin: .zero, size: size)
      append(NSAttributedString(attachment: attachment), paragraph)
    }
    return true
  }

  // MARK: Quotes and alerts

  enum AlertKind: String, CaseIterable {
    case note, tip, important, warning, caution

    var title: String { rawValue.capitalized }

    /// The desktop app's alert colours.
    var color: NSColor {
      switch self {
      case .note: Palette.pair(0x2D6AC0, 0x74A6EE)
      case .tip: Palette.pair(0x2B8150, 0x68BD88)
      case .important: Palette.pair(0x7A4CC0, 0xAE8CEE)
      case .warning: Palette.pair(0x9A6512, 0xE2A852)
      case .caution: Palette.pair(0xC0403A, 0xEE7C70)
      }
    }

    var symbol: String {
      switch self {
      case .note: "info.circle"
      case .tip: "lightbulb"
      case .important: "exclamationmark.bubble"
      case .warning: "exclamationmark.triangle"
      case .caution: "exclamationmark.octagon"
      }
    }
  }

  func alertKind(_ quote: BlockQuote) -> AlertKind? {
    guard let first = quote.child(at: 0) as? Paragraph else { return nil }
    let text = first.plainText
    guard text.hasPrefix("[!"), let close = text.firstIndex(of: "]") else { return nil }
    let name = text[text.index(text.startIndex, offsetBy: 2)..<close].lowercased()
    return AlertKind(rawValue: name)
  }

  func renderQuote(_ quote: BlockQuote, _ ctx: Context) {
    flushMarker(ctx)
    let alert = alertKind(quote)
    let block = QuoteTextBlock()
    block.setWidth(alert == nil ? 18 : 18, type: .absoluteValueType, for: .padding, edge: .minX)
    block.setWidth(alert == nil ? 2 : 12, type: .absoluteValueType, for: .padding, edge: .minY)
    block.setWidth(alert == nil ? 2 : 12, type: .absoluteValueType, for: .padding, edge: .maxY)
    block.setWidth(alert == nil ? 0 : 16, type: .absoluteValueType, for: .padding, edge: .maxX)
    block.setWidth(style.blockSpacing, type: .absoluteValueType, for: .margin, edge: .maxY)
    block.setWidth(ctx.indent, type: .absoluteValueType, for: .margin, edge: .minX)

    var inner = ctx
    inner.blocks.append(block)
    inner.indent = 0
    inner.spacing = style.blockSpacing * 0.75
    let start = out.length

    if let alert {
      block.barColor = alert.color
      block.fillColor = ReaderColors.dynamic(
        light: alert.color.withAlphaComponent(0.06), dark: alert.color.withAlphaComponent(0.09))
      let title = NSMutableAttributedString()
      if let symbol = NSImage(systemSymbolName: alert.symbol, accessibilityDescription: nil) {
        let config = NSImage.SymbolConfiguration(pointSize: style.bodySize * 0.9, weight: .semibold)
          .applying(.init(paletteColors: [alert.color]))
        let attachment = NSTextAttachment()
        attachment.image = symbol.withSymbolConfiguration(config)
        title.append(NSAttributedString(attachment: attachment))
        title.append(NSAttributedString(string: " "))
      }
      title.append(
        NSAttributedString(
          string: alert.title,
          attributes: [
            .font: style.code(size: (style.bodySize * 0.78).rounded(), weight: .semibold),
            .foregroundColor: alert.color, .kern: 0.2,
          ]))
      append(title, paragraphStyle(inner, spacing: 4))

      for (index, child) in quote.children.enumerated() {
        guard index == 0, let paragraph = child as? Paragraph else {
          renderBlock(child, inner)
          continue
        }
        let text = NSMutableAttributedString()
        renderInlines(paragraph.children, baseInline(inner), into: text)
        if let close = text.string.firstIndex(of: "]") {
          var cut = text.string.utf16.distance(
            from: text.string.utf16.startIndex, to: close.samePosition(in: text.string.utf16)!) + 1
          let units = Array(text.string.utf16)
          while cut < units.count, units[cut] == 32 || units[cut] == 10 || units[cut] == 0x2028 {
            cut += 1
          }
          text.deleteCharacters(in: NSRange(location: 0, length: cut))
        }
        if text.length > 0 { append(text, paragraphStyle(inner)) }
      }
    } else {
      renderBlocks(quote.children, inner)
    }
    if out.length == start { append(NSAttributedString(string: " "), paragraphStyle(inner)) }
    setTrailingSpacing(from: start, 0, depth: inner.blocks.count)
  }

  // MARK: Lists

  func isTight(_ items: [ListItem]) -> Bool {
    for (index, item) in items.enumerated() {
      if item.childCount > 1, item.children.contains(where: { $0 is Paragraph }),
        item.children.filter({ $0 is Paragraph }).count > 1
      {
        return false
      }
      if index + 1 < items.count, let end = item.range?.upperBound.line,
        let next = items[index + 1].range?.lowerBound.line, next > end + 1
      {
        return false
      }
    }
    return true
  }

  func renderList(_ items: [ListItem], ordered: Bool, start: Int, _ ctx: Context) {
    flushMarker(ctx)
    let listStart = out.length
    let tight = isTight(items)
    let digits = String(start + items.count - 1).count
    let width: CGFloat = ordered ? max(26, CGFloat(digits) * style.bodySize * 0.6 + 14) : 22
    let markerFont = ordered ? digitFont : bodyFont

    var inner = ctx
    inner.listDepth += 1
    inner.indent = ctx.indent + width
    inner.spacing = tight ? (style.bodySize * 0.25).rounded() : style.blockSpacing

    for (index, item) in items.enumerated() {
      let text: NSAttributedString
      if let checkbox = item.checkbox {
        text = checkboxMarker(checked: checkbox == .checked)
      } else if ordered {
        text = NSAttributedString(
          string: "\(start + index).",
          attributes: [.font: markerFont, .foregroundColor: ReaderColors.secondary])
      } else {
        let bullets = ["•", "◦", "▪︎"]
        text = NSAttributedString(
          string: bullets[min(ctx.listDepth, bullets.count - 1)],
          attributes: [.font: markerFont, .foregroundColor: ReaderColors.secondary])
      }
      marker = Marker(text: text, start: ctx.indent, indent: inner.indent)
      renderBlocks(item.children, inner)
      if marker != nil { append(NSAttributedString(), paragraphStyle(inner)) }
    }
    setTrailingSpacing(from: listStart, ctx.spacing, depth: ctx.blocks.count)
  }

  func checkboxMarker(checked: Bool) -> NSAttributedString {
    let name = checked ? "checkmark.square.fill" : "square"
    guard let symbol = NSImage(systemSymbolName: name, accessibilityDescription: checked ? "Done" : "To do")
    else { return NSAttributedString(string: checked ? "☑" : "☐") }
    let color: NSColor = checked ? Palette.accent : ReaderColors.tertiary
    let config = NSImage.SymbolConfiguration(pointSize: style.bodySize * 0.95, weight: .regular)
      .applying(.init(paletteColors: [checked ? .white : color, color]))
    let attachment = NSTextAttachment()
    attachment.image = symbol.withSymbolConfiguration(config)
    if let image = attachment.image {
      attachment.bounds = CGRect(
        x: 0, y: bodyFont.descender + 1, width: image.size.width, height: image.size.height)
    }
    return NSAttributedString(attachment: attachment)
  }

  // MARK: Tables

  func renderTable(_ table: Table, _ ctx: Context) {
    flushMarker(ctx)
    let columns = max(table.maxColumnCount, 1)
    let textTable = NSTextTable()
    textTable.numberOfColumns = columns
    textTable.collapsesBorders = true
    textTable.hidesEmptyCells = false
    textTable.setContentWidth(100, type: .percentageValueType)
    textTable.setWidth(style.blockSpacing, type: .absoluteValueType, for: .margin, edge: .maxY)
    textTable.setWidth(ctx.indent, type: .absoluteValueType, for: .margin, edge: .minX)

    var rows: [(cells: [Table.Cell], header: Bool)] = [(Array(table.head.cells), true)]
    for row in table.body.rows { rows.append((Array(row.cells), false)) }

    for (rowIndex, row) in rows.enumerated() {
      var column = 0
      for cell in row.cells {
        guard cell.colspan > 0, column < columns else { continue }
        let span = min(Int(cell.colspan), columns - column)
        let block = NSTextTableBlock(
          table: textTable, startingRow: rowIndex, rowSpan: max(Int(cell.rowspan), 1),
          startingColumn: column, columnSpan: span)
        // Rules between rows only, with the outer columns flush with the text around them.
        let isLastRow = rowIndex == rows.count - 1
        if !isLastRow {
          block.setWidth(row.header ? 1 : 0.5, type: .absoluteValueType, for: .border, edge: .maxY)
          block.setBorderColor(row.header ? ReaderColors.tableBorder : ReaderColors.separator)
        }
        if isLastRow {
          block.setWidth(style.blockSpacing, type: .absoluteValueType, for: .margin, edge: .maxY)
        }
        block.setWidth(7, type: .absoluteValueType, for: .padding, edge: .minY)
        block.setWidth(7, type: .absoluteValueType, for: .padding, edge: .maxY)
        // 2pt rather than 0 at the edges leaves room for inline code's rounded background.
        block.setWidth(column == 0 ? 2 : 10, type: .absoluteValueType, for: .padding, edge: .minX)
        block.setWidth(
          column + span >= columns ? 2 : 10, type: .absoluteValueType, for: .padding, edge: .maxX)

        var inline = baseInline(ctx)
        if row.header { inline.weight = .semibold }
        let text = NSMutableAttributedString()
        renderInlines(cell.children, inline, into: text)

        var cellContext = ctx
        cellContext.blocks.append(block)
        cellContext.indent = 0
        let paragraph = paragraphStyle(cellContext, spacing: 0)
        paragraph.lineHeightMultiple = max(1.1, style.lineSpacing.multiple - 0.12)
        let alignments = table.columnAlignments
        switch column < alignments.count ? alignments[column] : nil {
        case .center: paragraph.alignment = .center
        case .right: paragraph.alignment = .right
        default: paragraph.alignment = .natural
        }
        let saved = marker
        marker = nil
        append(text, paragraph)
        marker = saved
        column += span
      }
    }
  }

  func renderRule(_ ctx: Context) {
    flushMarker(ctx)
    let block = RuleTextBlock()
    block.setWidth(style.blockSpacing, type: .absoluteValueType, for: .padding, edge: .minY)
    block.setWidth(style.blockSpacing, type: .absoluteValueType, for: .padding, edge: .maxY)
    block.setWidth(style.blockSpacing, type: .absoluteValueType, for: .margin, edge: .maxY)
    block.setWidth(ctx.indent, type: .absoluteValueType, for: .margin, edge: .minX)
    var inner = ctx
    inner.blocks.append(block)
    inner.indent = 0
    let paragraph = paragraphStyle(inner, spacing: 0)
    paragraph.lineHeightMultiple = 1
    paragraph.minimumLineHeight = 1
    paragraph.maximumLineHeight = 1
    append(
      NSAttributedString(string: "\u{200B}", attributes: [.font: NSFont.systemFont(ofSize: 1)]),
      paragraph)
  }

  // MARK: HTML blocks

  func renderHTMLBlock(_ html: String, _ ctx: Context) {
    let trimmed = html.trimmingCharacters(in: .whitespacesAndNewlines)
    if trimmed.hasPrefix("<!--") { return }
    let images = HTMLSnippet.images(in: trimmed)
    if !images.isEmpty {
      let text = NSMutableAttributedString()
      for (index, image) in images.enumerated() {
        if index > 0 { text.append(NSAttributedString(string: " ")) }
        appendImage(
          source: image.src, alt: image.alt, width: image.width, height: image.height,
          inline: baseInline(ctx), indent: ctx.indent, into: text, link: image.link)
      }
      let paragraph = paragraphStyle(ctx)
      if HTMLSnippet.isCentered(trimmed) { paragraph.alignment = .center }
      append(text, paragraph)
      let rest = HTMLSnippet.text(in: HTMLSnippet.removingImages(trimmed))
      if rest.isEmpty { return }
      renderHTMLText(rest, centered: HTMLSnippet.isCentered(trimmed), ctx)
      return
    }
    renderHTMLText(HTMLSnippet.text(in: trimmed), centered: HTMLSnippet.isCentered(trimmed), ctx)
  }

  func renderHTMLText(_ text: String, centered: Bool, _ ctx: Context) {
    for line in text.components(separatedBy: "\n") {
      let line = line.trimmingCharacters(in: .whitespaces)
      guard !line.isEmpty else { continue }
      let paragraph = paragraphStyle(ctx)
      if centered { paragraph.alignment = .center }
      append(NSAttributedString(string: line, attributes: attributes(baseInline(ctx))), paragraph)
    }
  }

  // MARK: Footnotes

  func renderFootnotes() {
    guard !footnotes.notes.isEmpty else { return }
    renderRule(rootContext)
    var ctx = rootContext
    ctx.spacing = 6
    var inline = baseInline(ctx)
    inline.size = (style.bodySize * 0.88).rounded()
    for note in footnotes.notes {
      let text = NSMutableAttributedString(
        string: "\(note.number). ",
        attributes: attributes(Inline(size: inline.size, weight: .medium, color: ReaderColors.secondary)))
      let parsed = Document(parsing: note.text, options: [.disableSmartOpts])
      var first = true
      for child in parsed.children {
        if !first { text.append(NSAttributedString(string: " ", attributes: attributes(inline))) }
        first = false
        renderInlines(child.children, inline, into: text)
      }
      var back = inline
      back.link = anchorURL("fnref-\(note.id)")
      text.append(NSAttributedString(string: " ↩︎", attributes: attributes(back)))
      text.addAttribute(.mdowAnchor, value: "fn-\(note.id)", range: NSRange(location: 0, length: text.length))
      append(text, paragraphStyle(ctx))
    }
  }

  // MARK: - Inlines

  func renderInlines(
    _ children: MarkupChildren, _ inline: Inline, into text: NSMutableAttributedString,
    indent: CGFloat = 0
  ) {
    var inline = inline
    // Inline HTML arrives as separate open and close tags, so it changes the style of the
    // siblings between them.
    var htmlStack: [(String, Inline)] = []
    for child in children {
      if let node = child as? InlineHTML {
        let rawHTML = math.restoringSource(node.rawHTML)
        let tag = HTMLSnippet.tag(rawHTML)
        switch tag.name {
        case "br":
          text.append(NSAttributedString(string: "\u{2028}", attributes: attributes(inline)))
        case "img":
          if let image = HTMLSnippet.images(in: rawHTML).first {
            appendImage(
              source: image.src, alt: image.alt, width: image.width, height: image.height,
              inline: inline, indent: indent, into: text, link: inline.link)
          }
        case let name? where !tag.closing:
          htmlStack.append((name, inline))
          switch name {
          case "b", "strong": inline.weight = .bold
          case "i", "em": inline.italic = true
          case "code", "kbd", "samp", "tt": inline.code = true
          case "s", "del", "strike": inline.strike = true
          case "u", "ins": inline.underline = true
          case "sup": inline.superscript = 1
          case "sub": inline.superscript = -1
          case "a":
            if let href = HTMLSnippet.attribute("href", in: rawHTML) { inline.link = resolve(href) }
          default: break
          }
        case let name?:
          if let index = htmlStack.lastIndex(where: { $0.0 == name }) {
            inline = htmlStack[index].1
            htmlStack.removeSubrange(index...)
          }
        default:
          break
        }
        continue
      }
      renderInline(child, inline, into: text, indent: indent)
    }
  }

  func renderInline(_ markup: Markup, _ inline: Inline, into text: NSMutableAttributedString, indent: CGFloat) {
    var inline = inline
    switch markup {
    case let node as Markdown.Text:
      appendText(node.string, inline, into: text)
    case is SoftBreak:
      text.append(NSAttributedString(string: " ", attributes: attributes(inline)))
    case is LineBreak:
      text.append(NSAttributedString(string: "\u{2028}", attributes: attributes(inline)))
    case let code as InlineCode:
      inline.code = true
      text.append(
        NSAttributedString(string: math.restoringSource(code.code), attributes: attributes(inline)))
    case let node as Emphasis:
      inline.italic = true
      renderInlines(node.children, inline, into: text, indent: indent)
    case let node as Strong:
      inline.weight = inline.weight == .regular ? .bold : .heavy
      renderInlines(node.children, inline, into: text, indent: indent)
    case let node as Strikethrough:
      inline.strike = true
      renderInlines(node.children, inline, into: text, indent: indent)
    case let link as Markdown.Link:
      if let destination = link.destination { inline.link = resolve(destination) }
      let start = text.length
      renderInlines(link.children, inline, into: text, indent: indent)
      if let destination = link.destination, text.length > start {
        text.addAttribute(.toolTip, value: math.restoringSource(destination), range: NSRange(location: start, length: text.length - start))
      }
    case let image as Markdown.Image:
      appendImage(
        source: image.source ?? "", alt: math.replacingWithTeX(image.plainText), width: nil,
        height: nil, inline: inline,
        indent: indent, into: text, link: inline.link)
    case let html as InlineHTML:
      // Only reached for HTML outside `renderInlines`, e.g. a lone tag.
      if HTMLSnippet.tag(html.rawHTML).name == "br" {
        text.append(NSAttributedString(string: "\u{2028}", attributes: attributes(inline)))
      }
    default:
      renderInlines(markup.children, inline, into: text, indent: indent)
    }
  }

  func appendText(_ string: String, _ inline: Inline, into text: NSMutableAttributedString) {
    guard !math.spans.isEmpty else {
      appendPlainText(string, inline, into: text)
      return
    }
    for piece in math.pieces(string) {
      switch piece {
      case .text(let run): appendPlainText(String(run), inline, into: text)
      case .math(let index): appendMath(math.spans[index], inline, into: text)
      }
    }
  }

  /// Text, with footnote references.
  func appendPlainText(_ string: String, _ inline: Inline, into text: NSMutableAttributedString) {
    guard !footnotes.notes.isEmpty, string.containsBytes("[^") else {
      text.append(NSAttributedString(string: string, attributes: attributes(inline)))
      return
    }
    var rest = string[...]
    while let open = rest.rangeOfBytes("[^"), let close = rest[open.upperBound...].firstIndex(of: "]"),
      let number = footnotes.number(for: String(rest[open.upperBound..<close]))
    {
      let id = String(rest[open.upperBound..<close])
      text.append(NSAttributedString(string: String(rest[..<open.lowerBound]), attributes: attributes(inline)))
      var ref = inline
      ref.superscript = 1
      ref.link = anchorURL("fn-\(id)")
      let start = text.length
      text.append(NSAttributedString(string: "\(number)", attributes: attributes(ref)))
      text.addAttribute(.mdowAnchor, value: "fnref-\(id)", range: NSRange(location: start, length: text.length - start))
      rest = rest[rest.index(after: close)...]
    }
    text.append(NSAttributedString(string: String(rest), attributes: attributes(inline)))
  }

  func appendImage(
    source: String, alt: String, width: CGFloat?, height: CGFloat?, inline: Inline, indent: CGFloat,
    into text: NSMutableAttributedString, link: URL?
  ) {
    guard let url = resolve(source) else { return }
    var placeholder = inline
    placeholder.color = ReaderColors.tertiary
    let slot = nextAssetSlot()
    let start = text.length
    defer {
      text.addAttribute(
        .mdowAssetSlot, value: slot, range: NSRange(location: start, length: text.length - start))
    }
    // An image without alt text still needs a character to hold its slot.
    let altText = alt.isEmpty ? "\u{200B}" : alt
    switch assets.image(url) {
    case .pending:
      pendingImages.append(url)
      text.append(NSAttributedString(string: altText, attributes: attributes(placeholder)))
    case .failed:
      text.append(NSAttributedString(string: altText, attributes: attributes(placeholder)))
    case .ready(let image):
      var size = image.size
      if size.width <= 0 || size.height <= 0 { return }
      if let width, let height {
        size = CGSize(width: width, height: height)
      } else if let width {
        size = CGSize(width: width, height: size.height * width / size.width)
      } else if let height {
        size = CGSize(width: size.width * height / size.height, height: height)
      }
      size = fitted(size, maxWidth: style.maxContentWidth - indent)
      let attachment = NSTextAttachment()
      attachment.image = image
      let font = font(for: inline)
      // Small images (badges, icons) sit on the text's baseline like a glyph would.
      let y = size.height < font.pointSize * 2.2 ? (font.descender + (font.capHeight - size.height) / 2 + 2).rounded() : 0
      attachment.bounds = CGRect(x: 0, y: min(0, y), width: size.width, height: size.height)
      var attributes: [NSAttributedString.Key: Any] = [.attachment: attachment]
      if let link { attributes[.link] = link }
      if !alt.isEmpty { attributes[.toolTip] = alt }
      text.append(NSAttributedString(string: "\u{FFFC}", attributes: attributes))
    }
  }

  func nextAssetSlot() -> Int {
    assetSlots += 1
    return assetSlots
  }

  func fitted(_ size: CGSize, maxWidth: CGFloat) -> CGSize {
    guard size.width > maxWidth, maxWidth > 0 else { return size }
    return CGSize(width: maxWidth, height: (size.height * maxWidth / size.width).rounded())
  }

  /// Resolves a link or image path against the document's folder.
  func resolve(_ destination: String) -> URL? {
    let destination = math.restoringSource(destination).trimmingCharacters(in: .whitespaces)
    if destination.hasPrefix("#") { return anchorURL(String(destination.dropFirst())) }
    if let url = URL(string: destination), url.scheme != nil { return url }
    // Already-encoded paths (e.g. `my%20file.md`) are decoded first so they aren't encoded twice.
    let parts = destination.split(separator: "#", maxSplits: 1, omittingEmptySubsequences: false)
    let path = String(parts[0]).removingPercentEncoding ?? String(parts[0])
    var encoded = path.addingPercentEncoding(withAllowedCharacters: .urlPathAllowed) ?? path
    if parts.count > 1 {
      encoded += "#" + (String(parts[1]).addingPercentEncoding(withAllowedCharacters: .urlFragmentAllowed) ?? "")
    }
    if destination.hasPrefix("/") { return URL(string: "file://" + encoded) }
    return URL(string: encoded, relativeTo: baseURL)?.absoluteURL
  }

  // MARK: Attributes

  struct FontKey: Hashable {
    var size: CGFloat
    var weight: CGFloat
    var italic: Bool
    var code: Bool
  }

  func font(for inline: Inline) -> NSFont {
    let size = inline.superscript != 0 ? (inline.size * 0.75).rounded() : inline.size
    let key = FontKey(size: size, weight: inline.weight.rawValue, italic: inline.italic, code: inline.code)
    if let font = fonts[key] { return font }
    var font =
      inline.code
      ? style.code(size: (size * 0.88).rounded(), weight: inline.weight == .regular ? .regular : .semibold)
      : style.bodyFont.font(size: size, weight: inline.weight)
    if inline.italic {
      let descriptor = font.fontDescriptor.withSymbolicTraits(
        font.fontDescriptor.symbolicTraits.union(.italic))
      font = NSFont(descriptor: descriptor, size: size) ?? font
    }
    fonts[key] = font
    return font
  }

  func attributes(_ inline: Inline) -> [NSAttributedString.Key: Any] {
    var attributes: [NSAttributedString.Key: Any] = [
      .font: font(for: inline), .foregroundColor: inline.color,
    ]
    if inline.code { attributes[.backgroundColor] = ReaderColors.inlineCodeBackground }
    if inline.strike { attributes[.strikethroughStyle] = NSUnderlineStyle.single.rawValue }
    if inline.underline { attributes[.underlineStyle] = NSUnderlineStyle.single.rawValue }
    if inline.superscript != 0 {
      attributes[.baselineOffset] = inline.superscript > 0 ? inline.size * 0.35 : -inline.size * 0.15
    }
    if let link = inline.link { attributes[.link] = link }
    return attributes
  }
}

private extension Table {
  var maxColumnCount: Int {
    var count = head.cells.reduce(0) { $0 + max(Int($1.colspan), 1) }
    for row in body.rows {
      count = max(count, row.cells.reduce(0) { $0 + max(Int($1.colspan), 1) })
    }
    return count
  }
}

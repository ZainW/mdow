import AppKit

/// Margins are outside the drawn area; the frame AppKit hands blocks includes them.
private func contentFrame(of block: NSTextBlock, _ frame: NSRect) -> NSRect {
  let left = block.width(for: .margin, edge: .minX)
  let right = block.width(for: .margin, edge: .maxX)
  let top = block.width(for: .margin, edge: .minY)
  let bottom = block.width(for: .margin, edge: .maxY)
  return NSRect(
    x: frame.minX + left, y: frame.minY + top,
    width: frame.width - left - right, height: frame.height - top - bottom)
}

/// A fenced code block: a rounded, softly filled card with the language in the corner.
public final class CodeTextBlock: NSTextBlock {
  public var label: String?
  public var labelFont = NSFont.systemFont(ofSize: 10.5, weight: .medium)
  /// Set while the pointer is over the block, so the label makes room for the copy button.
  public var hidesLabel = false

  public override init() {
    super.init()
    // AppKit only asks a block to draw its background when it has one.
    backgroundColor = .clear
    // Without a width a block shrinks to its narrowest, wrapping after every glyph.
    setContentWidth(100, type: .percentageValueType)
  }

  required init?(coder: NSCoder) {
    super.init(coder: coder)
  }

  public override func drawBackground(
    withFrame frameRect: NSRect, in controlView: NSView?, characterRange charRange: NSRange,
    layoutManager: NSLayoutManager
  ) {
    let rect = contentFrame(of: self, frameRect)
    let path = NSBezierPath(roundedRect: rect.insetBy(dx: 0.5, dy: 0.5), xRadius: 8, yRadius: 8)
    ReaderColors.codeBackground.setFill()
    path.fill()
    ReaderColors.codeBorder.setStroke()
    path.lineWidth = 1
    path.stroke()

    guard let label, !hidesLabel else { return }
    let attributes: [NSAttributedString.Key: Any] = [
      .font: labelFont, .foregroundColor: ReaderColors.tertiary,
    ]
    let size = (label as NSString).size(withAttributes: attributes)
    let origin = NSPoint(x: rect.maxX - size.width - 10, y: rect.minY + 6)
    (label as NSString).draw(at: origin, withAttributes: attributes)
  }
}

/// A block quote or GitHub alert: a rounded bar down the leading edge.
public final class QuoteTextBlock: NSTextBlock {
  public var barColor: NSColor = ReaderColors.quoteBar
  public var fillColor: NSColor?

  public override init() {
    super.init()
    // AppKit only asks a block to draw its background when it has one.
    backgroundColor = .clear
    // Without a width a block shrinks to its narrowest, wrapping after every glyph.
    setContentWidth(100, type: .percentageValueType)
  }

  required init?(coder: NSCoder) {
    super.init(coder: coder)
  }

  public override func drawBackground(
    withFrame frameRect: NSRect, in controlView: NSView?, characterRange charRange: NSRange,
    layoutManager: NSLayoutManager
  ) {
    let rect = contentFrame(of: self, frameRect)
    if let fillColor {
      fillColor.setFill()
      NSBezierPath(roundedRect: rect, xRadius: 6, yRadius: 6).fill()
    }
    barColor.setFill()
    let bar = NSRect(x: rect.minX, y: rect.minY, width: 3, height: rect.height)
    NSBezierPath(roundedRect: bar, xRadius: 1.5, yRadius: 1.5).fill()
  }
}

/// A thematic break: a hairline across the column.
public final class RuleTextBlock: NSTextBlock {
  public override init() {
    super.init()
    // AppKit only asks a block to draw its background when it has one.
    backgroundColor = .clear
    // Without a width a block shrinks to its narrowest, wrapping after every glyph.
    setContentWidth(100, type: .percentageValueType)
  }

  required init?(coder: NSCoder) {
    super.init(coder: coder)
  }

  public override func drawBackground(
    withFrame frameRect: NSRect, in controlView: NSView?, characterRange charRange: NSRange,
    layoutManager: NSLayoutManager
  ) {
    let rect = contentFrame(of: self, frameRect)
    ReaderColors.separator.setFill()
    NSRect(x: rect.minX, y: rect.midY.rounded(), width: rect.width, height: 0.5).fill()
  }
}

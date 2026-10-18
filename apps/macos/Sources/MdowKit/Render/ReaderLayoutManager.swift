import AppKit

/// Draws inline code backgrounds and find highlights as rounded capsules that hug the text,
/// instead of the rectangles AppKit fills across the whole line height.
public final class ReaderLayoutManager: NSLayoutManager {
  private var drawingOrigin = NSPoint.zero

  public override func drawBackground(forGlyphRange glyphsToShow: NSRange, at origin: NSPoint) {
    drawingOrigin = origin
    super.drawBackground(forGlyphRange: glyphsToShow, at: origin)
  }

  public override func fillBackgroundRectArray(
    _ rectArray: UnsafePointer<NSRect>, count rectCount: Int, forCharacterRange charRange: NSRange,
    color: NSColor
  ) {
    let rounded =
      color === ReaderColors.inlineCodeBackground || color === ReaderColors.findMatch
      || color === ReaderColors.findCurrent
    guard rounded, let storage = textStorage,
      charRange.location < storage.length, let container = textContainers.first
    else {
      super.fillBackgroundRectArray(
        rectArray, count: rectCount, forCharacterRange: charRange, color: color)
      return
    }
    let font = storage.attribute(.font, at: charRange.location, effectiveRange: nil) as? NSFont
    let origin = drawingOrigin
    color.setFill()
    for index in 0..<rectCount {
      let rect = rectArray[index]
      guard let font, rect.height > 0 else {
        NSBezierPath(roundedRect: rect, xRadius: 4, yRadius: 4).fill()
        continue
      }
      // The baseline of the line this piece sits on, in the view's coordinates.
      let point = NSPoint(x: rect.midX - origin.x, y: rect.midY - origin.y)
      let glyph = glyphIndex(for: point, in: container)
      let line = lineFragmentRect(forGlyphAt: glyph, effectiveRange: nil)
      let baseline = line.minY + location(forGlyphAt: glyph).y + origin.y
      let top = baseline - font.ascender - 2
      let bottom = baseline - font.descender + 2
      let pill = NSRect(x: rect.minX - 2, y: top, width: rect.width + 4, height: bottom - top)
      NSBezierPath(roundedRect: pill, xRadius: 4, yRadius: 4).fill()
    }
  }
}

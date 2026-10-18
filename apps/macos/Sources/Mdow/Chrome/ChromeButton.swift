import AppKit
import MdowKit

/// A flat button for the window chrome: an icon, optionally a title and a key hint, on a soft
/// rounded background that appears on hover.
final class ChromeButton: NSButton {
  enum Style {
    /// A square icon button.
    case icon
    /// Icon and title, with a hover background.
    case text
    /// A quiet, always-filled pill that reads like a search field.
    case field
  }

  var style: Style { didSet { invalidateIntrinsicContentSize(); needsDisplay = true } }
  var symbolName: String? { didSet { if symbolName != oldValue { needsDisplay = true } } }
  var hint: String? { didSet { invalidateIntrinsicContentSize(); needsDisplay = true } }
  /// Shows the button as switched on, e.g. while the panel it toggles is open.
  var isOn = false { didSet { if isOn != oldValue { needsDisplay = true } } }
  var symbolSize: CGFloat = 14
  var side: CGFloat = 28
  var titleFont = Theme.ui(12.5, .medium)

  private var hovering = false

  init(symbol: String?, title: String, style: Style = .icon, hint: String? = nil, target: AnyObject?, action: Selector?) {
    self.style = style
    self.symbolName = symbol
    self.hint = hint
    super.init(frame: .zero)
    self.title = title
    self.target = target
    self.action = action
    isBordered = false
    setButtonType(.momentaryChange)
    focusRingType = .none
    toolTip = hint.map { "\(title)  \($0)" } ?? title
    setAccessibilityLabel(title)
    translatesAutoresizingMaskIntoConstraints = false
  }

  required init?(coder: NSCoder) { fatalError() }

  override var intrinsicContentSize: NSSize {
    switch style {
    case .icon:
      return NSSize(width: side, height: side)
    case .text, .field:
      var width: CGFloat = 10
      if symbolName != nil { width += symbolSize + 7 }
      width += (title as NSString).size(withAttributes: [.font: titleFont]).width
      if let hint { width += 14 + (hint as NSString).size(withAttributes: [.font: Theme.mono(11)]).width }
      return NSSize(width: (width + 10).rounded(.up), height: side)
    }
  }

  override var isEnabled: Bool { didSet { if isEnabled != oldValue { needsDisplay = true } } }
  override var isHighlighted: Bool { didSet { if isHighlighted != oldValue { needsDisplay = true } } }
  override var mouseDownCanMoveWindow: Bool { false }

  override func updateTrackingAreas() {
    super.updateTrackingAreas()
    trackingAreas.forEach(removeTrackingArea)
    addTrackingArea(
      NSTrackingArea(
        rect: .zero, options: [.mouseEnteredAndExited, .activeAlways, .inVisibleRect], owner: self))
  }

  override func mouseEntered(with event: NSEvent) {
    hovering = true
    needsDisplay = true
  }

  override func mouseExited(with event: NSEvent) {
    hovering = false
    needsDisplay = true
  }

  override func draw(_ dirtyRect: NSRect) {
    let active = isEnabled && (hovering || isHighlighted)
    let shape = NSBezierPath(roundedRect: bounds, xRadius: 7, yRadius: 7)
    switch style {
    case .field:
      (active ? Palette.selected : Palette.hover).setFill()
      shape.fill()
    case .icon, .text:
      if isEnabled, isHighlighted {
        Palette.selected.setFill()
        shape.fill()
      } else if active || isOn {
        Palette.hover.setFill()
        shape.fill()
      }
    }

    let color: NSColor =
      !isEnabled ? Palette.tertiaryText
      : style == .field ? Palette.secondaryText
      : active || isOn ? Palette.text : Palette.secondaryText
    var x: CGFloat = 10
    if let symbolName, let image = Theme.symbol(symbolName, size: symbolSize, color: color) {
      let size = image.size
      let origin =
        style == .icon
        ? NSPoint(x: ((bounds.width - size.width) / 2).rounded(), y: ((bounds.height - size.height) / 2).rounded())
        : NSPoint(x: x + ((symbolSize - size.width) / 2).rounded(), y: ((bounds.height - size.height) / 2).rounded())
      image.draw(in: NSRect(origin: origin, size: size))
      x += symbolSize + 7
    }
    guard style != .icon else { return }
    let titleAttributes: [NSAttributedString.Key: Any] = [.font: titleFont, .foregroundColor: color]
    let titleSize = (title as NSString).size(withAttributes: titleAttributes)
    (title as NSString).draw(
      at: NSPoint(x: x, y: ((bounds.height - titleSize.height) / 2).rounded()),
      withAttributes: titleAttributes)
    if let hint {
      let hintAttributes: [NSAttributedString.Key: Any] = [
        .font: Theme.mono(11), .foregroundColor: Palette.tertiaryText,
      ]
      let hintSize = (hint as NSString).size(withAttributes: hintAttributes)
      (hint as NSString).draw(
        at: NSPoint(x: bounds.width - 10 - hintSize.width, y: ((bounds.height - hintSize.height) / 2).rounded()),
        withAttributes: hintAttributes)
    }
  }

  override func viewDidChangeEffectiveAppearance() {
    super.viewDidChangeEffectiveAppearance()
    needsDisplay = true
  }
}

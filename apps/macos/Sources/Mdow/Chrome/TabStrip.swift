import AppKit
import MdowKit

@MainActor protocol TabStripDelegate: AnyObject {
  func tabStrip(_ strip: TabStrip, didSelect index: Int)
  func tabStrip(_ strip: TabStrip, didClose index: Int)
  func tabStrip(_ strip: TabStrip, didMove from: Int, to: Int)
  func tabStripNewTab(_ strip: TabStrip)
}

struct TabStripItem: Equatable {
  var title: String
  var symbol: String
  var toolTip: String?
  /// Marks a tab with something waiting, like a suggested edit.
  var badge = false
}

/// The window's tabs: compact pills in the header, the selected one raised like a card.
final class TabStrip: NSView {
  weak var delegate: TabStripDelegate?

  private(set) var items: [TabStripItem] = []
  private(set) var selectedIndex = 0
  private var tabViews: [TabPill] = []
  /// The tab being dragged, which moves under the pointer as the tabs reorder.
  fileprivate var draggedIndex: Int?
  private lazy var addButton = ChromeButton(
    symbol: "plus", title: "New Tab", hint: "⌘T", target: self, action: #selector(newTab))

  private let spacing: CGFloat = 4
  private let maxTabWidth: CGFloat = 220
  private let minTabWidth: CGFloat = 84
  private let tabHeight: CGFloat = 28

  override init(frame: NSRect) {
    super.init(frame: frame)
    addButton.translatesAutoresizingMaskIntoConstraints = true
    addButton.symbolSize = 12.5
    addSubview(addButton)
    setAccessibilityRole(.tabGroup)
    setAccessibilityLabel("Tabs")
  }

  required init?(coder: NSCoder) { fatalError() }

  override var mouseDownCanMoveWindow: Bool { true }

  func update(items: [TabStripItem], selectedIndex: Int) {
    guard items != self.items || selectedIndex != self.selectedIndex else { return }
    if items.count != tabViews.count {
      tabViews.forEach { $0.removeFromSuperview() }
      tabViews = items.indices.map { _ in
        let view = TabPill()
        view.strip = self
        addSubview(view, positioned: .below, relativeTo: addButton)
        return view
      }
    }
    self.items = items
    self.selectedIndex = selectedIndex
    for (index, view) in tabViews.enumerated() {
      view.item = items[index]
      view.isSelected = index == selectedIndex
      view.index = index
    }
    needsLayout = true
  }

  override func layout() {
    super.layout()
    let addSize: CGFloat = 28
    let count = CGFloat(max(tabViews.count, 1))
    let available = bounds.width - addSize - spacing
    let width = min(maxTabWidth, max(minTabWidth, ((available - spacing * (count - 1)) / count).rounded(.down)))
    let y = ((bounds.height - tabHeight) / 2).rounded()
    var x: CGFloat = 0
    for view in tabViews {
      view.frame = NSRect(x: x, y: y, width: width, height: tabHeight)
      view.isHidden = x + width > available + spacing
      x += width + spacing
    }
    addButton.frame = NSRect(
      x: min(x, bounds.width - addSize), y: ((bounds.height - addSize) / 2).rounded(), width: addSize,
      height: addSize)
  }

  @objc private func newTab() { delegate?.tabStripNewTab(self) }

  fileprivate func select(_ index: Int) { delegate?.tabStrip(self, didSelect: index) }
  fileprivate func close(_ index: Int) { delegate?.tabStrip(self, didClose: index) }

  /// Where a tab dragged to `x` (in the strip's coordinates) would land.
  fileprivate func dropIndex(forX x: CGFloat) -> Int {
    guard let first = tabViews.first else { return 0 }
    let stride = first.frame.width + spacing
    return min(max(Int((x / stride).rounded(.down)), 0), tabViews.count - 1)
  }

  fileprivate func drag(toX x: CGFloat) {
    guard let from = draggedIndex else { return }
    let to = dropIndex(forX: x)
    guard from != to else { return }
    draggedIndex = to
    delegate?.tabStrip(self, didMove: from, to: to)
  }
}

/// One tab: icon, title and a close button that shows on hover and on the selected tab.
private final class TabPill: NSView {
  weak var strip: TabStrip?
  var index = 0
  var item = TabStripItem(title: "", symbol: "doc.text") {
    didSet {
      guard item != oldValue else { return }
      toolTip = item.toolTip ?? item.title
      setAccessibilityLabel(item.title)
      needsDisplay = true
    }
  }
  var isSelected = false {
    didSet {
      guard isSelected != oldValue else { return }
      setAccessibilityValue(isSelected)
      updateClose()
      needsDisplay = true
    }
  }

  private var hovering = false
  private var dragStart: NSPoint?
  private lazy var closeButton: ChromeButton = {
    let button = ChromeButton(symbol: "xmark", title: "Close Tab", target: self, action: #selector(closeTab))
    button.translatesAutoresizingMaskIntoConstraints = true
    button.symbolSize = 9
    button.side = 18
    button.toolTip = "Close Tab  ⌘W"
    addSubview(button)
    return button
  }()

  override init(frame: NSRect) {
    super.init(frame: frame)
    setAccessibilityElement(true)
    setAccessibilityRole(.radioButton)
    updateClose()
  }

  required init?(coder: NSCoder) { fatalError() }

  override var mouseDownCanMoveWindow: Bool { false }

  override func layout() {
    super.layout()
    closeButton.frame = NSRect(x: bounds.width - 23, y: ((bounds.height - 18) / 2).rounded(), width: 18, height: 18)
  }

  private func updateClose() {
    closeButton.isHidden = !(hovering || isSelected)
  }

  override func updateTrackingAreas() {
    super.updateTrackingAreas()
    trackingAreas.forEach(removeTrackingArea)
    addTrackingArea(
      NSTrackingArea(rect: .zero, options: [.mouseEnteredAndExited, .activeAlways, .inVisibleRect], owner: self))
  }

  override func mouseEntered(with event: NSEvent) {
    hovering = true
    updateClose()
    needsDisplay = true
  }

  override func mouseExited(with event: NSEvent) {
    hovering = false
    updateClose()
    needsDisplay = true
  }

  override func mouseDown(with event: NSEvent) {
    dragStart = event.locationInWindow
    strip?.draggedIndex = index
    strip?.select(index)
  }

  override func mouseDragged(with event: NSEvent) {
    guard let strip, let dragStart, abs(event.locationInWindow.x - dragStart.x) > 6 else { return }
    strip.drag(toX: strip.convert(event.locationInWindow, from: nil).x)
  }

  override func mouseUp(with event: NSEvent) {
    dragStart = nil
    strip?.draggedIndex = nil
  }

  override func otherMouseUp(with event: NSEvent) {
    // A middle click closes, as in browsers.
    if event.buttonNumber == 2 { strip?.close(index) }
  }

  @objc private func closeTab() { strip?.close(index) }

  override func accessibilityPerformPress() -> Bool {
    strip?.select(index)
    return true
  }

  override func draw(_ dirtyRect: NSRect) {
    let rect = bounds.insetBy(dx: 0.5, dy: 0.5)
    let shape = NSBezierPath(roundedRect: rect, xRadius: 8, yRadius: 8)
    if isSelected {
      Palette.elevated.setFill()
      shape.fill()
      Palette.line.setStroke()
      shape.lineWidth = 1
      shape.stroke()
    } else if hovering {
      Palette.hover.setFill()
      shape.fill()
    }

    let color = isSelected ? Palette.text : Palette.secondaryText
    var x: CGFloat = 10
    if let image = Theme.symbol(item.symbol, size: 11.5, color: isSelected ? Palette.secondaryText : Palette.tertiaryText) {
      image.draw(
        in: NSRect(
          x: x + ((14 - image.size.width) / 2).rounded(), y: ((bounds.height - image.size.height) / 2).rounded(),
          width: image.size.width, height: image.size.height))
      x += 20
    }
    let paragraph = NSMutableParagraphStyle()
    paragraph.lineBreakMode = .byTruncatingTail
    let attributes: [NSAttributedString.Key: Any] = [
      .font: Theme.ui(12.5, .medium), .foregroundColor: color,
      .paragraphStyle: paragraph,
    ]
    // Room for the close button is kept whether or not it shows, so a long title doesn't reflow
    // as the pointer passes over the tab.
    let trailing: CGFloat = item.badge ? 34 : 28
    let height = (item.title as NSString).size(withAttributes: attributes).height
    (item.title as NSString).draw(
      in: NSRect(
        x: x, y: ((bounds.height - height) / 2).rounded(), width: max(0, bounds.width - x - trailing),
        height: height),
      withAttributes: attributes)
    if item.badge {
      Palette.accent.setFill()
      // The dot sits where the close button appears, and steps aside for it on hover.
      let dot = NSRect(x: bounds.width - (closeButton.isHidden ? 17 : 30), y: bounds.midY - 3, width: 6, height: 6)
      NSBezierPath(ovalIn: dot).fill()
    }
  }

  override func viewDidChangeEffectiveAppearance() {
    super.viewDidChangeEffectiveAppearance()
    needsDisplay = true
  }
}

import AppKit
import MdowKit

/// The strip across the top of a window: the window buttons, sidebar and history buttons, the
/// tabs, lined up with the document card, and the window's actions on the right.
final class HeaderView: NSView {
  let sidebarButton: ChromeButton
  let backButton: ChromeButton
  let forwardButton: ChromeButton
  let tabs = TabStrip()
  let searchButton: ChromeButton
  let companionButton: ChromeButton
  let moreButton: ChromeButton

  /// Where the document card starts, so the tabs can line up with it.
  var contentLeading: CGFloat = 0 { didSet { if contentLeading != oldValue { needsLayout = true } } }
  /// Full screen hides the window buttons, so everything moves left.
  var isFullScreen = false { didSet { needsLayout = true } }

  init(target: AnyObject) {
    sidebarButton = ChromeButton(
      symbol: "sidebar.left", title: "Toggle Sidebar", hint: "⌃⌘S", target: target,
      action: #selector(DocumentWindowController.toggleSidebarPanel(_:)))
    backButton = ChromeButton(
      symbol: "chevron.left", title: "Back", hint: "⌘[", target: target,
      action: #selector(DocumentWindowController.goBack(_:)))
    forwardButton = ChromeButton(
      symbol: "chevron.right", title: "Forward", hint: "⌘]", target: target,
      action: #selector(DocumentWindowController.goForward(_:)))
    searchButton = ChromeButton(
      symbol: "magnifyingglass", title: "Search", style: .field, hint: "⌘K", target: target,
      action: #selector(DocumentWindowController.quickOpen(_:)))
    companionButton = ChromeButton(
      symbol: "sparkle", title: "Companion", hint: "⌥⌘0", target: target,
      action: #selector(DocumentWindowController.toggleCompanion(_:)))
    moreButton = ChromeButton(
      symbol: "ellipsis", title: "More", target: target,
      action: #selector(DocumentWindowController.showMoreMenu(_:)))
    super.init(frame: .zero)
    backButton.symbolSize = 13
    forwardButton.symbolSize = 13
    searchButton.symbolSize = 12
    searchButton.titleFont = Theme.ui(12.5)
    for view: NSView in [
      sidebarButton, backButton, forwardButton, tabs, searchButton, companionButton, moreButton,
    ] {
      view.translatesAutoresizingMaskIntoConstraints = true
      addSubview(view)
    }
  }

  required init?(coder: NSCoder) { fatalError() }

  override var mouseDownCanMoveWindow: Bool { true }
  override var isFlipped: Bool { true }

  override func layout() {
    super.layout()
    let side: CGFloat = 28
    let y = ((bounds.height - side) / 2).rounded()
    var x = isFullScreen ? Theme.gutter + 2 : Theme.trafficLightsWidth
    for button in [sidebarButton, backButton, forwardButton] {
      button.frame = NSRect(x: x, y: y, width: side, height: side)
      x += side + 2
    }
    let tabsStart = max(x + 10, contentLeading)

    var right = bounds.width - Theme.gutter
    for button in [moreButton, companionButton] {
      right -= side
      button.frame = NSRect(x: right, y: y, width: side, height: side)
      right -= 2
    }
    let searchWidth = max(searchButton.intrinsicContentSize.width, 150)
    right -= 6 + searchWidth
    searchButton.frame = NSRect(x: right, y: y, width: searchWidth, height: side)
    searchButton.isHidden = right < tabsStart + 140
    if searchButton.isHidden { right = companionButton.frame.minX - 2 }
    tabs.frame = NSRect(x: tabsStart, y: 0, width: max(0, right - 12 - tabsStart), height: bounds.height)
  }

  override func mouseDown(with event: NSEvent) {
    guard let window else { return }
    if event.clickCount == 2 {
      // Double-clicking a title bar does what the user chose in Desktop & Dock settings.
      switch UserDefaults.standard.string(forKey: "AppleActionOnDoubleClick") {
      case "Minimize": window.performMiniaturize(nil)
      case "None": break
      default: window.performZoom(nil)
      }
      return
    }
    window.performDrag(with: event)
  }
}

/// The document's card: a rounded sheet of paper inset from the window's edges.
final class CardView: NSView {
  override init(frame: NSRect) {
    super.init(frame: frame)
    wantsLayer = true
    layer?.cornerRadius = Theme.cardRadius
    layer?.cornerCurve = .continuous
    layer?.masksToBounds = true
    layer?.borderWidth = 1
  }

  required init?(coder: NSCoder) { fatalError() }

  override var wantsUpdateLayer: Bool { true }

  override func updateLayer() {
    layer?.backgroundColor = Palette.paper.cgColor
    layer?.borderColor = Palette.line.cgColor
  }
}

/// Hosts the active tab's content in the card, with the window's chrome around it.
final class CardViewController: NSViewController {
  let card = CardView()
  private var leading: NSLayoutConstraint!
  private var trailing: NSLayoutConstraint!
  private(set) var content: NSViewController?

  override func loadView() {
    let root = FilledView(fill: Palette.chrome)
    card.translatesAutoresizingMaskIntoConstraints = false
    root.addSubview(card)
    leading = card.leadingAnchor.constraint(equalTo: root.leadingAnchor, constant: Theme.gutter)
    trailing = root.trailingAnchor.constraint(equalTo: card.trailingAnchor, constant: Theme.gutter)
    NSLayoutConstraint.activate([
      leading, trailing,
      card.topAnchor.constraint(equalTo: root.topAnchor),
      root.bottomAnchor.constraint(equalTo: card.bottomAnchor, constant: Theme.gutter),
    ])
    view = root
  }

  /// The card sits flush against an open sidebar or companion, and inset from the window's edge.
  func setInsets(leading: CGFloat, trailing: CGFloat) {
    _ = view
    if self.leading.constant != leading { self.leading.constant = leading }
    if self.trailing.constant != trailing { self.trailing.constant = trailing }
  }

  func show(_ controller: NSViewController) {
    guard controller !== content else { return }
    if let content {
      content.view.removeFromSuperview()
      content.removeFromParent()
    }
    content = controller
    addChild(controller)
    let view = controller.view
    view.translatesAutoresizingMaskIntoConstraints = false
    card.addSubview(view)
    NSLayoutConstraint.activate([
      view.leadingAnchor.constraint(equalTo: card.leadingAnchor),
      view.trailingAnchor.constraint(equalTo: card.trailingAnchor),
      view.topAnchor.constraint(equalTo: card.topAnchor),
      view.bottomAnchor.constraint(equalTo: card.bottomAnchor),
    ])
  }
}

/// A split view whose dividers are hairlines in the palette's line colour, or invisible.
final class ChromeSplitView: NSSplitView {
  var lineColor: NSColor? = nil

  override var dividerColor: NSColor { lineColor ?? .clear }
  override var dividerThickness: CGFloat { 1 }

  override func drawDivider(in rect: NSRect) {
    guard let lineColor else { return }
    lineColor.setFill()
    rect.fill()
  }
}

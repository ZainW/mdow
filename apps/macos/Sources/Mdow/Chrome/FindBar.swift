import AppKit
import MdowKit

/// Find in the document: a small floating panel in the card's top corner, for Markdown and HTML
/// alike.
final class FindBar: NSView, NSTextFieldDelegate {
  /// The query changed; search again from the reading position.
  var onSearch: ((String) -> Void)?
  /// Return or the arrows: go to the next (or previous) match.
  var onStep: ((_ backwards: Bool) -> Void)?
  var onClose: (() -> Void)?

  private let field = NSTextField()
  private let count = NSTextField(labelWithString: "")
  private lazy var previous = ChromeButton(
    symbol: "chevron.up", title: "Previous Match", hint: "⇧⌘G", target: self, action: #selector(stepBack))
  private lazy var next = ChromeButton(
    symbol: "chevron.down", title: "Next Match", hint: "⌘G", target: self, action: #selector(stepForward))
  private lazy var close = ChromeButton(
    symbol: "xmark", title: "Close", hint: "esc", target: self, action: #selector(closeBar))

  var query: String {
    get { field.stringValue }
    set {
      field.stringValue = newValue
      onSearch?(newValue)
    }
  }

  override init(frame: NSRect) {
    super.init(frame: frame)
    wantsLayer = true
    layer?.cornerRadius = 10
    layer?.cornerCurve = .continuous
    layer?.borderWidth = 1
    layer?.shadowRadius = 14
    layer?.shadowOffset = NSSize(width: 0, height: -4)

    let icon = NSImageView(image: NSImage(systemSymbolName: "magnifyingglass", accessibilityDescription: nil)!)
    icon.symbolConfiguration = .init(pointSize: 12, weight: .medium)
    icon.contentTintColor = Palette.tertiaryText
    field.placeholderAttributedString = NSAttributedString(
      string: "Find in document", attributes: [.font: Theme.ui(13), .foregroundColor: Palette.tertiaryText])
    field.font = Theme.ui(13)
    field.textColor = Palette.text
    field.isBordered = false
    field.drawsBackground = false
    field.focusRingType = .none
    field.delegate = self
    field.lineBreakMode = .byTruncatingTail
    field.cell?.isScrollable = true
    field.setAccessibilityLabel("Find")
    count.font = Theme.mono(11)
    count.textColor = Palette.tertiaryText
    count.alignment = .right
    count.setContentHuggingPriority(.required, for: .horizontal)
    count.setContentCompressionResistancePriority(.required, for: .horizontal)
    for button in [previous, next, close] {
      button.side = 24
      button.symbolSize = 11
    }
    let divider = FilledView(fill: Palette.line)
    for view: NSView in [icon, field, count, divider, previous, next, close] {
      view.translatesAutoresizingMaskIntoConstraints = false
      addSubview(view)
    }
    NSLayoutConstraint.activate([
      heightAnchor.constraint(equalToConstant: 38),
      icon.leadingAnchor.constraint(equalTo: leadingAnchor, constant: 12),
      icon.centerYAnchor.constraint(equalTo: centerYAnchor),
      field.leadingAnchor.constraint(equalTo: icon.trailingAnchor, constant: 8),
      field.centerYAnchor.constraint(equalTo: centerYAnchor),
      count.leadingAnchor.constraint(equalTo: field.trailingAnchor, constant: 8),
      count.centerYAnchor.constraint(equalTo: centerYAnchor),
      divider.leadingAnchor.constraint(equalTo: count.trailingAnchor, constant: 10),
      divider.widthAnchor.constraint(equalToConstant: 1),
      divider.heightAnchor.constraint(equalToConstant: 18),
      divider.centerYAnchor.constraint(equalTo: centerYAnchor),
      previous.leadingAnchor.constraint(equalTo: divider.trailingAnchor, constant: 6),
      previous.centerYAnchor.constraint(equalTo: centerYAnchor),
      next.leadingAnchor.constraint(equalTo: previous.trailingAnchor, constant: 2),
      next.centerYAnchor.constraint(equalTo: centerYAnchor),
      close.leadingAnchor.constraint(equalTo: next.trailingAnchor, constant: 2),
      close.trailingAnchor.constraint(equalTo: trailingAnchor, constant: -7),
      close.centerYAnchor.constraint(equalTo: centerYAnchor),
    ])
    setAccessibilityElement(true)
    setAccessibilityRole(.group)
    setAccessibilityLabel("Find")
  }

  required init?(coder: NSCoder) { fatalError() }

  override var wantsUpdateLayer: Bool { true }

  override func updateLayer() {
    let dark = effectiveAppearance.bestMatch(from: [.aqua, .darkAqua]) == .darkAqua
    layer?.backgroundColor = Palette.elevated.cgColor
    layer?.borderColor = Palette.line.cgColor
    layer?.shadowColor = NSColor.black.cgColor
    layer?.shadowOpacity = dark ? 0.45 : 0.1
  }

  /// Shows "3/12", "No results", or nothing when there's no query or no count.
  func setResult(current: Int?, total: Int?) {
    if field.stringValue.isEmpty {
      count.stringValue = ""
    } else if let total, total == 0 {
      count.stringValue = "No results"
    } else if let total, let current {
      count.stringValue = "\(current)/\(total)"
    } else {
      count.stringValue = ""
    }
    count.textColor = total == 0 && !field.stringValue.isEmpty ? Palette.accent : Palette.tertiaryText
    previous.isEnabled = total != 0
    next.isEnabled = total != 0
  }

  func focus() {
    window?.makeFirstResponder(field)
    field.currentEditor()?.selectAll(nil)
  }

  func controlTextDidChange(_ notification: Notification) {
    onSearch?(field.stringValue)
  }

  func control(_ control: NSControl, textView: NSTextView, doCommandBy selector: Selector) -> Bool {
    switch selector {
    case #selector(NSResponder.cancelOperation(_:)):
      onClose?()
      return true
    case #selector(NSResponder.insertNewline(_:)):
      onStep?(NSApp.currentEvent?.modifierFlags.contains(.shift) == true)
      return true
    default:
      return false
    }
  }

  @objc private func stepBack() { onStep?(true) }
  @objc private func stepForward() { onStep?(false) }
  @objc private func closeBar() { onClose?() }
}

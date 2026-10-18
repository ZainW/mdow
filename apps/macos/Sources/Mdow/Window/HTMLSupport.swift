import AppKit

/// A find bar for HTML pages that looks like the one `NSTextFinder` gives the text view.
final class HTMLFindBar: NSView, NSSearchFieldDelegate {
  var onFind: ((String, Bool) -> Void)?
  var onClose: (() -> Void)?

  private let field = NSSearchField()
  private let arrows = NSSegmentedControl()
  private let done = NSButton(title: "Done", target: nil, action: nil)
  private let effect = NSVisualEffectView()

  var query: String {
    get { field.stringValue }
    set {
      field.stringValue = newValue
      onFind?(newValue, false)
    }
  }

  /// Tints the field when the last search found nothing, like the system find bar.
  var notFound = false {
    didSet { field.textColor = notFound ? .systemRed : .labelColor }
  }

  override init(frame: NSRect) {
    super.init(frame: frame)
    effect.material = .headerView
    effect.blendingMode = .withinWindow
    effect.translatesAutoresizingMaskIntoConstraints = false
    addSubview(effect)

    field.placeholderString = "Find"
    field.controlSize = .small
    field.font = .systemFont(ofSize: NSFont.smallSystemFontSize)
    field.sendsSearchStringImmediately = true
    field.sendsWholeSearchString = false
    field.delegate = self
    field.target = self
    field.action = #selector(search)
    field.translatesAutoresizingMaskIntoConstraints = false

    arrows.segmentCount = 2
    arrows.trackingMode = .momentary
    arrows.controlSize = .small
    arrows.setImage(NSImage(systemSymbolName: "chevron.left", accessibilityDescription: "Previous"), forSegment: 0)
    arrows.setImage(NSImage(systemSymbolName: "chevron.right", accessibilityDescription: "Next"), forSegment: 1)
    arrows.setToolTip("Previous", forSegment: 0)
    arrows.setToolTip("Next", forSegment: 1)
    arrows.target = self
    arrows.action = #selector(step)
    arrows.translatesAutoresizingMaskIntoConstraints = false

    done.controlSize = .small
    done.bezelStyle = .push
    done.target = self
    done.action = #selector(close)
    done.translatesAutoresizingMaskIntoConstraints = false

    let separator = NSBox()
    separator.boxType = .separator
    separator.translatesAutoresizingMaskIntoConstraints = false
    for view in [field, arrows, done, separator] as [NSView] { addSubview(view) }
    NSLayoutConstraint.activate([
      heightAnchor.constraint(equalToConstant: 30),
      effect.leadingAnchor.constraint(equalTo: leadingAnchor),
      effect.trailingAnchor.constraint(equalTo: trailingAnchor),
      effect.topAnchor.constraint(equalTo: topAnchor),
      effect.bottomAnchor.constraint(equalTo: bottomAnchor),
      field.leadingAnchor.constraint(equalTo: leadingAnchor, constant: 10),
      field.centerYAnchor.constraint(equalTo: centerYAnchor),
      arrows.leadingAnchor.constraint(equalTo: field.trailingAnchor, constant: 8),
      arrows.centerYAnchor.constraint(equalTo: centerYAnchor),
      done.leadingAnchor.constraint(equalTo: arrows.trailingAnchor, constant: 8),
      done.trailingAnchor.constraint(equalTo: trailingAnchor, constant: -10),
      done.centerYAnchor.constraint(equalTo: centerYAnchor),
      separator.leadingAnchor.constraint(equalTo: leadingAnchor),
      separator.trailingAnchor.constraint(equalTo: trailingAnchor),
      separator.bottomAnchor.constraint(equalTo: bottomAnchor),
    ])
    setAccessibilityRole(.group)
    setAccessibilityLabel("Find")
  }

  required init?(coder: NSCoder) { fatalError() }

  func focus() {
    window?.makeFirstResponder(field)
    field.currentEditor()?.selectAll(nil)
  }

  @objc private func search() {
    onFind?(field.stringValue, false)
  }

  @objc private func step() {
    onFind?(field.stringValue, arrows.selectedSegment == 0)
  }

  @objc private func close() {
    onClose?()
  }

  func control(_ control: NSControl, textView: NSTextView, doCommandBy selector: Selector) -> Bool {
    switch selector {
    case #selector(NSResponder.cancelOperation(_:)):
      onClose?()
      return true
    case #selector(NSResponder.insertNewline(_:)):
      let backwards = NSApp.currentEvent?.modifierFlags.contains(.shift) == true
      onFind?(field.stringValue, backwards)
      return true
    default:
      return false
    }
  }
}

/// Darkens HTML pages that have no dark appearance of their own, by inverting the page and
/// inverting its media back. The filter sits on `<body>`, so `<html>` keeps a real dark
/// background for the area WebKit paints under the toolbar. Runs in Mdow's own script world,
/// so page scripts stay off.
enum HTMLDarkMode {
  static let background = NSColor(srgbRed: 0x1e / 255, green: 0x1e / 255, blue: 0x1e / 255, alpha: 1)

  static let script = """
    (() => {
      if (window.__mdowSetDark) return;
      const supportsDark = () => {
        const meta = document.querySelector('meta[name="color-scheme"]');
        if (meta && /dark/i.test(meta.content)) return true;
        if (/dark/.test(getComputedStyle(document.documentElement).colorScheme || '')) return true;
        for (const sheet of document.styleSheets) {
          try {
            for (const rule of sheet.cssRules) {
              if (rule.media && /prefers-color-scheme:\\s*dark/.test(rule.media.mediaText)) return true;
            }
          } catch (error) {}
        }
        return false;
      };
      // Checked once, before Mdow's own style is added.
      const native = window.__mdowNative ?? (window.__mdowNative = supportsDark());
      window.__mdowSetDark = (on) => {
        let style = document.getElementById('__mdow-dark');
        if (native || !on) {
          if (style) style.remove();
          return false;
        }
        if (style) return true;
        style = document.createElement('style');
        style.id = '__mdow-dark';
        style.textContent = `
          html { background: #1e1e1e !important; }
          body { filter: invert(1) hue-rotate(180deg); background: #e1e1e1; min-height: 100vh; }
          img, video, picture, canvas, iframe, object, embed, svg image,
          [style*="background-image"] { filter: invert(1) hue-rotate(180deg); }`;
        document.documentElement.appendChild(style);
        return true;
      };
    })();
    """
}

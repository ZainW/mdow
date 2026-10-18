import AppKit
import MdowKit
import SwiftUI

/// Fonts and measurements for the window around the document. Colours come from `Palette`, which
/// the reader shares.
enum Theme {
  /// The strip across the top of the window with the window buttons, tabs and actions.
  static let headerHeight: CGFloat = 40
  /// The margin between the document card and the window's edges.
  static let gutter: CGFloat = 8
  static let cardRadius: CGFloat = 12
  /// Room for the close, minimise and zoom buttons at the left of the header.
  static let trafficLightsWidth: CGFloat = 76

  /// Registers the bundled fonts for this process.
  static func registerFonts() {
    guard let folder = Bundle.main.resourceURL?.appendingPathComponent("Fonts"),
      let files = try? FileManager.default.contentsOfDirectory(at: folder, includingPropertiesForKeys: nil)
    else { return }
    for file in files where ["woff2", "ttf", "otf"].contains(file.pathExtension) {
      CTFontManagerRegisterFontsForURL(file as CFURL, .process, nil)
    }
  }

  /// Inter, falling back to the system font when it isn't bundled (e.g. `swift run`).
  static func ui(_ size: CGFloat, _ weight: NSFont.Weight = .regular) -> NSFont {
    BundledFont.font(family: "Inter Variable", size: size, weight: weight)
      ?? .systemFont(ofSize: size, weight: weight)
  }

  /// Geist Mono, for paths, key hints and other machine-ish text.
  static func mono(_ size: CGFloat, _ weight: NSFont.Weight = .regular) -> NSFont {
    BundledFont.font(family: "Geist Mono", size: size, weight: weight)
      ?? .monospacedSystemFont(ofSize: size, weight: weight)
  }

  /// A symbol image drawn in one colour, resolved for the current drawing appearance.
  static func symbol(_ name: String, size: CGFloat, weight: NSFont.Weight = .medium, color: NSColor)
    -> NSImage?
  {
    let resolved = NSColor(cgColor: color.cgColor) ?? color
    let config = NSImage.SymbolConfiguration(pointSize: size, weight: weight)
      .applying(.init(paletteColors: [resolved]))
    return NSImage(systemSymbolName: name, accessibilityDescription: nil)?
      .withSymbolConfiguration(config)
  }
}

/// SwiftUI versions of the same fonts and colours, for the panels written in SwiftUI.
extension Font {
  static func mdowUI(_ size: CGFloat, _ weight: NSFont.Weight = .regular) -> Font {
    Font(Theme.ui(size, weight) as CTFont)
  }

  static func mdowMono(_ size: CGFloat, _ weight: NSFont.Weight = .regular) -> Font {
    Font(Theme.mono(size, weight) as CTFont)
  }
}

extension Color {
  static let mdowChrome = Color(nsColor: Palette.chrome)
  static let mdowPaper = Color(nsColor: Palette.paper)
  static let mdowElevated = Color(nsColor: Palette.elevated)
  static let mdowWell = Color(nsColor: Palette.well)
  static let mdowText = Color(nsColor: Palette.text)
  static let mdowSecondary = Color(nsColor: Palette.secondaryText)
  static let mdowTertiary = Color(nsColor: Palette.tertiaryText)
  static let mdowLine = Color(nsColor: Palette.line)
  static let mdowStrongLine = Color(nsColor: Palette.strongLine)
  static let mdowHover = Color(nsColor: Palette.hover)
  static let mdowSelected = Color(nsColor: Palette.selected)
  static let mdowAccent = Color(nsColor: Palette.accent)
  static let mdowAccentSoft = Color(nsColor: Palette.accentSoft)
}

/// A view filled with one of the palette's colours, kept in step with the appearance.
class FilledView: NSView {
  var fill: NSColor? { didSet { needsDisplay = true } }

  init(fill: NSColor?) {
    self.fill = fill
    super.init(frame: .zero)
  }

  required init?(coder: NSCoder) { fatalError() }

  override var isOpaque: Bool { false }

  override func draw(_ dirtyRect: NSRect) {
    guard let fill else { return }
    fill.setFill()
    dirtyRect.fill()
  }
}

import AppKit

/// Mdow's colours, shared by the reader and the window around it: warm paper in light mode,
/// neutral ink in dark mode, with one clay accent. All are dynamic, so anything drawn with them
/// follows the window's appearance.
public enum Palette {
  /// The window itself: sidebar, tab bar and the margin around the document.
  public static let chrome = pair(0xF3F1EB, 0x131313)
  /// The card the document sits on.
  public static let paper = pair(0xFDFCF9, 0x1B1B1B)
  /// Things that float: the active tab, menus, the command palette.
  public static let elevated = pair(0xFFFFFF, 0x242424)
  /// Recessed areas inside the paper, like code blocks.
  public static let well = pair(0xF5F3ED, 0x161616)

  public static let text = pair(0x1E1C19, 0xECECEC)
  public static let secondaryText = pair(0x6B665E, 0xA3A3A3)
  public static let tertiaryText = pair(0x9E998F, 0x6E6E6E)

  /// Hairlines between surfaces.
  public static let line = ReaderColors.dynamic(light: ink(0.1), dark: glow(0.08))
  /// Lines that need to read as structure, like table rules.
  public static let strongLine = ReaderColors.dynamic(light: ink(0.16), dark: glow(0.14))
  public static let hover = ReaderColors.dynamic(light: ink(0.05), dark: glow(0.05))
  public static let selected = ReaderColors.dynamic(light: ink(0.085), dark: glow(0.085))

  public static let accent = pair(0xB95A33, 0xE08A66)
  public static let accentSoft = ReaderColors.dynamic(
    light: rgb(0xB95A33).withAlphaComponent(0.12), dark: rgb(0xE08A66).withAlphaComponent(0.16))
  public static let textSelection = ReaderColors.dynamic(
    light: rgb(0xD97757).withAlphaComponent(0.24), dark: rgb(0xE08A66).withAlphaComponent(0.3))

  public static func rgb(_ hex: UInt32) -> NSColor {
    NSColor(
      srgbRed: CGFloat((hex >> 16) & 0xFF) / 255, green: CGFloat((hex >> 8) & 0xFF) / 255,
      blue: CGFloat(hex & 0xFF) / 255, alpha: 1)
  }

  static func pair(_ light: UInt32, _ dark: UInt32) -> NSColor {
    ReaderColors.dynamic(light: rgb(light), dark: rgb(dark))
  }

  /// The light theme's warm near-black at an opacity.
  private static func ink(_ alpha: CGFloat) -> NSColor { rgb(0x3C2D1E).withAlphaComponent(alpha) }
  private static func glow(_ alpha: CGFloat) -> NSColor { NSColor(white: 1, alpha: alpha) }
}

/// Looks up the fonts the app bundles. They're variable fonts, so weights are set on the
/// weight axis rather than picked by name.
public enum BundledFont {
  public static func font(family: String, size: CGFloat, weight: NSFont.Weight) -> NSFont? {
    guard let base = NSFont(descriptor: NSFontDescriptor(fontAttributes: [.family: family]), size: size),
      base.familyName == family
    else { return nil }
    let axis = NSNumber(value: 0x7767_6874)  // 'wght'
    let descriptor = base.fontDescriptor.addingAttributes([
      .variation: [axis: NSNumber(value: cssWeight(weight))]
    ])
    return NSFont(descriptor: descriptor, size: size) ?? base
  }

  /// `NSFont.Weight` on CSS's 100–900 scale.
  static func cssWeight(_ weight: NSFont.Weight) -> Double {
    let steps: [(NSFont.Weight, Double)] = [
      (.ultraLight, 200), (.thin, 250), (.light, 300), (.regular, 400), (.medium, 500),
      (.semibold, 600), (.bold, 700), (.heavy, 800), (.black, 900),
    ]
    return steps.min { abs($0.0.rawValue - weight.rawValue) < abs($1.0.rawValue - weight.rawValue) }!.1
  }
}

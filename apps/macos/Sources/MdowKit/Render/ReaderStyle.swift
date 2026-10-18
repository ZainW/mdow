import AppKit

public enum BodyFont: String, CaseIterable, Sendable, Identifiable {
  case charter, inter, system, newYork, georgia, avenir

  public var id: String { rawValue }

  public var displayName: String {
    switch self {
    case .inter: "Inter"
    case .system: "San Francisco"
    case .newYork: "New York"
    case .charter: "Charter"
    case .georgia: "Georgia"
    case .avenir: "Avenir Next"
    }
  }

  public func font(size: CGFloat, weight: NSFont.Weight = .regular) -> NSFont {
    switch self {
    case .inter:
      return BundledFont.font(family: "Inter Variable", size: size, weight: weight)
        ?? NSFont.systemFont(ofSize: size, weight: weight)
    case .system:
      return NSFont.systemFont(ofSize: size, weight: weight)
    case .newYork:
      let base = NSFont.systemFont(ofSize: size, weight: weight)
      if let serif = base.fontDescriptor.withDesign(.serif) {
        return NSFont(descriptor: serif, size: size) ?? base
      }
      return base
    case .charter, .georgia, .avenir:
      let family =
        switch self {
        case .charter: "Charter"
        case .georgia: "Georgia"
        default: "Avenir Next"
        }
      let descriptor = NSFontDescriptor(fontAttributes: [
        .family: family,
        .traits: [NSFontDescriptor.TraitKey.weight: weight.rawValue],
      ])
      return NSFont(descriptor: descriptor, size: size)
        ?? NSFont.systemFont(ofSize: size, weight: weight)
    }
  }
}

public enum CodeFont: String, CaseIterable, Sendable, Identifiable {
  case geistMono, sfMono, menlo, monaco

  public var id: String { rawValue }

  public var displayName: String {
    switch self {
    case .geistMono: "Geist Mono"
    case .sfMono: "SF Mono"
    case .menlo: "Menlo"
    case .monaco: "Monaco"
    }
  }

  public func font(size: CGFloat, weight: NSFont.Weight = .regular) -> NSFont {
    switch self {
    case .geistMono:
      return BundledFont.font(family: "Geist Mono", size: size, weight: weight)
        ?? NSFont.monospacedSystemFont(ofSize: size, weight: weight)
    case .sfMono:
      return NSFont.monospacedSystemFont(ofSize: size, weight: weight)
    case .menlo, .monaco:
      let name = self == .menlo ? "Menlo" : "Monaco"
      let descriptor = NSFontDescriptor(fontAttributes: [
        .family: name,
        .traits: [NSFontDescriptor.TraitKey.weight: weight.rawValue],
      ])
      return NSFont(descriptor: descriptor, size: size)
        ?? NSFont.monospacedSystemFont(ofSize: size, weight: weight)
    }
  }
}

public enum LineSpacing: String, CaseIterable, Sendable, Identifiable {
  case compact, standard, relaxed

  public var id: String { rawValue }

  public var displayName: String {
    switch self {
    case .compact: "Compact"
    case .standard: "Standard"
    case .relaxed: "Relaxed"
    }
  }

  public var multiple: CGFloat {
    switch self {
    case .compact: 1.18
    case .standard: 1.32
    case .relaxed: 1.5
    }
  }
}

public enum ReadingWidth: String, CaseIterable, Sendable, Identifiable {
  case narrow, standard, wide, full

  public var id: String { rawValue }

  public var displayName: String {
    switch self {
    case .narrow: "Narrow"
    case .standard: "Standard"
    case .wide: "Wide"
    case .full: "Full Width"
    }
  }

  /// The text column's maximum width in points at the default text size.
  public var points: CGFloat? {
    switch self {
    case .narrow: 600
    case .standard: 720
    case .wide: 920
    case .full: nil
    }
  }
}

/// Everything the renderer needs to know about how a document should look.
public struct ReaderStyle: Sendable, Equatable {
  public var bodyFont: BodyFont = .charter
  public var codeFont: CodeFont = .geistMono
  public var lineSpacing: LineSpacing = .standard
  /// Body text size in points, zoom included.
  public var fontSize: CGFloat = 16
  /// The widest an image or diagram may be drawn.
  public var maxContentWidth: CGFloat = 720

  public init() {}

  var bodySize: CGFloat { fontSize }
  var codeSize: CGFloat { (fontSize * 0.86).rounded(.toNearestOrEven) }
  var blockSpacing: CGFloat { (fontSize * 0.85).rounded() }

  func body(_ weight: NSFont.Weight = .regular) -> NSFont {
    bodyFont.font(size: bodySize, weight: weight)
  }

  func code(size: CGFloat? = nil, weight: NSFont.Weight = .regular) -> NSFont {
    codeFont.font(size: size ?? codeSize, weight: weight)
  }

  func headingSize(_ level: Int) -> CGFloat {
    let scale: CGFloat =
      switch level {
      case 1: 2.0
      case 2: 1.45
      case 3: 1.2
      case 4: 1.06
      default: 1.0
      }
    return (fontSize * scale).rounded()
  }
}

/// Colours used by the reader, drawn from `Palette`. All are dynamic, so a rendered document
/// follows the window's appearance without being rebuilt.
public enum ReaderColors {
  public static let text = Palette.text
  public static let secondary = Palette.secondaryText
  public static let tertiary = Palette.tertiaryText
  public static let separator = Palette.line
  public static let link = Palette.accent

  public static let codeBackground = Palette.well
  public static let codeBorder = Palette.line
  public static let inlineCodeBackground = dynamic(
    light: NSColor(srgbRed: 0.24, green: 0.18, blue: 0.12, alpha: 0.07),
    dark: NSColor(white: 1, alpha: 0.08))
  public static let tableBorder = Palette.strongLine
  public static let quoteBar = dynamic(
    light: NSColor(srgbRed: 0.24, green: 0.18, blue: 0.12, alpha: 0.18),
    dark: NSColor(white: 1, alpha: 0.18))
  public static let mark = dynamic(
    light: NSColor(srgbRed: 1, green: 0.85, blue: 0.35, alpha: 0.55),
    dark: NSColor(srgbRed: 0.95, green: 0.75, blue: 0.25, alpha: 0.35))

  /// Every match while finding, and the one being shown.
  public static let findMatch = dynamic(
    light: NSColor(srgbRed: 1, green: 0.8, blue: 0.3, alpha: 0.35),
    dark: NSColor(srgbRed: 1, green: 0.8, blue: 0.3, alpha: 0.22))
  public static let findCurrent = dynamic(
    light: NSColor(srgbRed: 1, green: 0.72, blue: 0.2, alpha: 0.85),
    dark: NSColor(srgbRed: 1, green: 0.72, blue: 0.25, alpha: 0.6))

  public static func dynamic(light: NSColor, dark: NSColor) -> NSColor {
    NSColor(name: nil) { appearance in
      appearance.bestMatch(from: [.aqua, .darkAqua]) == .darkAqua ? dark : light
    }
  }
}

public extension NSAttributedString.Key {
  /// A `CodeBlockInfo` covering a fenced code block's text.
  static let mdowCodeBlock = NSAttributedString.Key("mdow.codeBlock")
  /// A heading's anchor slug, on the heading's text.
  static let mdowAnchor = NSAttributedString.Key("mdow.anchor")
  /// Numbers each image and diagram in document order, on its placeholder or attachment, so a
  /// later render with the assets loaded can be swapped in piece by piece.
  static let mdowAssetSlot = NSAttributedString.Key("mdow.assetSlot")
}

/// Identifies a code block so the reader can offer to copy it.
public final class CodeBlockInfo: NSObject, @unchecked Sendable {
  public let code: String
  public let language: String?

  public init(code: String, language: String?) {
    self.code = code
    self.language = language
  }
}

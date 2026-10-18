import AppKit

public enum BodyFont: String, CaseIterable, Sendable, Identifiable {
  case system, newYork, charter, georgia, avenir

  public var id: String { rawValue }

  public var displayName: String {
    switch self {
    case .system: "San Francisco"
    case .newYork: "New York"
    case .charter: "Charter"
    case .georgia: "Georgia"
    case .avenir: "Avenir Next"
    }
  }

  public func font(size: CGFloat, weight: NSFont.Weight = .regular) -> NSFont {
    switch self {
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
  case sfMono, menlo, monaco

  public var id: String { rawValue }

  public var displayName: String {
    switch self {
    case .sfMono: "SF Mono"
    case .menlo: "Menlo"
    case .monaco: "Monaco"
    }
  }

  public func font(size: CGFloat, weight: NSFont.Weight = .regular) -> NSFont {
    switch self {
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
  public var bodyFont: BodyFont = .system
  public var codeFont: CodeFont = .sfMono
  public var lineSpacing: LineSpacing = .standard
  /// Body text size in points, zoom included.
  public var fontSize: CGFloat = 15
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
      case 1: 1.9
      case 2: 1.5
      case 3: 1.25
      case 4: 1.1
      default: 1.0
      }
    return (fontSize * scale).rounded()
  }
}

/// Colours used by the reader. All are dynamic, so a rendered document follows the
/// window's appearance without being rebuilt.
public enum ReaderColors {
  public static let text = NSColor.labelColor
  public static let secondary = NSColor.secondaryLabelColor
  public static let tertiary = NSColor.tertiaryLabelColor
  public static let separator = NSColor.separatorColor
  public static let link = NSColor.linkColor

  public static let codeBackground = dynamic(
    light: NSColor(white: 0, alpha: 0.04), dark: NSColor(white: 1, alpha: 0.06))
  public static let codeBorder = dynamic(
    light: NSColor(white: 0, alpha: 0.07), dark: NSColor(white: 1, alpha: 0.08))
  public static let inlineCodeBackground = dynamic(
    light: NSColor(white: 0, alpha: 0.055), dark: NSColor(white: 1, alpha: 0.1))
  public static let tableBorder = dynamic(
    light: NSColor(white: 0, alpha: 0.12), dark: NSColor(white: 1, alpha: 0.14))
  public static let quoteBar = dynamic(
    light: NSColor(white: 0, alpha: 0.14), dark: NSColor(white: 1, alpha: 0.2))
  public static let mark = NSColor.findHighlightColor.withAlphaComponent(0.5)

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

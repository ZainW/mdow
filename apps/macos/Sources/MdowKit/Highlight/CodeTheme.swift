import AppKit

/// Code colours from Xcode's "Default (Light)" and "Default (Dark)" themes.
public enum CodeTheme {
  public static func color(for kind: TokenKind) -> NSColor {
    switch kind {
    case .plain, .operator_, .punctuation: NSColor.labelColor
    case .keyword, .constant: keyword
    case .string: string
    case .number: number
    case .comment: comment
    case .type: type
    case .function, .property: function
    case .attribute: attribute
    case .tag, .heading: declaration
    case .preprocessor: preprocessor
    case .regex: regex
    case .link: link
    case .deleted: deleted
    case .inserted: inserted
    }
  }

  /// Xcode sets keywords (including `true`, `nil`, `self`) in bold.
  public static func isBold(_ kind: TokenKind) -> Bool {
    kind == .keyword || kind == .constant || kind == .heading
  }

  private typealias RGB = (r: Int, g: Int, b: Int)

  private static func dynamic(_ name: String, light: RGB, dark: RGB) -> NSColor {
    let l = (CGFloat(light.r) / 255, CGFloat(light.g) / 255, CGFloat(light.b) / 255)
    let d = (CGFloat(dark.r) / 255, CGFloat(dark.g) / 255, CGFloat(dark.b) / 255)
    return NSColor(name: NSColor.Name("mdow.code.\(name)")) { appearance in
      let c = appearance.bestMatch(from: [.aqua, .darkAqua]) == .darkAqua ? d : l
      return NSColor(srgbRed: c.0, green: c.1, blue: c.2, alpha: 1)
    }
  }

  private static let keyword = dynamic("keyword", light: (155, 35, 147), dark: (252, 95, 163))
  private static let string = dynamic("string", light: (196, 26, 22), dark: (252, 106, 93))
  private static let number = dynamic("number", light: (28, 0, 207), dark: (208, 191, 105))
  private static let comment = dynamic("comment", light: (93, 108, 121), dark: (108, 121, 134))
  private static let type = dynamic("type", light: (57, 0, 160), dark: (208, 168, 255))
  private static let function = dynamic("function", light: (108, 54, 169), dark: (161, 103, 230))
  private static let attribute = dynamic("attribute", light: (129, 95, 3), dark: (191, 133, 85))
  private static let declaration = dynamic(
    "declaration", light: (11, 79, 121), dark: (93, 216, 255))
  private static let preprocessor = dynamic(
    "preprocessor", light: (100, 56, 32), dark: (253, 143, 63))
  private static let regex = dynamic("regex", light: (209, 47, 27), dark: (255, 129, 112))
  private static let link = dynamic("link", light: (14, 14, 255), dark: (84, 130, 255))
  private static let deleted = dynamic("deleted", light: (179, 29, 40), dark: (255, 123, 114))
  private static let inserted = dynamic("inserted", light: (34, 134, 58), dark: (126, 231, 135))
}

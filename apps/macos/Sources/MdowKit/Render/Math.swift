import AppKit
import SwiftMath

// MARK: - Extraction

/// TeX pulled out of the markdown before it's parsed, following GitHub's rules: `$…$` and
/// `` $`…`$ `` inline, `$$…$$` for display math. (```` ```math ```` fences reach the renderer as
/// code blocks.) Markdown would otherwise mangle TeX: backslash escapes eat `\{` and `\\`, and
/// `_` or `*` turn into emphasis. Each span is replaced by a private-use placeholder,
/// `U+E000 <index> U+E001`, which the parser passes through as plain text.
public struct MathSpans: Sendable, Equatable {
  public struct Span: Sendable, Equatable {
    public var tex: String
    public var display: Bool
    /// The span as written in the markdown, delimiters included.
    public var source: String
  }

  public var spans: [Span] = []

  static let open: Character = "\u{E000}"
  static let close: Character = "\u{E001}"

  public init() {}

  enum Piece: Equatable {
    case text(Substring)
    case math(Int)
  }

  /// Splits text into plain runs and placeholders.
  func pieces(_ string: String) -> [Piece] {
    guard !spans.isEmpty, string.contains(Self.open) else { return [.text(string[...])] }
    var pieces: [Piece] = []
    var rest = string[...]
    while let start = rest.firstIndex(of: Self.open) {
      let digits = rest[rest.index(after: start)...].prefix { $0.isASCII && $0.isNumber }
      guard digits.endIndex < rest.endIndex, rest[digits.endIndex] == Self.close,
        let index = Int(digits), index < spans.count
      else {
        // Not one of ours: keep the character and look further on.
        let next = rest.index(after: start)
        pieces.append(.text(rest[..<next]))
        rest = rest[next...]
        continue
      }
      if start > rest.startIndex { pieces.append(.text(rest[..<start])) }
      pieces.append(.math(index))
      rest = rest[rest.index(after: digits.endIndex)...]
    }
    if !rest.isEmpty { pieces.append(.text(rest)) }
    return pieces
  }

  private func substitute(_ string: String, _ replacement: (Span) -> String) -> String {
    guard !spans.isEmpty, string.contains(Self.open) else { return string }
    var out = ""
    for piece in pieces(string) {
      switch piece {
      case .text(let text): out += text
      case .math(let index): out += replacement(spans[index])
      }
    }
    return out
  }

  /// Puts the original markdown back, for places math doesn't apply: code, HTML and URLs.
  public func restoringSource(_ string: String) -> String { substitute(string) { $0.source } }

  /// Replaces placeholders with bare TeX, for headings' outline titles and slugs.
  public func replacingWithTeX(_ string: String) -> String { substitute(string) { $0.tex } }

  /// Returns the body with math replaced by placeholders, plus the spans.
  public static func extract(from body: String) -> (String, MathSpans) {
    guard body.containsBytes("$") else { return (body, MathSpans()) }
    var scanner = Scanner()
    scanner.run(body)
    guard !scanner.spans.spans.isEmpty else { return (body, MathSpans()) }
    return (scanner.output, scanner.spans)
  }

  /// Walks the document line by line, leaving fenced and indented code alone, and scans each
  /// run of text lines (a paragraph, more or less) for math.
  private struct Scanner {
    enum LineKind { case code, blank, text }

    var spans = MathSpans()
    /// Output pieces, each standing for one or more input lines; joined with newlines.
    var parts: [String] = []
    var pending: [Substring] = []
    var fence: (marker: Character, length: Int)?
    var inList = false
    var previousBlank = true
    var inIndentedCode = false

    var output: String { parts.joined(separator: "\n") }

    mutating func run(_ body: String) {
      for line in body.split(separator: "\n", omittingEmptySubsequences: false) {
        switch kind(of: line) {
        case .text:
          pending.append(line)
        case .code, .blank:
          flush()
          parts.append(String(line))
        }
      }
      flush()
    }

    mutating func flush() {
      guard !pending.isEmpty else { return }
      parts.append(scanInline(pending.joined(separator: "\n")))
      pending.removeAll()
    }

    /// Classifies a line, tracking fences, indented code and lists as it goes.
    mutating func kind(of line: Substring) -> LineKind {
      let content = Self.stripContainers(line)
      let blank = content.allSatisfy { $0 == " " || $0 == "\t" }
      defer { previousBlank = blank }

      if let open = fence {
        let run = content.prefix { $0 == open.marker }
        if run.count >= open.length,
          content.dropFirst(run.count).allSatisfy({ $0 == " " || $0 == "\t" })
        {
          fence = nil
        }
        return .code
      }
      if let marker = content.first, marker == "`" || marker == "~" {
        let run = content.prefix { $0 == marker }
        if run.count >= 3, marker == "~" || !content.dropFirst(run.count).contains("`") {
          fence = (marker, run.count)
          return .code
        }
      }
      // A blank line ends a text run but not indented code.
      if blank { return inIndentedCode ? .code : .blank }

      // Indented code: four spaces after a blank line, outside lists (where four spaces are
      // usually a continuation). Anything this misses is put back by the renderer.
      let indented = line.hasPrefix("    ") || line.hasPrefix("\t")
      if indented, !inList, previousBlank || inIndentedCode {
        inIndentedCode = true
        return .code
      }
      inIndentedCode = false
      let bare = line.drop { $0 == " " || $0 == "\t" || $0 == ">" }
      if Self.listMarkerLength(bare) != nil {
        inList = true
      } else if previousBlank, !line.hasPrefix(" "), !line.hasPrefix("\t") {
        inList = false
      }
      return .text
    }

    /// Strips block-quote markers, list markers and indentation, for spotting fences.
    static func stripContainers(_ line: Substring) -> Substring {
      var rest = line
      while true {
        let before = rest
        rest = rest.drop { $0 == " " || $0 == "\t" }
        if rest.first == ">" { rest = rest.dropFirst() }
        if let length = listMarkerLength(rest) { rest = rest.dropFirst(length) }
        if rest == before { return rest }
      }
    }

    /// The length of a list marker and its space at the start of `text`, if there is one.
    static func listMarkerLength(_ text: Substring) -> Int? {
      guard let first = text.first else { return nil }
      if first == "-" || first == "+" || first == "*" {
        let next = text.dropFirst().first
        return next == " " || next == "\t" ? 2 : nil
      }
      let digits = text.prefix { $0.isASCII && $0.isNumber }
      guard !digits.isEmpty, digits.count <= 9 else { return nil }
      let rest = text.dropFirst(digits.count)
      guard let mark = rest.first, mark == "." || mark == ")" else { return nil }
      let next = rest.dropFirst().first
      return next == " " || next == "\t" ? digits.count + 2 : nil
    }

    /// Replaces math in a run of text lines, skipping code spans and escaped dollars.
    mutating func scanInline(_ text: String) -> String {
      guard text.containsBytes("$") else { return text }
      let chars = Array(text)
      var out = ""
      var index = 0
      var plainStart = 0

      func emit(_ end: Int) {
        if end > plainStart { out += String(chars[plainStart..<end]) }
      }

      while index < chars.count {
        let char = chars[index]
        if char == "\\" {
          index += 2
          continue
        }
        if char == "`" {
          let run = Self.runLength(chars, from: index, of: "`")
          if let close = Self.findRun(chars, from: index + run, of: "`", length: run) {
            index = close + run
          } else {
            index += run
          }
          continue
        }
        guard char == "$" else {
          index += 1
          continue
        }
        if let (end, tex, display) = matchMath(chars, at: index) {
          emit(index)
          let source = String(chars[index..<end])
          out += "\(MathSpans.open)\(spans.spans.count)\(MathSpans.close)"
          spans.spans.append(Span(tex: tex, display: display, source: source))
          index = end
          plainStart = end
        } else {
          // A run of dollars that isn't math stays as written.
          index += Self.runLength(chars, from: index, of: "$")
        }
      }
      emit(chars.count)
      return out
    }

    /// Matches math starting at a `$`, returning where it ends, its TeX and whether it's display.
    func matchMath(_ chars: [Character], at start: Int) -> (Int, String, Bool)? {
      let count = chars.count
      // $`…`$
      if start + 1 < count, chars[start + 1] == "`" {
        let run = Self.runLength(chars, from: start + 1, of: "`")
        var search = start + 1 + run
        while let close = Self.findRun(chars, from: search, of: "`", length: run) {
          if close + run < count, chars[close + run] == "$" {
            let tex = String(chars[(start + 1 + run)..<close])
              .trimmingCharacters(in: .whitespaces)
            guard !tex.isEmpty else { return nil }
            return (close + run + 1, tex, false)
          }
          search = close + run
        }
        return nil
      }
      // $$…$$
      if start + 1 < count, chars[start + 1] == "$" {
        var index = start + 2
        while index + 1 < count {
          if chars[index] == "\\" {
            index += 2
            continue
          }
          if chars[index] == "$", chars[index + 1] == "$" {
            let tex = displayTeX(chars, from: start + 2, to: index, opener: start)
            guard !tex.isEmpty else { return nil }
            return (index + 2, tex, true)
          }
          index += 1
        }
        return nil
      }
      // $…$
      guard start + 1 < count, !chars[start + 1].isWhitespace else { return nil }
      var index = start + 1
      while index < count {
        let char = chars[index]
        if char == "\n" { return nil }
        if char == "\\" {
          index += 2
          continue
        }
        if char == "$" {
          if chars[index - 1].isWhitespace {
            return nil
          }
          if index + 1 < count, chars[index + 1].isASCII, chars[index + 1].isNumber { return nil }
          return (index + 1, String(chars[(start + 1)..<index]), false)
        }
        index += 1
      }
      return nil
    }

    /// Display math's TeX, without the `>` markers of the quote it may sit in.
    func displayTeX(_ chars: [Character], from: Int, to: Int, opener: Int) -> String {
      var lineStart = opener
      while lineStart > 0, chars[lineStart - 1] != "\n" { lineStart -= 1 }
      let quoted = chars[lineStart..<opener].contains(">")
      let lines = String(chars[from..<to]).split(separator: "\n", omittingEmptySubsequences: false)
      var kept: [Substring] = []
      for (index, line) in lines.enumerated() {
        var line = line
        if index > 0, quoted {
          line = line.drop { $0 == " " || $0 == "\t" || $0 == ">" }
        }
        kept.append(line)
      }
      return kept.joined(separator: "\n").trimmingCharacters(in: .whitespacesAndNewlines)
    }

    static func runLength(_ chars: [Character], from start: Int, of char: Character) -> Int {
      var end = start
      while end < chars.count, chars[end] == char { end += 1 }
      return end - start
    }

    /// The next run of exactly `length` copies of `char` at or after `start`.
    static func findRun(_ chars: [Character], from start: Int, of char: Character, length: Int) -> Int? {
      var index = start
      while index < chars.count {
        if chars[index] == char {
          let run = runLength(chars, from: index, of: char)
          if run == length { return index }
          index += run
        } else {
          index += 1
        }
      }
      return nil
    }
  }
}

// MARK: - Rendering

extension MarkdownRenderer {
  /// Display math is set a little larger than body text, inline math a touch larger, since
  /// Latin Modern has a smaller x-height than the reader's fonts.
  static let inlineMathScale: CGFloat = 1.1
  static let displayMathScale: CGFloat = 1.2

  /// Appends one formula: a baseline-aligned image, or its TeX in the code font if it doesn't
  /// parse. Display math is pulled out into its own paragraph by `splitDisplayMath`.
  func appendMath(_ span: MathSpans.Span, _ inline: Inline, into text: NSMutableAttributedString) {
    let base = inline.superscript != 0 ? (inline.size * 0.75).rounded() : inline.size
    let scale = span.display ? Self.displayMathScale : Self.inlineMathScale
    let source = MathSource(tex: span.tex, display: span.display)
    // Formulas are sized to the column, so a resize swaps them like images.
    let slot = nextAssetSlot()
    switch MathFormula.make(
      span.tex, fontSize: (base * scale).rounded(), display: span.display, color: inline.color)
    {
    case .success(let formula):
      let attachment = NSTextAttachment()
      attachment.image = formula.image
      attachment.bounds = mathBounds(formula, maxWidth: style.maxContentWidth)
      var attributes: [NSAttributedString.Key: Any] = [
        .attachment: attachment, .mdowMathSource: source, .font: font(for: inline),
        .mdowAssetSlot: slot,
      ]
      if let link = inline.link { attributes[.link] = link }
      text.append(NSAttributedString(string: "\u{FFFC}", attributes: attributes))
    case .failure(let error):
      var code = inline
      code.code = true
      var attributes = self.attributes(code)
      attributes[.foregroundColor] = NSColor.systemRed
      attributes[.backgroundColor] = ReaderColors.dynamic(
        light: NSColor.systemRed.withAlphaComponent(0.07),
        dark: NSColor.systemRed.withAlphaComponent(0.14))
      attributes[.toolTip] = error.message
      attributes[.mdowMathSource] = source
      attributes[.mdowAssetSlot] = slot
      text.append(NSAttributedString(string: span.source, attributes: attributes))
    }
  }

  /// Where a formula's image goes relative to the baseline, shrunk to fit `maxWidth`.
  func mathBounds(_ formula: MathFormula, maxWidth: CGFloat) -> CGRect {
    let size = fitted(formula.size, maxWidth: maxWidth)
    let scale = formula.size.width > 0 ? size.width / formula.size.width : 1
    return CGRect(x: 0, y: -formula.baseline * scale, width: size.width, height: size.height)
  }

  /// A fenced ```` ```math ```` block. Returns false to fall back to showing its source.
  func renderDisplayMath(_ tex: String, _ ctx: Context) -> Bool {
    let tex = tex.trimmingCharacters(in: .whitespacesAndNewlines)
    guard !tex.isEmpty,
      case .success(let formula) = MathFormula.make(
        tex, fontSize: (style.bodySize * Self.displayMathScale).rounded(), display: true,
        color: ctx.color)
    else { return false }
    flushMarker(ctx)
    let attachment = NSTextAttachment()
    attachment.image = formula.image
    attachment.bounds = mathBounds(formula, maxWidth: style.maxContentWidth - ctx.indent)
    let text = NSAttributedString(
      string: "\u{FFFC}",
      attributes: [
        .attachment: attachment, .mdowMathSource: MathSource(tex: tex, display: true),
        .font: bodyFont, .mdowAssetSlot: nextAssetSlot(),
      ])
    append(text, displayMathStyle(ctx))
    return true
  }

  func displayMathStyle(_ ctx: Context) -> NSMutableParagraphStyle {
    let paragraph = paragraphStyle(ctx, spacing: max(ctx.spacing, style.blockSpacing))
    paragraph.alignment = .center
    paragraph.lineHeightMultiple = 1
    paragraph.paragraphSpacingBefore = (style.blockSpacing * 0.4).rounded()
    return paragraph
  }

  /// Splits a paragraph around its `$$…$$` formulas, which go on lines of their own, centred.
  /// Returns false, having appended nothing, when the paragraph has none.
  func splitDisplayMath(_ text: NSAttributedString, _ ctx: Context) -> Bool {
    var displays: [(NSRange, MathSource)] = []
    text.enumerateAttribute(.mdowMathSource, in: NSRange(location: 0, length: text.length)) {
      value, range, _ in
      if let source = value as? MathSource, source.display { displays.append((range, source)) }
    }
    guard !displays.isEmpty else { return false }

    func appendText(_ range: NSRange) {
      let piece = text.attributedSubstring(from: range)
      let trimmed = piece.string.trimmingCharacters(in: Self.mathSplitTrim)
      guard !trimmed.isEmpty else { return }
      // Trim the soft breaks and spaces that separated the text from the formula.
      let string = piece.string as NSString
      var start = 0
      var end = string.length
      while start < end, Self.isSplitTrim(string.character(at: start)) { start += 1 }
      while end > start, Self.isSplitTrim(string.character(at: end - 1)) { end -= 1 }
      append(
        piece.attributedSubstring(from: NSRange(location: start, length: end - start)),
        paragraphStyle(ctx))
    }

    var cursor = 0
    for (range, source) in displays {
      appendText(NSRange(location: cursor, length: range.location - cursor))
      cursor = NSMaxRange(range)
      let piece = NSMutableAttributedString(attributedString: text.attributedSubstring(from: range))
      guard let attachment = piece.attribute(.attachment, at: 0, effectiveRange: nil)
        as? NSTextAttachment, let image = attachment.image
      else {
        // It didn't typeset: show the TeX as a code block.
        renderCode(source.tex, info: "math", ctx)
        continue
      }
      flushMarker(ctx)
      let natural = attachment.bounds
      let fitted = fitted(natural.size, maxWidth: style.maxContentWidth - ctx.indent)
      if fitted.width < natural.width, image.size.width > 0 {
        let scale = fitted.width / natural.width
        attachment.bounds = CGRect(
          x: 0, y: natural.minY * scale, width: fitted.width, height: fitted.height)
      }
      append(piece, displayMathStyle(ctx))
    }
    appendText(NSRange(location: cursor, length: text.length - cursor))
    return true
  }

  static let mathSplitTrim = CharacterSet.whitespacesAndNewlines.union(
    CharacterSet(charactersIn: "\u{2028}"))

  static func isSplitTrim(_ unit: unichar) -> Bool {
    unit == 32 || unit == 9 || unit == 10 || unit == 0x2028
  }
}

// MARK: - Typesetting

public extension NSAttributedString.Key {
  /// A `MathSource` on each rendered formula: its attachment character, or the TeX shown in
  /// its place when it doesn't parse.
  ///
  /// NSTextView copies an attachment as U+FFFC in plain text. To copy formulas as TeX, the
  /// reader can override `writeSelection(to:type:)` and write `mdowPlainText(in:)` for
  /// `.string`.
  static let mdowMathSource = NSAttributedString.Key("mdow.mathSource")
}

/// The TeX behind a rendered formula.
public final class MathSource: NSObject, @unchecked Sendable {
  public let tex: String
  public let display: Bool

  public init(tex: String, display: Bool) {
    self.tex = tex
    self.display = display
  }

  /// The formula as markdown, delimiters included.
  public var markdown: String { display ? "$$\(tex)$$" : "$\(tex)$" }
}

public extension NSAttributedString {
  /// The text in `range` with each formula written out as its TeX source, for copying.
  func mdowPlainText(in range: NSRange) -> String {
    var out = ""
    let string = self.string as NSString
    enumerateAttribute(.mdowMathSource, in: range) { value, run, _ in
      if let math = value as? MathSource,
        attribute(.attachment, at: run.location, effectiveRange: nil) != nil
      {
        out += math.markdown
      } else {
        out += string.substring(with: run)
      }
    }
    return out
  }
}

/// A formula typeset by SwiftMath.
///
/// `image` draws through a handler that runs each time it's drawn and picks a typesetting in
/// the formula's colour as resolved for the appearance being drawn, so formulas follow light
/// and dark without the document being rendered again.
final class MathFormula: @unchecked Sendable {
  let tex: String
  let fontSize: CGFloat
  let display: Bool
  let color: NSColor
  /// The image's size, and its baseline measured up from the bottom.
  private(set) var size = CGSize.zero
  private(set) var baseline: CGFloat = 0
  private(set) var image = NSImage()

  private let lock = NSLock()
  /// Typesettings by resolved colour.
  private var variants: [ColorKey: NSImage] = [:]

  struct ColorKey: Hashable {
    var red, green, blue, alpha: Int

    /// Resolves `color` for the current drawing appearance.
    init(_ color: NSColor) {
      let rgb = color.usingColorSpace(.sRGB) ?? .black
      red = Int((rgb.redComponent * 255).rounded())
      green = Int((rgb.greenComponent * 255).rounded())
      blue = Int((rgb.blueComponent * 255).rounded())
      alpha = Int((rgb.alphaComponent * 255).rounded())
    }

    var color: NSColor {
      NSColor(
        srgbRed: CGFloat(red) / 255, green: CGFloat(green) / 255, blue: CGFloat(blue) / 255,
        alpha: CGFloat(alpha) / 255)
    }
  }

  struct MathError: Error {
    var message: String
  }

  private init(tex: String, fontSize: CGFloat, display: Bool, color: NSColor) {
    self.tex = tex
    self.fontSize = fontSize
    self.display = display
    self.color = color
  }

  /// SwiftMath's lazily built symbol tables aren't all behind locks, so typesetting is serial.
  private static let typesetLock = NSLock()

  private static func typeset(
    _ tex: String, fontSize: CGFloat, display: Bool, color: NSColor
  ) -> Result<(NSImage, MathImage.LayoutInfo), MathError> {
    let tex = compatible(tex)
    typesetLock.lock()
    defer { typesetLock.unlock() }
    var math = MathImage(
      latex: tex, fontSize: fontSize, textColor: color,
      labelMode: display ? .display : .text, textAlignment: .left)
    math.font = .latinModernFont
    let (error, image, layout) = math.asImage()
    guard let image, let layout, error == nil, image.size.width > 0, image.size.height > 0 else {
      return .failure(MathError(message: error?.localizedDescription ?? "Couldn't typeset this"))
    }
    image.cacheMode = .never
    return .success((image, layout))
  }

  /// Rewrites of commands GitHub's MathJax knows and SwiftMath doesn't, into ones it does.
  /// Close enough to read; spacing can differ slightly.
  private static let rewrites: [(NSRegularExpression, String)] = [
    (#"\\[dt]frac(?![a-zA-Z])"#, #"\\frac"#),
    (#"\\operatorname\*?\{([^{}]*)\}"#, #"\\mathrm{$1}"#),
    (#"\\boldsymbol(?![a-zA-Z])"#, #"\\bm"#),
    (#"\\varnothing(?![a-zA-Z])"#, #"\\emptyset"#),
    (#"\\not\s*="#, #"\\neq"#),
    (#"\\iiint(?![a-zA-Z])"#, #"\\int\\!\\!\\int\\!\\!\\int"#),
    (#"\\iint(?![a-zA-Z])"#, #"\\int\\!\\!\\int"#),
    (#"\\lt(?![a-zA-Z])"#, "<"),
    (#"\\gt(?![a-zA-Z])"#, ">"),
    (#"\\bmod(?![a-zA-Z])"#, #"\\;\\mathrm{mod}\\;"#),
    (#"\\pmod\{([^{}]*)\}"#, #"\\quad(\\mathrm{mod}\\ $1)"#),
    (#"\\[Bb]igg?[lrm]?(?![a-zA-Z])"#, ""),
    (#"\\(begin|end)\{(align\*?|split|eqnarray\*?|alignat\*?)\}"#, #"\\$1{aligned}"#),
    (#"\\(begin|end)\{gather\*\}"#, #"\\$1{gather}"#),
    (#"\\(begin|end)\{equation\*?\}"#, ""),
  ].map { (try! NSRegularExpression(pattern: $0.0), $0.1) }

  static func compatible(_ tex: String) -> String {
    guard tex.contains("\\") else { return tex }
    var tex = tex
    for (pattern, template) in rewrites {
      tex = pattern.stringByReplacingMatches(
        in: tex, range: NSRange(location: 0, length: (tex as NSString).length),
        withTemplate: template)
    }
    return tex
  }

  private final class Box {
    let result: Result<MathFormula, MathError>
    init(_ result: Result<MathFormula, MathError>) { self.result = result }
  }

  /// NSCache is thread-safe.
  nonisolated(unsafe) private static let cache: NSCache<NSString, Box> = {
    let cache = NSCache<NSString, Box>()
    cache.countLimit = 2000
    return cache
  }()

  /// Typesets `tex`, or says why it couldn't. Results are cached by TeX, size, mode and colour,
  /// and can be made and drawn on any thread.
  static func make(
    _ tex: String, fontSize: CGFloat, display: Bool, color: NSColor
  ) -> Result<MathFormula, MathError> {
    let key = "\(display ? "D" : "I")|\(fontSize)|\(ObjectIdentifier(color).hashValue)|\(tex)"
      as NSString
    if let box = cache.object(forKey: key) { return box.result }
    let formula = MathFormula(tex: tex, fontSize: fontSize, display: display, color: color)
    let result = formula.prepare().map { formula }
    cache.setObject(Box(result), forKey: key)
    return result
  }

  private func prepare() -> Result<Void, MathError> {
    let resolved = ColorKey(color)
    switch Self.typeset(tex, fontSize: fontSize, display: display, color: resolved.color) {
    case .failure(let error):
      return .failure(error)
    case .success(let (first, layout)):
      size = first.size
      // SwiftMath centres the formula vertically in its rounded-up image, giving it at least
      // half the font size of height.
      let height = max(layout.ascent + layout.descent, fontSize / 2)
      baseline = (size.height - height) / 2 + layout.descent
      variants[resolved] = first
      let image = NSImage(size: size, flipped: false) { [self] rect in
        variant(for: ColorKey(color))?.draw(in: rect)
        return true
      }
      image.cacheMode = .never
      image.accessibilityDescription = tex
      self.image = image
      return .success(())
    }
  }

  private func variant(for key: ColorKey) -> NSImage? {
    lock.lock()
    defer { lock.unlock() }
    if let image = variants[key] { return image }
    guard
      case .success(let (image, _)) = Self.typeset(
        tex, fontSize: fontSize, display: display, color: key.color)
    else { return nil }
    variants[key] = image
    return image
  }
}

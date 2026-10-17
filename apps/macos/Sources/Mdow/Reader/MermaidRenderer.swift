import AppKit
import CoreText
import JavaScriptCore

enum MermaidError: Error, Sendable {
  /// Pele's script couldn't be found or loaded.
  case unavailable
  /// The diagram didn't parse or lay out; the message is Pele's.
  case syntax(String)
}

/// Renders Mermaid diagrams to vector `NSImage`s with Pele (https://pele.run), which lays diagrams
/// out without a DOM, so it runs in a bare JavaScriptCore context on a background queue. The SVG is
/// themed with literal colours and loaded through AppKit's SVG image rep.
final class MermaidRenderer: Sendable {
  static let shared = MermaidRenderer()

  private let queue = DispatchQueue(label: "app.mdow.mermaid", qos: .userInitiated)
  private let engine = Engine()

  /// Renders a diagram; results are cached by (source, dark).
  func render(_ source: String, dark: Bool) async throws -> NSImage {
    let engine = engine
    return try await withCheckedThrowingContinuation { continuation in
      queue.async {
        continuation.resume(with: Result { try engine.render(source, dark: dark) })
      }
    }
  }
}

// MARK: - Engine

/// Everything mutable lives here and is only touched on the renderer's serial queue.
private final class Engine: @unchecked Sendable {
  private var context: JSContext?
  private var renderFunction: JSValue?
  private var loadFailed = false
  private var cache: [String: NSImage] = [:]
  private var cacheOrder: [String] = []
  private let cacheLimit = 64
  private var fonts: [FontKey: CTFont] = [:]

  func render(_ source: String, dark: Bool) throws -> NSImage {
    let key = (dark ? "d\n" : "l\n") + source
    if let hit = cache[key] { return hit }

    let palette = dark ? Palette.dark : Palette.light
    let svg = try layout(source.trimmingCharacters(in: .whitespacesAndNewlines), palette: palette)
    let resolved = SVGTheming.resolve(svg.markup, variables: palette.variables)
    let flattened = SVGTheming.flattenText(resolved, fontSize: Palette.fontSize) {
      self.width(of: $0, size: $1, style: $2)
    }
    let themed = SVGTheming.normalizeFontWeights(flattened)
    guard let image = NSImage(data: Data(themed.utf8)), image.isValid else {
      throw MermaidError.syntax("The diagram could not be drawn")
    }
    image.size = NSSize(width: svg.width, height: svg.height)
    image.accessibilityDescription = "Mermaid diagram"

    cache[key] = image
    cacheOrder.append(key)
    if cacheOrder.count > cacheLimit { cache[cacheOrder.removeFirst()] = nil }
    return image
  }

  private func layout(
    _ source: String,
    palette: Palette
  ) throws -> (markup: String, width: Double, height: Double) {
    guard let context = loadedContext(), let renderFunction else { throw MermaidError.unavailable }
    context.exception = nil
    let options: [String: Any] = [
      "fontFamily": Palette.font,
      "fontFamilyMono": Palette.monoFont,
      "fontSize": Palette.fontSize,
      // Keep the author's direction; the reader column decides how wide the image is drawn.
      "autoDirection": false,
      "responsive": false,
    ]
    let result = renderFunction.call(withArguments: [source, options])
    if let exception = context.exception {
      context.exception = nil
      let message = exception.objectForKeyedSubscript("message")?.toString()
      throw MermaidError.syntax(message ?? exception.toString() ?? "Invalid diagram")
    }
    guard let result, let markup = result.objectForKeyedSubscript("svg")?.toString() else {
      throw MermaidError.syntax("Invalid diagram")
    }
    let width = result.objectForKeyedSubscript("width")?.toDouble() ?? 0
    let height = result.objectForKeyedSubscript("height")?.toDouble() ?? 0
    guard width > 0, height > 0, width.isFinite, height.isFinite else {
      throw MermaidError.syntax("The diagram has no size")
    }
    return (markup, width, height)
  }

  private func loadedContext() -> JSContext? {
    if let context { return context }
    if loadFailed { return nil }
    loadFailed = true
    guard let url = Self.scriptURL(),
      let script = try? String(contentsOf: url, encoding: .utf8),
      let context = JSContext()
    else { return nil }
    context.name = "Mdow Mermaid"

    // Text widths come from Core Text, so labels fit the system font the SVG is drawn with.
    let measure: @convention(block) (String, Double, Int) -> Double = { [unowned self] in
      self.width(of: $0, size: $1, style: $2)
    }
    context.setObject(measure, forKeyedSubscript: "__mdowMeasure" as NSString)
    context.evaluateScript(script, withSourceURL: url)
    context.evaluateScript(
      """
      var __mdowMeasurer = { width: function (t, s, st) { return __mdowMeasure(t, s, st | 0) } };
      function __mdowRender(source, options) {
        options.measurer = __mdowMeasurer;
        return pele.render(source, options);
      }
      """)
    guard context.exception == nil,
      let function = context.objectForKeyedSubscript("__mdowRender"), !function.isUndefined
    else { return nil }

    loadFailed = false
    self.context = context
    renderFunction = function
    return context
  }

  /// The bundled copy in Contents/Resources, else the one `scripts/fetch-pele.sh` downloads.
  private static func scriptURL() -> URL? {
    if let url = Bundle.main.url(forResource: "pele.global", withExtension: "js") { return url }
    let dev = URL(fileURLWithPath: #filePath)
      .deletingLastPathComponent()  // Reader
      .deletingLastPathComponent()  // Mdow
      .deletingLastPathComponent()  // Sources
      .deletingLastPathComponent()  // apps/macos
      .appendingPathComponent("Resources/vendor/pele.global.js")
    return FileManager.default.fileExists(atPath: dev.path) ? dev : nil
  }

  // MARK: Text measurement

  private struct FontKey: Hashable {
    let size: Double
    let style: Int
  }

  /// Pele's style bits: 1 bold, 2 italic, 4 monospace. Italic is measured upright because
  /// AppKit's SVG renderer draws it upright.
  private func width(of text: String, size: Double, style: Int) -> Double {
    guard !text.isEmpty, size > 0 else { return 0 }
    let string = NSAttributedString(
      string: text,
      attributes: [.font: font(size: size, style: style)]
    )
    let line = CTLineCreateWithAttributedString(string)
    return CTLineGetTypographicBounds(line, nil, nil, nil)
  }

  private func font(size: Double, style: Int) -> CTFont {
    let key = FontKey(size: size, style: style)
    if let font = fonts[key] { return font }
    let weight: NSFont.Weight = style & 1 != 0 ? .bold : .regular
    let font: NSFont =
      style & 4 != 0
      ? .monospacedSystemFont(ofSize: size, weight: weight)
      : .systemFont(ofSize: size, weight: weight)
    fonts[key] = font
    return font
  }
}

// MARK: - Palette

/// Literal colours for Pele's `--pele-*` variables, tuned to the macOS system palette.
private struct Palette {
  // AppKit's SVG renderer takes a single family name (no fallback lists, no `-apple-system`);
  // these are the system UI fonts' family names, and anything unknown falls back to Helvetica.
  static let font = ".SF NS"
  static let monoFont = ".SF NS Mono"
  static let fontSize = 14.0

  let bg, fg, muted, line, surface, surfaceAlt, border, accent, critical: String
  let series: [String]

  static let light = Palette(
    bg: "#ffffff", fg: "#1d1d1f", muted: "#6e6e73", line: "#8e8e93", surface: "#f2f2f7",
    surfaceAlt: "#fafafa", border: "#d1d1d6", accent: "#007aff", critical: "#ff3b30",
    series: ["#007aff", "#ff9500", "#34c759", "#ff3b30", "#30b0c7", "#ffcc00", "#af52de", "#a2845e"]
  )

  static let dark = Palette(
    bg: "#1e1e1e", fg: "#f5f5f7", muted: "#a1a1a6", line: "#8e8e93", surface: "#2c2c2e",
    surfaceAlt: "#242426", border: "#3a3a3c", accent: "#0a84ff", critical: "#ff453a",
    series: ["#0a84ff", "#ff9f0a", "#30d158", "#ff453a", "#40c8e0", "#ffd60a", "#bf5af2", "#ac8e68"]
  )

  var variables: [String: String] {
    var vars = [
      "--pele-bg": bg, "--pele-fg": fg, "--pele-muted": muted, "--pele-line": line,
      "--pele-surface": surface, "--pele-surface-alt": surfaceAlt, "--pele-border": border,
      "--pele-accent": accent, "--pele-critical": critical, "--pele-error": critical,
      "--pele-font": Self.font, "--pele-font-mono": Self.monoFont, "--pele-radius": "6px",
    ]
    for (index, color) in series.enumerated() { vars["--pele-series-\(index + 1)"] = color }
    return vars
  }
}

// MARK: - SVG theming

/// AppKit's SVG renderer can't resolve CSS custom properties or `color-mix()`, so this inlines
/// them: every `var(--x)` becomes the palette value (or the root `<svg>`'s declaration, or the
/// fallback) and `style="rx:…"` becomes an `rx` attribute.
enum SVGTheming {
  static func resolve(_ svg: String, variables: [String: String]) -> String {
    guard let rootRange = rootTagRange(in: svg) else { return svg }
    var root = String(svg[rootRange])

    // Custom properties the root declares, e.g. `--_fg:var(--pele-fg,#1f1f1f)`.
    var declared: [String: String] = [:]
    var kept: [String] = []
    if let style = attribute("style", in: root) {
      for declaration in splitTopLevel(style.value, on: ";") {
        let trimmed = declaration.trimmingCharacters(in: .whitespaces)
        guard !trimmed.isEmpty else { continue }
        if trimmed.hasPrefix("--"), let colon = trimmed.firstIndex(of: ":") {
          let name = trimmed[..<colon].trimmingCharacters(in: .whitespaces)
          declared[name] = trimmed[trimmed.index(after: colon)...]
            .trimmingCharacters(in: .whitespaces)
        } else {
          kept.append(trimmed)
        }
      }
      root.replaceSubrange(style.range, with: "")
    }

    func value(_ input: String, depth: Int = 0) -> String {
      guard depth < 8 else { return input }
      let resolved = replaceCalls(named: "var", in: input) { inner in
        let parts = splitTopLevel(inner, on: ",", limit: 1)
        let name = parts[0].trimmingCharacters(in: .whitespaces)
        let fallback = parts.count > 1 ? parts[1].trimmingCharacters(in: .whitespaces) : ""
        return value(variables[name] ?? declared[name] ?? fallback, depth: depth + 1)
      }
      return replaceCalls(named: "color-mix", in: resolved) { colorMix($0) ?? "#808080" }
    }

    // Spell the font out as attributes too: the `font` shorthand isn't reliably honoured.
    var extra = ""
    // The font goes in attributes below; a `font-family` in the style would override them.
    let resolvedStyle = kept.map { value($0) }
      .filter { !$0.hasPrefix("font:") && !$0.hasPrefix("font-family:") }
    if !resolvedStyle.isEmpty {
      extra += " style=\"\(escape(resolvedStyle.joined(separator: ";")))\""
    }
    if let family = variables["--pele-font"], attribute("font-family", in: root) == nil {
      extra += " font-family=\"\(escape(family))\""
    }
    if attribute("font-size", in: root) == nil,
      let font = kept.first(where: { $0.hasPrefix("font:") }),
      let size = font.dropFirst(5).split(separator: " ").first(where: { $0.hasSuffix("px") })
    {
      extra += " font-size=\"\(size.dropLast(2))\""
    }
    if let close = root.lastIndex(of: ">") {
      let insertAt = root[root.index(before: close)] == "/" ? root.index(before: close) : close
      root.insert(contentsOf: extra, at: insertAt)
    }

    var output = svg
    output.replaceSubrange(rootRange, with: root)
    return rewriteAttributes(in: output) { value($0) }
  }

  /// Resolves `var()` in every attribute value and turns `style="rx:N"` into `rx="N"`.
  private static func rewriteAttributes(
    in svg: String,
    resolve: (String) -> String
  ) -> String {
    var output = ""
    output.reserveCapacity(svg.utf8.count)
    var rest = svg[...]
    while let open = rest.range(of: "=\"") {
      output += rest[..<open.lowerBound]
      guard let close = rest[open.upperBound...].firstIndex(of: "\"") else {
        rest = rest[open.lowerBound...]
        break
      }
      let raw = String(rest[open.upperBound..<close])
      let name = attributeName(endingAt: output)
      if name == "style", raw.hasPrefix("rx:"), !raw.dropLast().contains(";") {
        // `style="rx:var(--_r)"` → `rx="6"`.
        output.removeLast(name.count)
        let rx = resolve(String(raw.dropFirst(3)))
          .trimmingCharacters(in: CharacterSet(charactersIn: " ;"))
        output += "rx=\"\(rx.hasSuffix("px") ? String(rx.dropLast(2)) : rx)\""
      } else if raw.contains("var(") || raw.contains("color-mix(") {
        output += "=\"\(escape(unescape(resolve(unescape(raw)))))\""
      } else {
        output += "=\"\(raw)\""
      }
      rest = rest[rest.index(after: close)...]
    }
    output += rest
    return output
  }

  private static func attributeName(endingAt text: String) -> String {
    String(text.reversed().prefix { $0.isLetter || $0.isNumber || $0 == "-" || $0 == ":" }
      .reversed())
  }

  private static func rootTagRange(in svg: String) -> Range<String.Index>? {
    guard let start = svg.range(of: "<svg") else { return nil }
    var quoted = false
    var index = start.upperBound
    while index < svg.endIndex {
      let char = svg[index]
      if char == "\"" { quoted.toggle() }
      if char == ">", !quoted { return start.lowerBound..<svg.index(after: index) }
      index = svg.index(after: index)
    }
    return nil
  }

  private static func attribute(
    _ name: String,
    in tag: String
  ) -> (value: String, range: Range<String.Index>)? {
    guard let start = tag.range(of: " \(name)=\"") else { return nil }
    guard let end = tag[start.upperBound...].firstIndex(of: "\"") else { return nil }
    return (unescape(String(tag[start.upperBound..<end])), start.lowerBound..<tag.index(after: end))
  }

  /// Replaces each `fn(…)` call, innermost first, with `map(inner)`.
  private static func replaceCalls(
    named fn: String,
    in input: String,
    map: (String) -> String
  ) -> String {
    let open = fn + "("
    var output = ""
    var rest = input[...]
    while let start = rest.range(of: open) {
      // Skip matches that are the tail of a longer identifier.
      if start.lowerBound > rest.startIndex {
        let before = rest[rest.index(before: start.lowerBound)]
        if before.isLetter || before == "-" {
          output += rest[..<start.upperBound]
          rest = rest[start.upperBound...]
          continue
        }
      }
      var depth = 0
      var end = start.upperBound
      while end < rest.endIndex {
        let char = rest[end]
        if char == "(" {
          depth += 1
        } else if char == ")" {
          if depth == 0 { break }
          depth -= 1
        }
        end = rest.index(after: end)
      }
      let inner = String(rest[start.upperBound..<end])
      output += rest[..<start.lowerBound]
      output += map(replaceCalls(named: fn, in: inner, map: map))
      rest = end < rest.endIndex ? rest[rest.index(after: end)...] : rest[end...]
    }
    return output + rest
  }

  private static func splitTopLevel(
    _ value: String,
    on separator: Character,
    limit: Int = .max
  ) -> [String] {
    var parts: [String] = []
    var depth = 0
    var current = ""
    for char in value {
      if char == "(" { depth += 1 }
      if char == ")" { depth -= 1 }
      if char == separator, depth == 0, parts.count < limit {
        parts.append(current)
        current = ""
      } else {
        current.append(char)
      }
    }
    parts.append(current)
    return parts
  }

  /// `in srgb,#aaa 40%,#bbb` → hex.
  private static func colorMix(_ inner: String) -> String? {
    let parts = splitTopLevel(inner, on: ",").map { $0.trimmingCharacters(in: .whitespaces) }
    guard parts.count == 3 else { return nil }
    func split(_ part: String) -> (color: [Double], percent: Double?)? {
      let pieces = part.split(separator: " ")
      guard let first = pieces.first, let color = parseHex(String(first)) else { return nil }
      let percent = pieces.count > 1 ? Double(pieces[1].dropLast()).map { $0 / 100 } : nil
      return (color, percent)
    }
    guard let a = split(parts[1]), let b = split(parts[2]) else { return nil }
    let weight = a.percent ?? b.percent.map { 1 - $0 } ?? 0.5
    return toHex(zip(a.color, b.color).map { $0 * weight + $1 * (1 - weight) })
  }

  private static func parseHex(_ color: String) -> [Double]? {
    var hex = color.hasPrefix("#") ? String(color.dropFirst()) : color
    guard hex.allSatisfy(\.isHexDigit) else { return nil }
    if hex.count == 3 || hex.count == 4 { hex = hex.map { "\($0)\($0)" }.joined() }
    guard hex.count == 6 || hex.count == 8 else { return nil }
    var channels: [Double] = stride(from: 0, to: hex.count, by: 2).map { offset in
      let start = hex.index(hex.startIndex, offsetBy: offset)
      return Double(Int(hex[start..<hex.index(start, offsetBy: 2)], radix: 16) ?? 0)
    }
    if channels.count == 3 { channels.append(255) }
    return channels
  }

  private static func toHex(_ channels: [Double]) -> String {
    let values = channels.map { Int(max(0, min(255, $0.rounded()))) }
    let rgb = values.prefix(3).map { String(format: "%02x", $0) }.joined()
    return values[3] == 255 ? "#\(rgb)" : "#\(rgb)\(String(format: "%02x", values[3]))"
  }

  // MARK: Text

  /// AppKit's SVG renderer ignores `bold` and draws 700 as black; 600 comes out as bold.
  static func normalizeFontWeights(_ svg: String) -> String {
    svg.replacing(#/font-weight(="|:\s*)(bold|bolder|[7-9]00)/#) { match in
      "font-weight\(match.output.1)600"
    }
  }

  /// AppKit's SVG renderer ignores `<tspan>` positions and styles, drawing every span as one run
  /// at the `<text>` origin. This splits each `<text>` that has spans into one `<text>` per run:
  /// lines keep their `x`/`y`, and runs within a line are placed with measured widths.
  static func flattenText(
    _ svg: String,
    fontSize: Double,
    measure: (String, Double, Int) -> Double
  ) -> String {
    var output = ""
    output.reserveCapacity(svg.utf8.count)
    var sizes = [fontSize]  // Inherited font-size per open element.
    var index = svg.startIndex
    while let open = svg[index...].firstIndex(of: "<") {
      output += svg[index..<open]
      guard let close = svg[open...].firstIndex(of: ">") else {
        index = open
        break
      }
      let tag = String(svg[open...close])
      index = svg.index(after: close)

      if tag.hasPrefix("</") {
        if sizes.count > 1 { sizes.removeLast() }
        output += tag
      } else if tag.hasPrefix("<text"), !tag.hasSuffix("/>"),
        let end = svg.range(of: "</text>", range: index..<svg.endIndex)
      {
        let body = String(svg[index..<end.lowerBound])
        let size = attribute("font-size", in: tag).flatMap { Double($0.value) } ?? sizes.last!
        output +=
          splitSpans(tag: tag, body: body, fontSize: size, measure: measure)
          ?? tag + body + "</text>"
        index = end.upperBound
      } else {
        if !tag.hasSuffix("/>"), !tag.hasPrefix("<?"), !tag.hasPrefix("<!") {
          sizes.append(attribute("font-size", in: tag).flatMap { Double($0.value) } ?? sizes.last!)
        }
        output += tag
      }
    }
    return output + svg[index...]
  }

  private struct Run {
    var attributes: [(String, String)]
    var text: String

    func value(_ name: String) -> String? { attributes.last { $0.0 == name }?.1 }
  }

  private static func splitSpans(
    tag: String,
    body: String,
    fontSize: Double,
    measure: (String, Double, Int) -> Double
  ) -> String? {
    guard body.contains("<tspan") else { return nil }

    // Lines of runs; a span with x or y starts a new line.
    var lines: [(x: String?, y: String?, runs: [Run])] = []
    var rest = body[...]
    while !rest.isEmpty {
      if rest.hasPrefix("<tspan") {
        guard let headEnd = rest.firstIndex(of: ">"),
          let spanEnd = rest.range(of: "</tspan>", range: headEnd..<rest.endIndex)
        else { return nil }
        let text = rest[rest.index(after: headEnd)..<spanEnd.lowerBound]
        if text.contains("<") { return nil }  // Nested markup: leave it alone.
        var attributes = attributes(in: String(rest[...headEnd]))
        let x = attributes.last { $0.0 == "x" }?.1
        let y = attributes.last { $0.0 == "y" }?.1
        attributes.removeAll { $0.0 == "x" || $0.0 == "y" }
        let run = Run(attributes: attributes, text: String(text))
        if x != nil || y != nil || lines.isEmpty {
          let previous = lines.last
          lines.append((x ?? previous?.x, y ?? previous?.y, [run]))
        } else {
          lines[lines.count - 1].runs.append(run)
        }
        rest = rest[spanEnd.upperBound...]
      } else {
        let next = rest.range(of: "<")?.lowerBound ?? rest.endIndex
        if next == rest.startIndex { return nil }
        let run = Run(attributes: [], text: String(rest[..<next]))
        if lines.isEmpty { lines.append((nil, nil, [])) }
        lines[lines.count - 1].runs.append(run)
        rest = rest[next...]
      }
    }

    let base = attributes(in: tag).filter { $0.0 != "x" && $0.0 != "y" }
    let baseRun = Run(attributes: base, text: "")
    let anchor = baseRun.value("text-anchor") ?? "start"
    var output = ""

    func element(_ attributes: [(String, String)], x: String?, y: String?, text: String) -> String {
      var merged: [(String, String)] = []
      for pair in attributes where !merged.contains(where: { $0.0 == pair.0 }) {
        merged.append(attributes.last { $0.0 == pair.0 }!)
      }
      if let x { merged.append(("x", x)) }
      if let y { merged.append(("y", y)) }
      let list = merged.map { " \($0.0)=\"\($0.1)\"" }.joined()
      return "<text\(list)>\(text)</text>"
    }

    for line in lines {
      let runs = line.runs.filter { !$0.text.isEmpty }
      if runs.count <= 1 {
        let run = runs.first ?? Run(attributes: [], text: "")
        output += element(base + run.attributes, x: line.x, y: line.y, text: run.text)
        continue
      }
      // Several styled runs on one line: lay them out left to right ourselves.
      // Each run is drawn trimmed (the renderer drops edge spaces), offset by its leading space.
      let metrics = runs.map { run -> (width: Double, lead: Double) in
        let merged = Run(attributes: base + run.attributes, text: run.text)
        let size = merged.value("font-size").flatMap(Double.init) ?? fontSize
        let text = unescape(run.text)
        let leading = String(text.prefix { $0 == " " })
        return (measure(text, size, styleBits(merged)), measure(leading, size, styleBits(merged)))
      }
      let total = metrics.reduce(0) { $0 + $1.width }
      var cursor = Double(line.x ?? "0") ?? 0
      if anchor == "middle" { cursor -= total / 2 }
      if anchor == "end" { cursor -= total }
      for (run, metric) in zip(runs, metrics) {
        let text = run.text.trimmingCharacters(in: .whitespaces)
        if !text.isEmpty {
          let attributes = base.filter { $0.0 != "text-anchor" } + run.attributes
            + [("text-anchor", "start")]
          output += element(attributes, x: format(cursor + metric.lead), y: line.y, text: text)
        }
        cursor += metric.width
      }
    }
    return output
  }

  /// Pele's measurer style bits: 1 bold, 2 italic, 4 monospace.
  private static func styleBits(_ run: Run) -> Int {
    var bits = 0
    if let weight = run.value("font-weight"),
      weight.hasPrefix("bold") || (Int(weight) ?? 400) >= 600
    {
      bits |= 1
    }
    if let style = run.value("font-style"), style == "italic" || style == "oblique" { bits |= 2 }
    if let family = run.value("font-family")?.lowercased(),
      family.contains("mono") || family.contains("menlo")
    {
      bits |= 4
    }
    return bits
  }

  private static func attributes(in tag: String) -> [(String, String)] {
    tag.matches(of: #/([\w:-]+)="([^"]*)"/#).map { (String($0.output.1), String($0.output.2)) }
  }

  private static func format(_ value: Double) -> String {
    String(format: "%.2f", value)
  }

  private static func escape(_ value: String) -> String {
    value.replacingOccurrences(of: "&", with: "&amp;")
      .replacingOccurrences(of: "\"", with: "&quot;")
      .replacingOccurrences(of: "<", with: "&lt;")
  }

  private static func unescape(_ value: String) -> String {
    value.replacingOccurrences(of: "&quot;", with: "\"")
      .replacingOccurrences(of: "&#39;", with: "'")
      .replacingOccurrences(of: "&lt;", with: "<")
      .replacingOccurrences(of: "&gt;", with: ">")
      .replacingOccurrences(of: "&amp;", with: "&")
  }
}

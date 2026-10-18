import Foundation

/// Just enough HTML for what READMEs embed: centred logos, badges, `<br>`, `<kbd>`, and
/// `<details>`. Raw HTML is never executed or laid out by a browser engine.
enum HTMLSnippet {
  struct ImageTag {
    var src: String
    var alt: String
    var width: CGFloat?
    var height: CGFloat?
    var link: URL?
  }

  struct Tag {
    var name: String?
    var closing: Bool
  }

  static func tag(_ raw: String) -> Tag {
    var body = raw.trimmingCharacters(in: .whitespacesAndNewlines)
    guard body.hasPrefix("<"), !body.hasPrefix("<!") else { return Tag(name: nil, closing: false) }
    body.removeFirst()
    let closing = body.hasPrefix("/")
    if closing { body.removeFirst() }
    let name = body.prefix { $0.isLetter || $0.isNumber }.lowercased()
    return Tag(name: name.isEmpty ? nil : name, closing: closing)
  }

  static func attribute(_ name: String, in tag: String) -> String? {
    let lower = tag.lowercased()
    var search = lower.startIndex
    while let range = lower.range(of: name, range: search..<lower.endIndex) {
      search = range.upperBound
      // Make sure it's the whole attribute name, e.g. not `data-src` for `src`.
      if range.lowerBound > lower.startIndex {
        let before = lower[lower.index(before: range.lowerBound)]
        if !before.isWhitespace { continue }
      }
      var index = range.upperBound
      while index < lower.endIndex, lower[index].isWhitespace { index = lower.index(after: index) }
      guard index < lower.endIndex, lower[index] == "=" else { continue }
      index = lower.index(after: index)
      while index < lower.endIndex, lower[index].isWhitespace { index = lower.index(after: index) }
      guard index < lower.endIndex else { return nil }
      let quote = lower[index]
      if quote == "\"" || quote == "'" {
        let start = tag.index(after: index)
        guard let end = tag[start...].firstIndex(of: quote) else { return nil }
        return decodeEntities(String(tag[start..<end]))
      }
      let end = tag[index...].firstIndex { $0.isWhitespace || $0 == ">" } ?? tag.endIndex
      return decodeEntities(String(tag[index..<end]))
    }
    return nil
  }

  static func images(in html: String) -> [ImageTag] {
    var images: [ImageTag] = []
    var search = html.startIndex
    while let open = html.range(of: "<img", options: .caseInsensitive, range: search..<html.endIndex) {
      guard let close = html[open.upperBound...].firstIndex(of: ">") else { break }
      let tag = String(html[open.lowerBound...close])
      search = html.index(after: close)
      guard let src = attribute("src", in: tag) else { continue }
      // A wrapping <a href> makes the image a link.
      var link: URL?
      if let anchor = html[..<open.lowerBound].range(of: "<a ", options: [.caseInsensitive, .backwards]),
        !html[anchor.upperBound..<open.lowerBound].lowercased().contains("</a>"),
        let href = attribute("href", in: String(html[anchor.lowerBound...]))
      {
        link = URL(string: href)
      }
      images.append(
        ImageTag(
          src: src, alt: attribute("alt", in: tag) ?? "",
          width: attribute("width", in: tag).flatMap(points),
          height: attribute("height", in: tag).flatMap(points), link: link))
    }
    return images
  }

  static func points(_ value: String) -> CGFloat? {
    let number = value.trimmingCharacters(in: .whitespaces).replacingOccurrences(of: "px", with: "")
    guard !number.hasSuffix("%"), let double = Double(number), double > 0 else { return nil }
    return CGFloat(double)
  }

  static func isCentered(_ html: String) -> Bool {
    let lower = html.lowercased()
    return lower.contains("align=\"center\"") || lower.contains("align='center'")
      || lower.contains("align=center") || lower.contains("<center")
      || lower.contains("text-align: center") || lower.contains("text-align:center")
  }

  static func removingImages(_ html: String) -> String {
    var out = ""
    var rest = html[...]
    while let open = rest.range(of: "<img", options: .caseInsensitive) {
      out += rest[..<open.lowerBound]
      guard let close = rest[open.upperBound...].firstIndex(of: ">") else { return out }
      rest = rest[rest.index(after: close)...]
    }
    return out + rest
  }

  /// The text of an HTML fragment, with block-level tags turned into line breaks.
  static func text(in html: String) -> String {
    let breaking: Set<String> = [
      "br", "p", "div", "li", "tr", "summary", "h1", "h2", "h3", "h4", "h5", "h6", "details",
      "table", "ul", "ol", "blockquote", "hr", "center", "section", "header", "footer",
    ]
    let hidden: Set<String> = ["script", "style", "template", "head", "title"]
    var out = ""
    var hiddenDepth = 0
    var index = html.startIndex
    while index < html.endIndex {
      let character = html[index]
      if character == "<" {
        if html[index...].hasPrefix("<!--") {
          index = html.range(of: "-->", range: index..<html.endIndex)?.upperBound ?? html.endIndex
          continue
        }
        guard let close = html[index...].firstIndex(of: ">") else { break }
        let tag = tag(String(html[index...close]))
        if let name = tag.name {
          if hidden.contains(name) { hiddenDepth = max(0, hiddenDepth + (tag.closing ? -1 : 1)) }
          if breaking.contains(name) { out += "\n" }
        }
        index = html.index(after: close)
        continue
      }
      if hiddenDepth == 0 { out.append(character) }
      index = html.index(after: index)
    }
    return decodeEntities(out)
      .components(separatedBy: "\n")
      .map { $0.split(whereSeparator: \.isWhitespace).joined(separator: " ") }
      .filter { !$0.isEmpty }
      .joined(separator: "\n")
  }

  static func decodeEntities(_ string: String) -> String {
    guard string.contains("&") else { return string }
    let named: [String: String] = [
      "amp": "&", "lt": "<", "gt": ">", "quot": "\"", "apos": "'", "nbsp": "\u{00A0}",
      "copy": "©", "reg": "®", "trade": "™", "mdash": "—", "ndash": "–", "hellip": "…",
      "middot": "·", "bull": "•", "rarr": "→", "larr": "←", "times": "×",
    ]
    var out = ""
    var rest = string[...]
    while let amp = rest.firstIndex(of: "&") {
      out += rest[..<amp]
      guard let semi = rest[amp...].firstIndex(of: ";"), rest.distance(from: amp, to: semi) <= 10 else {
        out.append("&")
        rest = rest[rest.index(after: amp)...]
        continue
      }
      let entity = rest[rest.index(after: amp)..<semi]
      if let value = named[String(entity)] {
        out += value
      } else if entity.hasPrefix("#x") || entity.hasPrefix("#X"),
        let code = UInt32(entity.dropFirst(2), radix: 16), let scalar = Unicode.Scalar(code)
      {
        out.unicodeScalars.append(scalar)
      } else if entity.hasPrefix("#"), let code = UInt32(entity.dropFirst()), let scalar = Unicode.Scalar(code) {
        out.unicodeScalars.append(scalar)
      } else {
        out += rest[amp...semi]
      }
      rest = rest[rest.index(after: semi)...]
    }
    return out + rest
  }
}

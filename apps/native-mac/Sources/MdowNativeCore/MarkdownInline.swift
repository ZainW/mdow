import Foundation

public enum MarkdownInline {
    public static func normalizedMarkdown(_ source: String) -> String {
        normalizeInlineMath(normalizedBareURLs(normalizedFootnoteReferences(normalizedWikiLinks(source))))
    }

    private static func normalizedWikiLinks(_ source: String) -> String {
        var result = ""
        var remaining = source[...]

        while let openRange = remaining.range(of: "[[") {
            result += remaining[..<openRange.lowerBound]

            let linkBodyStart = openRange.upperBound
            guard let closeRange = remaining[linkBodyStart...].range(of: "]]") else {
                result += remaining[openRange.lowerBound...]
                return result
            }

            let body = String(remaining[linkBodyStart..<closeRange.lowerBound])
            let parts = body.split(separator: "|", maxSplits: 1, omittingEmptySubsequences: false)
            let target = String(parts.first ?? "")
            let label = parts.count == 2 ? String(parts[1]) : target

            if target.isEmpty || label.isEmpty {
                result += remaining[openRange.lowerBound..<closeRange.upperBound]
            } else {
                result += "[\(escapedMarkdownLabel(label))](\(escapedLinkTarget(target)))"
            }

            remaining = remaining[closeRange.upperBound...]
        }

        result += remaining
        return result
    }

    private static func normalizeInlineMath(_ source: String) -> String {
        var result = ""
        var index = source.startIndex

        while index < source.endIndex {
            let character = source[index]

            if character == "\\" {
                let nextIndex = source.index(after: index)
                result.append(character)
                if nextIndex < source.endIndex {
                    result.append(source[nextIndex])
                    index = source.index(after: nextIndex)
                } else {
                    index = nextIndex
                }
                continue
            }

            guard character == "$",
                  let closeIndex = closingMathDelimiter(in: source, after: source.index(after: index))
            else {
                result.append(character)
                index = source.index(after: index)
                continue
            }

            let mathStart = source.index(after: index)
            let math = String(source[mathStart..<closeIndex])
            result += "`\(escapedInlineCode(math))`"
            index = source.index(after: closeIndex)
        }

        return result
    }

    private static func normalizedBareURLs(_ source: String) -> String {
        var result = ""
        var index = source.startIndex

        while index < source.endIndex {
            if source[index] == "`" {
                let codeEnd = source[source.index(after: index)...].firstIndex(of: "`")
                let end = codeEnd.map { source.index(after: $0) } ?? source.endIndex
                result += source[index..<end]
                index = end
                continue
            }

            if isBareURLStart(in: source, at: index),
               !isExistingMarkdownLinkTarget(in: source, at: index) {
                let consumed = consumeBareURL(in: source, from: index)
                result += "[\(consumed.url)](\(consumed.url))"
                result += consumed.trailingPunctuation
                index = consumed.endIndex
                continue
            }

            result.append(source[index])
            index = source.index(after: index)
        }

        return result
    }

    private static func normalizedFootnoteReferences(_ source: String) -> String {
        var result = ""
        var index = source.startIndex

        while index < source.endIndex {
            if source[index] == "\\" {
                let nextIndex = source.index(after: index)
                result.append(source[index])
                if nextIndex < source.endIndex {
                    result.append(source[nextIndex])
                    index = source.index(after: nextIndex)
                } else {
                    index = nextIndex
                }
                continue
            }

            if source[index] == "`" {
                let codeEnd = source[source.index(after: index)...].firstIndex(of: "`")
                let end = codeEnd.map { source.index(after: $0) } ?? source.endIndex
                result += source[index..<end]
                index = end
                continue
            }

            if source[index] == "[",
               let closeBracket = source[index...].firstIndex(of: "]") {
                if let existingLinkEnd = existingMarkdownLinkEnd(
                    in: source,
                    closeBracket: closeBracket
                ) {
                    result += source[index..<existingLinkEnd]
                    index = existingLinkEnd
                    continue
                }

                if source[index...].hasPrefix("[^") {
                    let labelStart = source.index(index, offsetBy: 2)
                    let label = String(source[labelStart..<closeBracket])
                        .trimmingCharacters(in: .whitespaces)
                    if !label.isEmpty {
                        let display = superscriptDigits(label) ?? escapedMarkdownLabel(label)
                        let target = escapedLinkTarget("fn-\(label)")
                        result += "[\(display)](#\(target))"
                        index = source.index(after: closeBracket)
                        continue
                    }
                }
            }

            result.append(source[index])
            index = source.index(after: index)
        }

        return result
    }

    private static func existingMarkdownLinkEnd(
        in source: String,
        closeBracket: String.Index
    ) -> String.Index? {
        let parenStart = source.index(after: closeBracket)
        guard parenStart < source.endIndex,
              source[parenStart] == "(" else {
            return nil
        }

        guard let closeParen = source[parenStart...].firstIndex(of: ")") else {
            return nil
        }

        return source.index(after: closeParen)
    }

    private static func superscriptDigits(_ source: String) -> String? {
        guard !source.isEmpty else { return nil }

        let digits: [Character: Character] = [
            "0": "⁰",
            "1": "¹",
            "2": "²",
            "3": "³",
            "4": "⁴",
            "5": "⁵",
            "6": "⁶",
            "7": "⁷",
            "8": "⁸",
            "9": "⁹",
        ]

        var result = ""
        for character in source {
            guard let digit = digits[character] else { return nil }
            result.append(digit)
        }
        return result
    }

    private static func isBareURLStart(in source: String, at index: String.Index) -> Bool {
        guard source[index...].hasPrefix("https://") || source[index...].hasPrefix("http://") else {
            return false
        }

        guard index > source.startIndex else { return true }
        let previous = source[source.index(before: index)]
        return previous.isWhitespace || "([<{".contains(previous)
    }

    private static func isExistingMarkdownLinkTarget(in source: String, at index: String.Index) -> Bool {
        guard index > source.startIndex else { return false }
        let previousIndex = source.index(before: index)
        guard source[previousIndex] == "(",
              previousIndex > source.startIndex else {
            return false
        }

        return source[source.index(before: previousIndex)] == "]"
    }

    private static func consumeBareURL(
        in source: String,
        from startIndex: String.Index
    ) -> (url: String, trailingPunctuation: String, endIndex: String.Index) {
        var index = startIndex
        while index < source.endIndex {
            let character = source[index]
            if character.isWhitespace || "<>\"'`".contains(character) {
                break
            }
            index = source.index(after: index)
        }

        var url = String(source[startIndex..<index])
        var trailing = ""
        while let last = url.last, ".,;:!?)]".contains(last) {
            trailing.insert(last, at: trailing.startIndex)
            url.removeLast()
        }

        return (url, trailing, index)
    }

    private static func closingMathDelimiter(in source: String, after startIndex: String.Index) -> String.Index? {
        guard startIndex < source.endIndex,
              !source[startIndex].isWhitespace,
              source[startIndex] != "$" else {
            return nil
        }

        var index = startIndex
        var previous = source[source.index(before: index)]
        while index < source.endIndex {
            let character = source[index]
            if character == "\n" {
                return nil
            }
            if character == "\\" {
                let nextIndex = source.index(after: index)
                guard nextIndex < source.endIndex else { return nil }
                previous = source[nextIndex]
                index = source.index(after: nextIndex)
                continue
            }
            if character == "$", previous != "$", !previous.isWhitespace {
                return index
            }
            previous = character
            index = source.index(after: index)
        }

        return nil
    }

    private static func escapedInlineCode(_ source: String) -> String {
        source.replacingOccurrences(of: "`", with: "\\`")
    }

    private static func escapedMarkdownLabel(_ label: String) -> String {
        label
            .replacingOccurrences(of: "\\", with: "\\\\")
            .replacingOccurrences(of: "[", with: "\\[")
            .replacingOccurrences(of: "]", with: "\\]")
    }

    private static func escapedLinkTarget(_ target: String) -> String {
        target.replacingOccurrences(of: " ", with: "%20")
    }
}

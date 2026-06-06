import Foundation

public enum MarkdownBlock: Equatable {
    case heading(level: Int, text: String)
    case paragraph(String)
    case unorderedListItem(String)
    case orderedListItem(number: Int, text: String)
    case taskListItem(isComplete: Bool, text: String)
    case blockquote(String)
    case callout(kind: String, title: String, text: String)
    case footnoteDefinition(label: String, text: String)
    case horizontalRule
    case codeBlock(language: String?, code: String)
    case mermaidDiagram(MermaidDiagram)
    case mathBlock(String)
    case table(headers: [String], rows: [[String]])
    case image(alt: String, source: String)
}

public struct MarkdownOutlineItem: Equatable, Identifiable {
    public let id: Int
    public let level: Int
    public let title: String
    public let blockIndex: Int

    public init(level: Int, title: String, blockIndex: Int) {
        self.id = blockIndex
        self.level = level
        self.title = title
        self.blockIndex = blockIndex
    }
}

public enum MarkdownParser {
    public static func parse(_ source: String) -> [MarkdownBlock] {
        let lines = source.components(separatedBy: .newlines)
        var blocks: [MarkdownBlock] = []
        var paragraphLines: [String] = []
        var codeLanguage: String?
        var codeLines: [String] = []
        var insideCodeBlock = false
        var codeFenceMarker: Character = "`"
        var codeFenceLength = 3
        var insideMathBlock = false
        var mathLines: [String] = []
        var index = startingContentIndex(in: lines)

        func appendCodeBlock(language: String?, code: String) {
            if language?.lowercased() == "mermaid",
               let diagram = MermaidDiagramParser.parse(code) {
                blocks.append(.mermaidDiagram(diagram))
            } else {
                blocks.append(.codeBlock(language: language, code: code))
            }
        }

        func flushParagraph() {
            guard !paragraphLines.isEmpty else { return }
            blocks.append(.paragraph(paragraphLines.joined(separator: "\n")))
            paragraphLines.removeAll()
        }

        while index < lines.count {
            let rawLine = lines[index]
            let trimmed = rawLine.trimmingCharacters(in: .whitespaces)

            if insideMathBlock {
                if trimmed == "$$" {
                    blocks.append(.mathBlock(mathLines.joined(separator: "\n")))
                    mathLines.removeAll()
                    insideMathBlock = false
                } else {
                    mathLines.append(rawLine)
                }
                index += 1
                continue
            }

            if insideCodeBlock {
                if isClosingCodeFence(
                    trimmed,
                    marker: codeFenceMarker,
                    minimumLength: codeFenceLength
                ) {
                    appendCodeBlock(language: codeLanguage, code: codeLines.joined(separator: "\n"))
                    codeLanguage = nil
                    codeLines.removeAll()
                    insideCodeBlock = false
                } else {
                    codeLines.append(rawLine)
                }
                index += 1
                continue
            }

            if trimmed == "$$" {
                flushParagraph()
                insideMathBlock = true
                mathLines.removeAll()
                index += 1
                continue
            }

            if let fence = parseCodeFence(trimmed) {
                flushParagraph()
                codeLanguage = fence.language
                codeFenceMarker = fence.marker
                codeFenceLength = fence.length
                insideCodeBlock = true
                index += 1
                continue
            }

            if trimmed.isEmpty {
                flushParagraph()
                index += 1
                continue
            }

            if let footnote = parseFootnoteDefinition(lines: lines, startIndex: index) {
                flushParagraph()
                blocks.append(footnote.block)
                index = footnote.nextIndex
                continue
            }

            if let indentedCode = parseIndentedCodeBlock(lines: lines, startIndex: index) {
                flushParagraph()
                blocks.append(.codeBlock(language: nil, code: indentedCode.code))
                index = indentedCode.nextIndex
                continue
            }

            if let setextLevel = parseSetextHeadingUnderline(trimmed), !paragraphLines.isEmpty {
                let headingText = paragraphLines.joined(separator: " ")
                paragraphLines.removeAll()
                blocks.append(.heading(level: setextLevel, text: headingText))
                index += 1
                continue
            }

            if trimmed == "---" || trimmed == "***" || trimmed == "___" {
                flushParagraph()
                blocks.append(.horizontalRule)
                index += 1
                continue
            }

            if let heading = parseHeading(trimmed) {
                flushParagraph()
                blocks.append(heading)
                index += 1
                continue
            }

            if let table = parseTable(lines: lines, startIndex: index) {
                flushParagraph()
                blocks.append(table.block)
                index = table.nextIndex
                continue
            }

            if let image = parseImage(trimmed) {
                flushParagraph()
                blocks.append(image)
                index += 1
                continue
            }

            if let task = parseTaskListItem(trimmed) {
                flushParagraph()
                blocks.append(task)
                index += 1
                continue
            }

            if trimmed.hasPrefix("- ") || trimmed.hasPrefix("* ") || trimmed.hasPrefix("+ ") {
                flushParagraph()
                blocks.append(.unorderedListItem(String(trimmed.dropFirst(2))))
                index += 1
                continue
            }

            if let ordered = parseOrderedListItem(trimmed) {
                flushParagraph()
                blocks.append(ordered)
                index += 1
                continue
            }

            if trimmed.hasPrefix(">") {
                flushParagraph()
                let quote = parseBlockquote(lines: lines, startIndex: index)
                if let callout = parseCallout(quote.text) {
                    blocks.append(callout)
                } else {
                    blocks.append(.blockquote(quote.text))
                }
                index = quote.nextIndex
                continue
            }

            paragraphLines.append(trimmed)
            index += 1
        }

        if insideCodeBlock {
            appendCodeBlock(language: codeLanguage, code: codeLines.joined(separator: "\n"))
        }
        if insideMathBlock {
            blocks.append(.mathBlock(mathLines.joined(separator: "\n")))
        }
        flushParagraph()

        return blocks
    }

    public static func outline(_ blocks: [MarkdownBlock]) -> [MarkdownOutlineItem] {
        blocks.enumerated().compactMap { index, block in
            guard case .heading(let level, let text) = block else { return nil }
            return MarkdownOutlineItem(level: level, title: text, blockIndex: index)
        }
    }

    private static func parseHeading(_ line: String) -> MarkdownBlock? {
        var level = 0
        for character in line {
            if character == "#" {
                level += 1
            } else {
                break
            }
        }

        guard (1...6).contains(level) else { return nil }
        let textStart = line.index(line.startIndex, offsetBy: level)
        guard textStart < line.endIndex, line[textStart] == " " else { return nil }

        let text = headingTextWithoutClosingMarker(
            String(line[line.index(after: textStart)...])
        )
        guard !text.isEmpty else { return nil }

        return .heading(level: level, text: text)
    }

    private static func headingTextWithoutClosingMarker(_ source: String) -> String {
        let trimmed = source.trimmingCharacters(in: .whitespaces)
        guard let lastNonHashIndex = trimmed.lastIndex(where: { $0 != "#" }) else {
            return ""
        }

        let markerStart = trimmed.index(after: lastNonHashIndex)
        guard markerStart < trimmed.endIndex,
              trimmed[lastNonHashIndex].isWhitespace else {
            return trimmed
        }

        return String(trimmed[..<lastNonHashIndex]).trimmingCharacters(in: .whitespaces)
    }

    private static func parseSetextHeadingUnderline(_ line: String) -> Int? {
        let normalized = line.replacingOccurrences(of: " ", with: "")
        guard !normalized.isEmpty else { return nil }

        if normalized.allSatisfy({ $0 == "=" }) {
            return 1
        }
        if normalized.allSatisfy({ $0 == "-" }) {
            return 2
        }
        return nil
    }

    private static func startingContentIndex(in lines: [String]) -> Int {
        guard let first = lines.first,
              first.trimmingCharacters(in: .whitespaces) == "---" else {
            return 0
        }

        for index in lines.indices.dropFirst() {
            if lines[index].trimmingCharacters(in: .whitespaces) == "---" {
                return index + 1
            }
        }

        return 0
    }

    private static func parseFootnoteDefinition(
        lines: [String],
        startIndex: Int
    ) -> (block: MarkdownBlock, nextIndex: Int)? {
        let line = lines[startIndex].trimmingCharacters(in: .whitespaces)
        guard line.hasPrefix("[^"),
              let closeLabel = line.firstIndex(of: "]"),
              line.indices.contains(line.index(after: closeLabel)),
              line[line.index(after: closeLabel)] == ":" else {
            return nil
        }

        let labelStart = line.index(line.startIndex, offsetBy: 2)
        let label = String(line[labelStart..<closeLabel]).trimmingCharacters(in: .whitespaces)
        guard !label.isEmpty else { return nil }

        let textStart = line.index(closeLabel, offsetBy: 2)
        var textLines = [
            String(line[textStart...]).trimmingCharacters(in: .whitespaces)
        ]

        var index = startIndex + 1
        while index < lines.count, let continuation = footnoteContinuationLine(lines[index]) {
            textLines.append(continuation)
            index += 1
        }

        return (
            .footnoteDefinition(label: label, text: textLines.joined(separator: "\n")),
            index
        )
    }

    private static func footnoteContinuationLine(_ line: String) -> String? {
        if line.hasPrefix("    ") {
            return String(line.dropFirst(4)).trimmingCharacters(in: .whitespaces)
        }
        if line.hasPrefix("\t") {
            return String(line.dropFirst()).trimmingCharacters(in: .whitespaces)
        }
        return nil
    }

    private static func parseOrderedListItem(_ line: String) -> MarkdownBlock? {
        guard let delimiterIndex = line.firstIndex(where: { $0 == "." || $0 == ")" }) else {
            return nil
        }
        let numberText = String(line[..<delimiterIndex])
        guard let number = Int(numberText), number > 0 else { return nil }

        let afterDelimiter = line.index(after: delimiterIndex)
        guard afterDelimiter < line.endIndex, line[afterDelimiter] == " " else { return nil }

        let textStart = line.index(after: afterDelimiter)
        guard textStart < line.endIndex else { return nil }

        return .orderedListItem(number: number, text: String(line[textStart...]))
    }

    private static func parseIndentedCodeBlock(
        lines: [String],
        startIndex: Int
    ) -> (code: String, nextIndex: Int)? {
        guard let firstLine = unindentedCodeLine(lines[startIndex]) else { return nil }

        var codeLines = [firstLine]
        var index = startIndex + 1
        while index < lines.count, let codeLine = unindentedCodeLine(lines[index]) {
            codeLines.append(codeLine)
            index += 1
        }

        return (codeLines.joined(separator: "\n"), index)
    }

    private static func unindentedCodeLine(_ line: String) -> String? {
        if line.hasPrefix("    ") {
            return String(line.dropFirst(4))
        }
        if line.hasPrefix("\t") {
            return String(line.dropFirst())
        }
        return nil
    }

    private static func parseCodeFence(_ line: String) -> (
        marker: Character,
        length: Int,
        language: String?
    )? {
        guard let marker = line.first, marker == "`" || marker == "~" else { return nil }
        let length = line.prefix(while: { $0 == marker }).count
        guard length >= 3 else { return nil }

        let info = String(line.dropFirst(length))
        return (marker, length, normalizedCodeLanguage(info))
    }

    private static func isClosingCodeFence(
        _ line: String,
        marker: Character,
        minimumLength: Int
    ) -> Bool {
        guard line.first == marker else { return false }

        let length = line.prefix(while: { $0 == marker }).count
        guard length >= minimumLength else { return false }

        let suffix = line.dropFirst(length)
        return suffix.trimmingCharacters(in: .whitespaces).isEmpty
    }

    private static func normalizedCodeLanguage(_ info: String) -> String? {
        let normalized = info
            .trimmingCharacters(in: .whitespaces)
            .lowercased()
            .replacingOccurrences(of: "language-", with: "")
            .split(whereSeparator: { $0.isWhitespace })
            .first

        guard let normalized, !normalized.isEmpty else { return nil }
        return String(normalized)
    }

    private static func parseBlockquote(lines: [String], startIndex: Int) -> (text: String, nextIndex: Int) {
        var quoteLines: [String] = []
        var index = startIndex

        while index < lines.count {
            let trimmed = lines[index].trimmingCharacters(in: .whitespaces)
            guard trimmed.hasPrefix(">") else { break }

            let quoteLine = String(trimmed.dropFirst()).trimmingCharacters(in: .whitespaces)
            quoteLines.append(quoteLine)
            index += 1
        }

        return (quoteLines.joined(separator: "\n"), index)
    }

    private static func parseCallout(_ quoteText: String) -> MarkdownBlock? {
        var lines = quoteText.components(separatedBy: "\n")
        guard let markerLine = lines.first?.trimmingCharacters(in: .whitespaces),
              markerLine.hasPrefix("[!"),
              let closeBracket = markerLine.firstIndex(of: "]") else {
            return nil
        }

        let kindStart = markerLine.index(markerLine.startIndex, offsetBy: 2)
        let rawKind = String(markerLine[kindStart..<closeBracket]).lowercased()
        guard ["tip", "note", "important", "warning", "caution"].contains(rawKind) else {
            return nil
        }

        let titleStart = markerLine.index(after: closeBracket)
        let explicitTitle = calloutTitleText(
            String(markerLine[titleStart...]).trimmingCharacters(in: .whitespaces)
        )
        let title = explicitTitle.isEmpty ? rawKind.capitalized : explicitTitle
        lines.removeFirst()
        let text = lines.joined(separator: "\n").trimmingCharacters(in: .newlines)

        return .callout(kind: rawKind, title: title, text: text)
    }

    private static func calloutTitleText(_ source: String) -> String {
        guard let first = source.first, first == "-" || first == "+" else {
            return source
        }

        return String(source.dropFirst()).trimmingCharacters(in: .whitespaces)
    }

    private static func parseTaskListItem(_ line: String) -> MarkdownBlock? {
        let prefixes: [(String, Bool)] = [
            ("- [x] ", true),
            ("- [X] ", true),
            ("* [x] ", true),
            ("* [X] ", true),
            ("+ [x] ", true),
            ("+ [X] ", true),
            ("- [ ] ", false),
            ("* [ ] ", false),
            ("+ [ ] ", false),
        ]

        for (prefix, isComplete) in prefixes where line.hasPrefix(prefix) {
            return .taskListItem(isComplete: isComplete, text: String(line.dropFirst(prefix.count)))
        }

        return nil
    }

    private static func parseImage(_ line: String) -> MarkdownBlock? {
        guard line.hasPrefix("!["),
              let closeAlt = line.firstIndex(of: "]"),
              line.indices.contains(line.index(after: closeAlt)),
              line[line.index(after: closeAlt)] == "(",
              line.hasSuffix(")") else {
            return nil
        }

        let altStart = line.index(line.startIndex, offsetBy: 2)
        let alt = String(line[altStart..<closeAlt])
        let sourceStart = line.index(closeAlt, offsetBy: 2)
        let sourceEnd = line.index(before: line.endIndex)
        let destination = String(line[sourceStart..<sourceEnd]).trimmingCharacters(in: .whitespaces)
        guard let source = parseImageDestination(destination) else { return nil }
        guard !source.isEmpty else { return nil }

        return .image(alt: alt, source: source)
    }

    private static func parseImageDestination(_ destination: String) -> String? {
        guard !destination.isEmpty else { return nil }

        if destination.hasPrefix("<") {
            guard let closeBracket = destination.firstIndex(of: ">") else { return nil }
            let sourceStart = destination.index(after: destination.startIndex)
            let source = String(destination[sourceStart..<closeBracket])
                .trimmingCharacters(in: .whitespaces)
            return source.isEmpty ? nil : source
        }

        guard let titleStart = destination.firstIndex(where: { $0.isWhitespace }) else {
            return destination
        }

        let source = String(destination[..<titleStart]).trimmingCharacters(in: .whitespaces)
        let titleCandidate = destination[titleStart...].trimmingCharacters(in: .whitespaces)
        guard let firstTitleCharacter = titleCandidate.first,
              firstTitleCharacter == "\"" || firstTitleCharacter == "'" || firstTitleCharacter == "(" else {
            return destination
        }

        return source.isEmpty ? nil : source
    }

    private static func parseTable(
        lines: [String],
        startIndex: Int
    ) -> (block: MarkdownBlock, nextIndex: Int)? {
        guard startIndex + 1 < lines.count else { return nil }

        let header = parseTableCells(lines[startIndex])
        let separator = parseTableCells(lines[startIndex + 1])
        guard !header.isEmpty, separator.count == header.count else { return nil }
        guard separator.allSatisfy({ cell in
            let normalized = cell.replacingOccurrences(of: ":", with: "")
            return !normalized.isEmpty && normalized.allSatisfy { $0 == "-" }
        }) else {
            return nil
        }

        var rows: [[String]] = []
        var index = startIndex + 2
        while index < lines.count {
            let cells = parseTableCells(lines[index])
            guard cells.count == header.count else { break }
            rows.append(cells)
            index += 1
        }

        return (.table(headers: header, rows: rows), index)
    }

    private static func parseTableCells(_ line: String) -> [String] {
        let trimmed = line.trimmingCharacters(in: .whitespaces)
        guard trimmed.contains("|") else { return [] }

        var cells: [String] = []
        var current = ""
        var isEscaped = false

        for character in trimmed {
            if isEscaped {
                if character == "|" {
                    current.append(character)
                } else {
                    current.append("\\")
                    current.append(character)
                }
                isEscaped = false
                continue
            }

            if character == "\\" {
                isEscaped = true
                continue
            }

            if character == "|" {
                cells.append(current.trimmingCharacters(in: .whitespaces))
                current = ""
                continue
            }

            current.append(character)
        }

        if isEscaped {
            current.append("\\")
        }

        cells.append(current.trimmingCharacters(in: .whitespaces))

        if cells.first == "" {
            cells.removeFirst()
        }
        if cells.last == "" {
            cells.removeLast()
        }

        return cells
    }
}

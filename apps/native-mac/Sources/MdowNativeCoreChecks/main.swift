import Foundation
import MdowNativeCore

func check(_ condition: @autoclosure () -> Bool, _ message: String) {
    if !condition() {
        fputs("FAIL: \(message)\n", stderr)
        exit(1)
    }
}

func checkMarkdownFileSupport() throws {
    check(MarkdownFile.isSupported(URL(fileURLWithPath: "/tmp/readme.md")), "supports .md")
    check(
        MarkdownFile.isSupported(URL(fileURLWithPath: "/tmp/readme.MARKDOWN")),
        "supports uppercase .MARKDOWN"
    )
    check(MarkdownFile.isSupported(URL(fileURLWithPath: "/tmp/readme.mdx")), "supports .mdx")
    check(!MarkdownFile.isSupported(URL(fileURLWithPath: "/tmp/readme.txt")), "rejects .txt")
    check(
        !MarkdownFile.isSupported(URL(fileURLWithPath: "/tmp/markdown.json")),
        "rejects markdown.json"
    )

    let url = FileManager.default.temporaryDirectory
        .appendingPathComponent(UUID().uuidString)
        .appendingPathExtension("md")
    try "# Hello\n\nNative reader".write(to: url, atomically: true, encoding: .utf8)
    defer { try? FileManager.default.removeItem(at: url) }

    let document = try MarkdownFile.load(url)
    check(document.url == url, "loads document URL")
    check(document.title == url.lastPathComponent, "uses file name as title")
    check(document.content == "# Hello\n\nNative reader", "loads UTF-8 content")

    let titledURL = FileManager.default.temporaryDirectory
        .appendingPathComponent(UUID().uuidString)
        .appendingPathExtension("md")
    try """
    ---
    title: Native Project Brief
    ---

    # Ignored Heading
    """.write(to: titledURL, atomically: true, encoding: .utf8)
    defer { try? FileManager.default.removeItem(at: titledURL) }

    let titledDocument = try MarkdownFile.load(titledURL)
    check(titledDocument.title == "Native Project Brief", "uses YAML frontmatter title")
}

func checkMarkdownOpenURLRouting() throws {
    let root = FileManager.default.temporaryDirectory
        .appendingPathComponent("MdowNativeOpenRouting-\(UUID().uuidString)")
    let fileURL = root.appendingPathComponent("Note.md")
    let unsupportedURL = root.appendingPathComponent("Note.txt")
    try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
    try "# Note".write(to: fileURL, atomically: true, encoding: .utf8)
    try "Note".write(to: unsupportedURL, atomically: true, encoding: .utf8)
    defer { try? FileManager.default.removeItem(at: root) }

    check(
        MarkdownOpenURLRouting.target(for: root) == .folder(root),
        "routes directory URLs to folders"
    )
    check(
        MarkdownOpenURLRouting.target(for: fileURL) == .markdownFile(fileURL),
        "routes markdown URLs to files"
    )
    check(
        MarkdownOpenURLRouting.target(for: unsupportedURL) == .ignored,
        "ignores unsupported file URLs"
    )
}

func checkMarkdownParser() {
    let source = """
    # Title

    Intro paragraph
    - Item one
    1. Step one
    > Quoted
    ---
    ```swift
    let value = 1
    ```
    """

    let blocks = MarkdownParser.parse(source)
    check(
        blocks == [
            .heading(level: 1, text: "Title"),
            .paragraph("Intro paragraph"),
            .unorderedListItem("Item one"),
            .orderedListItem(number: 1, text: "Step one"),
            .blockquote("Quoted"),
            .horizontalRule,
            .codeBlock(language: "swift", code: "let value = 1"),
        ],
        "parses core Markdown blocks"
    )

    check(
        MarkdownParser.parse("This wraps\nonto another line\n\nNext") == [
            .paragraph("This wraps\nonto another line"),
            .paragraph("Next"),
        ],
        "preserves soft line breaks inside paragraphs"
    )

    check(
        MarkdownParser.parse("# Closed heading ###\n## Second closed ##") == [
            .heading(level: 1, text: "Closed heading"),
            .heading(level: 2, text: "Second closed"),
        ],
        "strips closing ATX heading markers"
    )

    check(
        MarkdownParser.parse("# Literal#") == [
            .heading(level: 1, text: "Literal#"),
        ],
        "keeps non-separated trailing hash in heading text"
    )

    check(
        MarkdownParser.parse("```language-swift linenos\nlet value = 1\n```") == [
            .codeBlock(language: "swift", code: "let value = 1"),
        ],
        "normalizes fenced code language info"
    )

    check(
        MarkdownParser.parse("~~~python\nprint('hello')\n~~~") == [
            .codeBlock(language: "python", code: "print('hello')"),
        ],
        "parses tilde fenced code blocks"
    )

    check(
        MarkdownParser.parse("    let value = 1\n    print(value)") == [
            .codeBlock(language: nil, code: "let value = 1\nprint(value)"),
        ],
        "parses indented code blocks"
    )

    check(
        MarkdownParser.parse("Primary Title\n===\n\nSecondary Title\n---") == [
            .heading(level: 1, text: "Primary Title"),
            .heading(level: 2, text: "Secondary Title"),
        ],
        "parses Setext headings"
    )

    check(
        MarkdownParser.parse("$$\nx^2 + y^2 = z^2\n$$") == [
            .mathBlock("x^2 + y^2 = z^2"),
        ],
        "parses display math blocks"
    )

    check(
        MarkdownParser.parse("---") == [
            .horizontalRule,
        ],
        "keeps standalone dash rule as horizontal rule"
    )

    check(
        MarkdownParser.parse("> First line\n> Second line\n>\n> Final line") == [
            .blockquote("First line\nSecond line\n\nFinal line"),
        ],
        "groups consecutive blockquote lines"
    )

    check(
        MarkdownParser.parse("> [!WARNING] Heads up\n> Check credentials before deploy.") == [
            .callout(kind: "warning", title: "Heads up", text: "Check credentials before deploy."),
        ],
        "parses GitHub-style alert callouts"
    )

    check(
        MarkdownParser.parse("> [!NOTE]- Collapsible note\n> Hidden details stay readable.") == [
            .callout(kind: "note", title: "Collapsible note", text: "Hidden details stay readable."),
        ],
        "strips collapsed callout marker from title"
    )

    check(
        MarkdownParser.parse("> [!TIP]+\n> Expanded tip uses default title.") == [
            .callout(kind: "tip", title: "Tip", text: "Expanded tip uses default title."),
        ],
        "strips expanded callout marker before default title fallback"
    )

    check(
        MarkdownParser.parse("[^1]: Footnotes keep references readable.") == [
            .footnoteDefinition(label: "1", text: "Footnotes keep references readable."),
        ],
        "parses footnote definitions"
    )

    check(
        MarkdownParser.parse("[^note]: First line\n    Continued detail") == [
            .footnoteDefinition(label: "note", text: "First line\nContinued detail"),
        ],
        "parses continued footnote definitions"
    )

    check(
        MarkdownParser.parse(
            """
            ---
            tags: [project, notes]
            created: 2026-06-05
            ---

            # Project Notes

            Body
            """
        ) == [
            .heading(level: 1, text: "Project Notes"),
            .paragraph("Body"),
        ],
        "skips leading YAML frontmatter"
    )

    check(
        MarkdownParser.parse("- [x] Done\n- [ ] Todo\n+ [x] Plus done\n+ [ ] Plus todo") == [
            .taskListItem(isComplete: true, text: "Done"),
            .taskListItem(isComplete: false, text: "Todo"),
            .taskListItem(isComplete: true, text: "Plus done"),
            .taskListItem(isComplete: false, text: "Plus todo"),
        ],
        "parses task list items"
    )

    check(
        MarkdownParser.parse("+ Plus item\n2) Parenthesized step") == [
            .unorderedListItem("Plus item"),
            .orderedListItem(number: 2, text: "Parenthesized step"),
        ],
        "parses alternate CommonMark list markers"
    )

    check(
        MarkdownParser.parse("| Name | Value |\n| --- | --- |\n| One | 1 |") == [
            .table(headers: ["Name", "Value"], rows: [["One", "1"]]),
        ],
        "parses simple pipe tables"
    )

    check(
        MarkdownParser.parse("| Name | Value |\n| --- | --- |\n| Pipe | A \\| B |") == [
            .table(headers: ["Name", "Value"], rows: [["Pipe", "A | B"]]),
        ],
        "parses escaped pipes inside table cells"
    )

    check(
        MarkdownParser.parse("![Diagram](assets/diagram.png)") == [
            .image(alt: "Diagram", source: "assets/diagram.png"),
        ],
        "parses image blocks"
    )

    check(
        MarkdownParser.parse("![Diagram](assets/diagram.png \"Architecture diagram\")") == [
            .image(alt: "Diagram", source: "assets/diagram.png"),
        ],
        "parses image blocks with optional titles"
    )

    check(
        MarkdownParser.parse("![Diagram](<assets/my diagram.png> 'Architecture diagram')") == [
            .image(alt: "Diagram", source: "assets/my diagram.png"),
        ],
        "parses angle-bracketed image destinations"
    )

    let mermaidSource = """
    ```mermaid
    flowchart LR
      md[Markdown] --> parse[md4x]
      parse --> html[HTML]
      html --> view[MarkdownView]
    ```
    """
    check(
        MarkdownParser.parse(mermaidSource) == [
            .mermaidDiagram(
                MermaidDiagram(
                    direction: .leftToRight,
                    nodes: [
                        .init(id: "md", label: "Markdown"),
                        .init(id: "parse", label: "md4x"),
                        .init(id: "html", label: "HTML"),
                        .init(id: "view", label: "MarkdownView"),
                    ],
                    edges: [
                        .init(from: "md", to: "parse"),
                        .init(from: "parse", to: "html"),
                        .init(from: "html", to: "view"),
                    ],
                    source: "flowchart LR\n  md[Markdown] --> parse[md4x]\n  parse --> html[HTML]\n  html --> view[MarkdownView]"
                )
            ),
        ],
        "parses simple Mermaid flowcharts as native diagrams"
    )

    let outline = MarkdownParser.outline(
        MarkdownParser.parse("# Title\n\n## Section\n\n### Detail")
    )
    check(
        outline == [
            .init(level: 1, title: "Title", blockIndex: 0),
            .init(level: 2, title: "Section", blockIndex: 1),
            .init(level: 3, title: "Detail", blockIndex: 2),
        ],
        "extracts heading outline"
    )
}

func checkMarkdownInlineNormalization() {
    check(
        MarkdownInline.normalizedMarkdown("See [[Daily Notes]]") == "See [Daily Notes](Daily%20Notes)",
        "normalizes simple wiki links"
    )

    check(
        MarkdownInline.normalizedMarkdown("See [[../Daily/|Daily Notes]]") == "See [Daily Notes](../Daily/)",
        "normalizes aliased wiki links"
    )

    check(
        MarkdownInline.normalizedMarkdown("Keep **bold** and `code`") == "Keep **bold** and `code`",
        "preserves ordinary inline Markdown"
    )

    check(
        MarkdownInline.normalizedMarkdown("Visit https://example.com/docs.") ==
            "Visit [https://example.com/docs](https://example.com/docs).",
        "normalizes bare HTTPS URLs to links"
    )

    check(
        MarkdownInline.normalizedMarkdown("Keep [docs](https://example.com) and `https://code.test`") ==
            "Keep [docs](https://example.com) and `https://code.test`",
        "does not normalize URLs inside existing links or inline code"
    )

    check(
        MarkdownInline.normalizedMarkdown("Footnote reference[^12].") ==
            "Footnote reference[¹²](#fn-12).",
        "normalizes numeric footnote references to raised local links"
    )

    check(
        MarkdownInline.normalizedMarkdown("Keep `[^1]` and [^note].") ==
            "Keep `[^1]` and [note](#fn-note).",
        "normalizes nonnumeric footnote references outside inline code"
    )

    check(
        MarkdownInline.normalizedMarkdown("Inline $x^2$ math") == "Inline `x^2` math",
        "normalizes inline math to native inline code rendering"
    )

    check(
        MarkdownInline.normalizedMarkdown("Cost is $5 today") == "Cost is $5 today",
        "leaves unpaired dollar text alone"
    )

    check(
        MarkdownInline.normalizedMarkdown("Escaped \\$x\\$ stays literal") == "Escaped \\$x\\$ stays literal",
        "leaves escaped dollar delimiters alone"
    )
}

func checkMarkdownLinkResolver() {
    let documentURL = URL(fileURLWithPath: "/Users/zain/notes/today.md")

    check(
        MarkdownLinkResolver.resolve(
            href: "https://example.com/docs",
            documentURL: documentURL
        ) == .external(URL(string: "https://example.com/docs")!),
        "resolves external markdown links"
    )

    check(
        MarkdownLinkResolver.resolve(href: "#details", documentURL: documentURL) == .anchor("details"),
        "resolves same-document anchor links"
    )

    check(
        MarkdownLinkResolver.resolve(href: "Project Plan.md", documentURL: documentURL)
            == .markdownFile(URL(fileURLWithPath: "/Users/zain/notes/Project Plan.md"), anchor: nil),
        "resolves relative markdown links beside the current file"
    )

    check(
        MarkdownLinkResolver.resolve(href: "../archive/Spec.mdx#api", documentURL: documentURL)
            == .markdownFile(URL(fileURLWithPath: "/Users/zain/archive/Spec.mdx"), anchor: "api"),
        "resolves relative markdown links with anchors"
    )

    check(
        MarkdownLinkResolver.resolve(href: "assets/diagram.png", documentURL: documentURL) == .ignored,
        "ignores non-markdown relative assets"
    )

    let anchorBlocks = MarkdownParser.parse(
        """
        # Project Notes

        Body with footnote[^note].

        [^note]: Footnote details.
        """
    )
    check(
        MarkdownAnchorResolver.blockIndex(for: "project-notes", in: anchorBlocks) == 0,
        "resolves heading anchors to heading block indices"
    )
    check(
        MarkdownAnchorResolver.blockIndex(for: "fn-note", in: anchorBlocks) == 2,
        "resolves footnote anchors to footnote definition block indices"
    )
}

func checkMarkdownSearch() {
    check(
        MarkdownSearch.matches(in: "Goal goal GOAL", query: "goal") == [
            .init(offset: 0, length: 4),
            .init(offset: 5, length: 4),
            .init(offset: 10, length: 4),
        ],
        "finds case-insensitive search matches"
    )

    check(
        MarkdownSearch.matches(in: "Résumé resume", query: "resume") == [
            .init(offset: 0, length: 6),
            .init(offset: 7, length: 6),
        ],
        "finds diacritic-insensitive search matches"
    )

    check(
        MarkdownSearch.matches(in: "Anything", query: "") == [],
        "empty search has no matches"
    )

    check(
        MarkdownSearchNavigation.targets(in: ["Goal goal", "No match", "GOAL"], query: "goal") == [
            .init(blockIndex: 0, matchIndex: 0),
            .init(blockIndex: 0, matchIndex: 1),
            .init(blockIndex: 2, matchIndex: 0),
        ],
        "search navigation exposes one target per navigable match"
    )

    check(
        MarkdownSearchNavigation.counterText(query: "goal", targetCount: 3, currentIndex: 0) == "1 of 3",
        "search counter shows current match and total"
    )

    check(
        MarkdownSearchNavigation.counterText(query: "missing", targetCount: 0, currentIndex: nil) == "No results",
        "search counter reports empty results"
    )

    check(
        MarkdownSearchNavigation.counterText(query: "", targetCount: 3, currentIndex: 0) == "",
        "search counter stays blank for an empty query"
    )

    check(
        MarkdownSearchNavigation.advancedPosition(
            currentPosition: -1,
            targetCount: 3,
            offset: 1,
            queryChanged: true
        ) == 1,
        "first next action advances from visible first match to second match"
    )

    check(
        MarkdownSearchNavigation.advancedPosition(
            currentPosition: -1,
            targetCount: 3,
            offset: -1,
            queryChanged: true
        ) == 2,
        "first previous action wraps from visible first match to last match"
    )

    check(
        MarkdownSearchNavigation.advancedPosition(
            currentPosition: 2,
            targetCount: 3,
            offset: 1,
            queryChanged: false
        ) == 0,
        "search navigation wraps forward"
    )
}

func checkMarkdownFileErrors() {
    let missingURL = URL(fileURLWithPath: "/tmp/definitely-missing-mdow-file.md")
    let missingError = MarkdownFileErrorModel(
        url: missingURL,
        error: MarkdownFileError.notFound(missingURL)
    )
    check(missingError.kind == .notFound, "classifies missing markdown file errors")
    check(missingError.title == "File not found", "titles missing file errors")
    check(missingError.canRevealInFinder, "allows reveal for missing file parent")

    let readError = MarkdownFileErrorModel(
        url: missingURL,
        error: MarkdownFileError.unreadable(missingURL)
    )
    check(readError.kind == .readError, "classifies unreadable markdown file errors")
    check(readError.title == "Couldn't read file", "titles read errors")
}

func checkReaderChromePreferences() {
    check(AppTheme.allCases.map(\.title) == ["System", "Light", "Dark"], "lists app theme options")
    check(AppTheme(rawValue: "light") == .light, "loads light app theme raw value")
    check(AppTheme(rawValue: "dark") == .dark, "loads dark app theme raw value")
    check(
        ContentFontPreset.allCases.map(\.title) == ["Inter", "Charter", "System", "Georgia"],
        "lists content font presets"
    )
    check(
        CodeFontPreset.allCases.map(\.title) == ["Geist", "System", "SF Mono", "JetBrains"],
        "lists code font presets"
    )
    check(ContentFontPreset(rawValue: "system-sans") == .systemSans, "loads system content font")
    check(CodeFontPreset(rawValue: "jetbrains-mono") == .jetbrainsMono, "loads JetBrains code font")
    check(ReaderChromePreferences().appTheme == .system, "reader chrome starts with system app theme")
    check(ReaderChromePreferences().contentFont == .inter, "reader chrome starts with Inter content font")
    check(ReaderChromePreferences().codeFont == .geistMono, "reader chrome starts with Geist code font")
    check(ReaderChromePreferences().sidebarOpen, "reader chrome starts with sidebar open")
    check(
        ReaderChromePreferences(sidebarOpen: true).toggledSidebar().sidebarOpen == false,
        "reader chrome closes sidebar"
    )
    check(
        ReaderChromePreferences(sidebarOpen: false).toggledSidebar().sidebarOpen == true,
        "reader chrome opens sidebar"
    )
    check(ReaderChromePreferences().zoomLevel == 100, "reader chrome starts at 100 percent zoom")
    check(
        ReaderChromePreferences(zoomLevel: 100).zoomedIn().zoomLevel == 110,
        "reader chrome zooms in by ten percent"
    )
    check(
        ReaderChromePreferences(zoomLevel: 100).zoomedOut().zoomLevel == 90,
        "reader chrome zooms out by ten percent"
    )
    check(
        ReaderChromePreferences(zoomLevel: 130).resetZoom().zoomLevel == 100,
        "reader chrome resets zoom"
    )
    check(
        ReaderChromePreferences(zoomLevel: 200).zoomedIn().zoomLevel == 200,
        "reader chrome clamps maximum zoom"
    )
    check(
        ReaderChromePreferences(zoomLevel: 60).zoomedOut().zoomLevel == 60,
        "reader chrome clamps minimum zoom"
    )
    check(
        ReaderChromePreferences().readingWidth == .standard,
        "reader chrome starts with standard reading width"
    )
    check(
        ReaderChromePreferences(readingWidth: .standard).documentMaxWidth == 768,
        "standard reading width matches web reader"
    )
    check(
        ReaderChromePreferences(readingWidth: .comfortable).documentMaxWidth == 896,
        "comfortable reading width matches web reader"
    )
    check(
        ReaderChromePreferences(readingWidth: .wide).documentMaxWidth == 1088,
        "wide reading width matches web reader"
    )
    check(
        ReaderChromePreferences().interfaceScale == .compact,
        "reader chrome starts with compact interface scale"
    )
    check(
        ReaderChromePreferences(interfaceScale: .compact).controlFontSize == 12,
        "compact interface scale uses compact control text"
    )
    check(
        ReaderChromePreferences(interfaceScale: .comfortable).controlFontSize == 13,
        "comfortable interface scale uses comfortable control text"
    )
    check(
        ReaderChromePreferences(interfaceScale: .large).controlFontSize == 14,
        "large interface scale uses large control text"
    )
    check(
        ReaderChromePreferences(interfaceScale: .large).sidebarWidth == 280,
        "large interface scale widens the sidebar"
    )
    check(
        ReaderChromePreferences(contentFont: .georgia).withContentFont(.charter).contentFont == .charter,
        "reader chrome updates content font"
    )
    check(
        ReaderChromePreferences(codeFont: .sfMono).withCodeFont(.systemMono).codeFont == .systemMono,
        "reader chrome updates code font"
    )
}

func checkDocumentReadingLayout() {
    let standard = DocumentReadingLayout(readingWidth: .standard, isWide: false)
    check(standard.maxContentWidth == 768, "standard layout uses 48rem reader width")
    check(standard.horizontalPadding == 48, "standard layout uses px-12 reader padding")
    check(standard.alignment == .center, "standard layout centers the reader column")

    let fullWidth = DocumentReadingLayout(readingWidth: .standard, isWide: true)
    check(fullWidth.maxContentWidth == nil, "wide layout removes the max reader width")
    check(fullWidth.alignment == .leading, "wide layout keeps full-width content left aligned")
}

func checkMarkdownTableGridStyle() {
    let style = MarkdownTableGridStyle.standard
    check(style.cornerRadius == 8, "table grid uses rounded 8px corners")
    check(style.horizontalPadding == 14, "table grid uses readable horizontal cell padding")
    check(style.verticalPadding == 10, "table grid uses readable vertical cell padding")
    check(style.headerFontScale == 0.8, "table grid uses compact header typography")
    check(style.bodyFontScale == 0.925, "table grid uses compact body typography")
}

func checkMarkdownCodeBlockStyle() {
    let style = MarkdownCodeBlockStyle.standard
    check(style.cornerRadius == 10, "code block uses rounded 10px corners")
    check(style.horizontalPadding == 18, "code block uses 18px horizontal padding")
    check(style.verticalPadding == 14, "code block uses 14px vertical padding")
    check(style.languageFontSize == 11, "code block uses compact language badge text")
    check(style.codeFontScale == 0.875, "code block uses compact code text scale")
    check(style.shadowOffsetY == 1, "code block uses subtle lifted edge")
}

func checkMarkdownInlineCodeStyle() {
    let style = MarkdownInlineCodeStyle.standard
    check(style.fontScale == 0.875, "inline code uses compact code text scale")
    check(style.horizontalPadding == 5, "inline code keeps compact horizontal chip padding")
    check(style.cornerRadius == 4, "inline code uses small chip radius")
}

func checkMarkdownBlockquoteStyle() {
    let style = MarkdownBlockquoteStyle.standard
    check(style.borderWidth == 3, "blockquote uses a 3px leading rule")
    check(style.horizontalPadding == 16, "blockquote uses 1em horizontal padding")
    check(style.verticalPadding == 6, "blockquote uses compact vertical padding")
    check(style.textIsMuted, "blockquote text uses muted foreground")
}

func checkMarkdownImageStyle() {
    let style = MarkdownImageStyle.standard
    check(style.cornerRadius == 8, "image blocks use rounded 8px corners")
    check(style.captionFontSize == 12, "image captions use compact secondary text")
}

func checkMarkdownHeadingPresentation() {
    let h1 = MarkdownHeadingPresentation(level: 1)
    check(h1.fontScale == 1.875, "h1 uses large reader heading scale")
    check(h1.emphasis == .bold, "h1 uses bold emphasis")
    check(!h1.isMuted, "h1 uses foreground color")

    let h4 = MarkdownHeadingPresentation(level: 4)
    check(h4.fontScale == 1.0, "h4 uses body-sized heading scale")
    check(h4.isMuted, "h4 uses muted foreground")
    check(!h4.isUppercase, "h4 keeps original case")

    let h5 = MarkdownHeadingPresentation(level: 5)
    check(h5.fontScale == 0.95, "h5 is smaller than h4")
    check(h5.isMuted, "h5 uses muted foreground")

    let h6 = MarkdownHeadingPresentation(level: 6)
    check(h6.fontScale == 0.875, "h6 uses compact heading scale")
    check(h6.isMuted, "h6 uses muted foreground")
    check(h6.isUppercase, "h6 is uppercased")
}

func checkKeyboardShortcutReference() {
    let groups = KeyboardShortcutReference.groups
    check(groups.map(\.heading) == ["Files", "Navigation", "View", "App"], "lists shortcut groups")
    check(
        groups.flatMap(\.items).contains(.init(label: "Open file", keys: "⌘ O")),
        "lists open file shortcut"
    )
    check(
        groups.flatMap(\.items).contains(.init(label: "Keyboard shortcuts", keys: "⌘ /")),
        "lists keyboard shortcuts shortcut"
    )
    check(
        groups.flatMap(\.items).contains(.init(label: "Reset zoom", keys: "⌘ 0")),
        "lists reset zoom shortcut"
    )
}

func checkWelcomeRecents() {
    let files = (1...8).map {
        MarkdownFileSummary(url: URL(fileURLWithPath: "/tmp/recent-\($0).md"))
    }

    check(
        WelcomeRecents.displayed(files).map(\.title) == [
            "recent-1.md", "recent-2.md", "recent-3.md",
            "recent-4.md", "recent-5.md", "recent-6.md",
        ],
        "welcome recents show the first six files"
    )
    check(
        RecentFileList.removing(files[2].url, from: files).map(\.title) == [
            "recent-1.md", "recent-2.md", "recent-4.md", "recent-5.md",
            "recent-6.md", "recent-7.md", "recent-8.md",
        ],
        "recent list removes the selected file"
    )

    let duplicateRecentFiles = [
        MarkdownFileSummary(url: URL(fileURLWithPath: "/tmp/Shared.md")),
        MarkdownFileSummary(url: URL(fileURLWithPath: "/private/tmp/Shared.md")),
        MarkdownFileSummary(url: URL(fileURLWithPath: "/tmp/Other.md")),
    ]
    check(
        RecentFileList.deduped(duplicateRecentFiles).map(\.title) == ["Shared.md", "Other.md"],
        "recent list dedupes equivalent temporary file URLs"
    )
    check(
        RecentFileList.removing(
            URL(fileURLWithPath: "/private/tmp/Shared.md"),
            from: duplicateRecentFiles
        ).map(\.title) == ["Other.md"],
        "recent list removes equivalent temporary file URLs"
    )
}

func checkQuickOpenSearch() {
    let files = [
        MarkdownFileSummary(url: URL(fileURLWithPath: "/tmp/Backend Performance Improvements.md")),
        MarkdownFileSummary(url: URL(fileURLWithPath: "/tmp/Project Roadmap.md")),
        MarkdownFileSummary(url: URL(fileURLWithPath: "/tmp/archive/Performance Notes.md")),
    ]

    check(
        QuickOpenSearch.results(query: "", files: files).map(\.file.title) == [
            "Backend Performance Improvements.md",
            "Project Roadmap.md",
            "Performance Notes.md",
        ],
        "quick open blank query preserves source order"
    )

    check(
        QuickOpenSearch.results(query: "bpi", files: files).map(\.file.title) == [
            "Backend Performance Improvements.md",
        ],
        "quick open matches non-contiguous filename initials"
    )

    check(
        QuickOpenSearch.results(query: "perf", files: files).map(\.file.title) == [
            "Performance Notes.md",
            "Backend Performance Improvements.md",
        ],
        "quick open ranks stronger filename matches first"
    )

    check(
        QuickOpenSearch.deduped([
            MarkdownFileSummary(url: URL(fileURLWithPath: "/tmp/Shared.md")),
            MarkdownFileSummary(url: URL(fileURLWithPath: "/private/tmp/Shared.md")),
        ]).count == 1,
        "quick open dedupes equivalent temporary file URLs"
    )
}

func checkCodeCopyFeedback() {
    check(
        CodeCopyFeedbackState().copiedCodeIDs.isEmpty,
        "code copy feedback starts empty"
    )
    check(
        CodeCopyFeedbackState().copying("block-1").isCopied("block-1"),
        "code copy feedback marks a copied block"
    )
    check(
        !CodeCopyFeedbackState().copying("block-1").isCopied("block-2"),
        "code copy feedback keeps other blocks in default state"
    )
    check(
        !CodeCopyFeedbackState().copying("block-1").resetting("block-1").isCopied("block-1"),
        "code copy feedback resets a copied block"
    )
}

func checkDocumentTabOperations() {
    let urls = ["one.md", "two.md", "three.md", "four.md"].map {
        URL(fileURLWithPath: "/tmp/\($0)")
    }

    check(
        DocumentTabOperations.closeOthers(in: urls, keeping: urls[1]) == [urls[1]],
        "tab operations close other documents"
    )
    check(
        DocumentTabOperations.closeToRight(in: urls, after: urls[1]) == [urls[0], urls[1]],
        "tab operations close documents to the right"
    )
    check(
        DocumentTabOperations.closeToRight(in: urls, after: URL(fileURLWithPath: "/tmp/missing.md")) == urls,
        "tab operations ignore missing close-to-right target"
    )
}

func checkDocumentBreadcrumbSegments() {
    let fileURL = URL(fileURLWithPath: "/Users/zain/projects/mdow/notes/today.md")
    let segments = DocumentBreadcrumbSegments.parentSegments(for: fileURL, rootURL: nil)
    check(
        segments.map(\.name) == ["projects", "mdow", "notes"],
        "breadcrumb shows last three parent directories"
    )
    check(
        segments.map(\.url.path) == [
            "/Users/zain/projects",
            "/Users/zain/projects/mdow",
            "/Users/zain/projects/mdow/notes",
        ],
        "breadcrumb segments keep revealable absolute paths"
    )

    let rooted = DocumentBreadcrumbSegments.parentSegments(
        for: URL(fileURLWithPath: "/Users/zain/projects/mdow/docs/specs/today.md"),
        rootURL: URL(fileURLWithPath: "/Users/zain/projects/mdow")
    )
    check(
        rooted.map(\.name) == ["mdow", "docs", "specs"],
        "breadcrumb anchors at open folder when possible"
    )

    check(
        DocumentDisplayTitle.secondaryFilename(
            documentTitle: "Native Project Brief",
            fileURL: URL(fileURLWithPath: "/tmp/mdow-native-frontmatter-title.md")
        ) == "mdow-native-frontmatter-title.md",
        "breadcrumb keeps filename visible when frontmatter title differs"
    )
    check(
        DocumentDisplayTitle.secondaryFilename(
            documentTitle: "today.md",
            fileURL: URL(fileURLWithPath: "/tmp/today.md")
        ) == nil,
        "breadcrumb omits secondary filename when title is the filename"
    )
}

func checkFolderTreeContextActions() {
    check(
        FolderTreeContextActions.actions(isFolder: false).map(\.title) == [
            "Open", "Copy Path", "Reveal in Finder",
        ],
        "folder tree file rows expose file context actions"
    )
    check(
        FolderTreeContextActions.actions(isFolder: true).map(\.title) == [
            "Copy Path", "Reveal in Finder",
        ],
        "folder tree folder rows expose folder context actions"
    )
}

func checkFolderScanner() throws {
    let root = FileManager.default.temporaryDirectory
        .appendingPathComponent("MdowNativeChecks-\(UUID().uuidString)")
    let nested = root.appendingPathComponent("nested")
    try FileManager.default.createDirectory(at: nested, withIntermediateDirectories: true)
    try "# A".write(to: root.appendingPathComponent("a.md"), atomically: true, encoding: .utf8)
    try "# B".write(to: nested.appendingPathComponent("b.MARKDOWN"), atomically: true, encoding: .utf8)
    try "notes".write(to: root.appendingPathComponent("notes.txt"), atomically: true, encoding: .utf8)
    defer { try? FileManager.default.removeItem(at: root) }

    let files = try MarkdownFolder.scan(root)
    check(files.map(\.title) == ["a.md", "b.MARKDOWN"], "scans supported Markdown files only")
}

func checkTreeSitterSemanticHighlighter() throws {
    let source = """
    type CommandKind = 'file' | 'navigation'

    interface Command {
      readonly id: string
      run(): Promise<void>
    }

    const runCommand = async (command: Command): Promise<void> => {
      await command.run()
    }
    """

    let highlights = try TreeSitterSemanticHighlighter.highlightRanges(
        in: source,
        language: "typescript"
    )
    let names = Set(highlights.map(\.name))

    check(names.contains(.keyword), "Tree-sitter highlights TypeScript keywords")
    check(names.contains(.type), "Tree-sitter highlights TypeScript type names")
    check(names.contains(.string), "Tree-sitter highlights TypeScript strings")
    check(names.contains(.function), "Tree-sitter highlights TypeScript functions")
    check(
        highlights.allSatisfy { NSMaxRange($0.range) <= (source as NSString).length },
        "Tree-sitter highlight ranges stay inside source"
    )

    let unsupportedHighlights = try TreeSitterSemanticHighlighter.highlightRanges(
        in: "let value = 1",
        language: "swift"
    )
    check(unsupportedHighlights.isEmpty, "unsupported languages render without fake highlighting")
}

do {
    try checkMarkdownFileSupport()
    try checkMarkdownOpenURLRouting()
    checkMarkdownParser()
    checkMarkdownInlineNormalization()
    checkMarkdownLinkResolver()
    checkMarkdownSearch()
    checkMarkdownFileErrors()
    checkReaderChromePreferences()
    checkDocumentReadingLayout()
    checkMarkdownTableGridStyle()
    checkMarkdownCodeBlockStyle()
    checkMarkdownInlineCodeStyle()
    checkMarkdownBlockquoteStyle()
    checkMarkdownImageStyle()
    checkMarkdownHeadingPresentation()
    checkKeyboardShortcutReference()
    checkWelcomeRecents()
    checkQuickOpenSearch()
    checkCodeCopyFeedback()
    checkDocumentTabOperations()
    checkDocumentBreadcrumbSegments()
    checkFolderTreeContextActions()
    try checkFolderScanner()
    try checkTreeSitterSemanticHighlighter()
    print("MdowNativeCoreChecks passed")
} catch {
    fputs("FAIL: \(error)\n", stderr)
    exit(1)
}

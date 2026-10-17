import AppKit
import Testing

@testable import MdowKit

@Suite struct DocumentTests {
  @Test func frontmatterIsSplitAndTitled() {
    let source = MarkdownSource("---\ntitle: \"Hello\"\ntags: [a]\n---\n# Body\n")
    #expect(source.title == "Hello")
    #expect(source.body == "# Body\n")
    #expect(MarkdownSource("---\nnot closed\n# x").frontmatter == nil)
  }

  @Test func footnotesAreNumberedByFirstReference() {
    let (body, notes) = Footnotes.extract(from: "A[^b] then[^a].\n\n[^a]: First def\n[^b]: Second\n    more\n")
    #expect(!body.contains("[^a]:"))
    #expect(notes.number(for: "b") == 1)
    #expect(notes.number(for: "a") == 2)
    #expect(notes.notes.first?.text == "Second\nmore")
  }

  @Test func footnotesInCodeFencesAreLeftAlone() {
    let (body, notes) = Footnotes.extract(from: "```\n[^x]: not a note\n```\n")
    #expect(notes.notes.isEmpty)
    #expect(body.contains("[^x]:"))
  }

  @Test func slugsMatchGitHub() {
    var slugger = Slugger()
    #expect(slugger.next("Hello, World!") == "hello-world")
    #expect(slugger.next("Hello, World!") == "hello-world-1")
    #expect(Slugger.slug("API & SDK v2.0") == "api--sdk-v20")
  }

  @Test func htmlSnippets() {
    let images = HTMLSnippet.images(in: #"<p align="center"><a href="https://x.dev"><img src="logo.png" width="120" alt="Logo"></a></p>"#)
    #expect(images.count == 1)
    #expect(images.first?.src == "logo.png")
    #expect(images.first?.width == 120)
    #expect(images.first?.link?.absoluteString == "https://x.dev")
    #expect(HTMLSnippet.isCentered(#"<div align="center">"#))
    #expect(HTMLSnippet.text(in: "<details><summary>More</summary>Body &amp; soul<!-- hidden --></details>") == "More\nBody & soul")
    #expect(HTMLSnippet.attribute("src", in: #"<img data-src="a" src='b'>"#) == "b")
  }

  @Test func fuzzyMatchPrefersWordStarts() {
    let start = FuzzyMatch.score("rd", in: "README.md")
    let middle = FuzzyMatch.score("rd", in: "border.md")
    #expect(start != nil && middle != nil && start! > middle!)
    #expect(FuzzyMatch.score("zz", in: "README.md") == nil)
  }

  @Test func fileKinds() {
    #expect(FileKind(url: URL(fileURLWithPath: "/a/b.MD")) == .markdown)
    #expect(FileKind(url: URL(fileURLWithPath: "/a/b.mdx")) == .markdown)
    #expect(FileKind(url: URL(fileURLWithPath: "/a/b.htm")) == .html)
    #expect(FileKind(url: URL(fileURLWithPath: "/a/b.txt")) == nil)
  }
}

@Suite struct RendererTests {
  func render(_ markdown: String, assets: RenderAssets = .none) -> RenderedDocument {
    MarkdownRenderer.render(MarkdownSource(markdown), style: ReaderStyle(), baseURL: URL(fileURLWithPath: "/docs/"), assets: assets)
  }

  @Test func headingsBecomeTheOutline() {
    let doc = render("# One\n\ntext\n\n## Two\n\n> ## Quoted\n\n### Three")
    #expect(doc.headings.map(\.title) == ["One", "Two", "Three"])
    #expect(doc.headings.map(\.anchor) == ["one", "two", "three"])
    #expect(doc.title == "One")
    let string = doc.text.string as NSString
    #expect(string.substring(with: NSRange(location: doc.headings[1].location, length: 3)) == "Two")
  }

  @Test func linksResolveAgainstTheDocument() {
    let doc = render("[a](other.md#part) [b](#local) [c](https://example.com)")
    var links: [URL] = []
    doc.text.enumerateAttribute(.link, in: NSRange(location: 0, length: doc.text.length)) { value, _, _ in
      if let url = value as? URL { links.append(url) }
    }
    #expect(links[0].path == "/docs/other.md")
    #expect(links[0].fragment == "part")
    #expect(links[1].scheme == anchorScheme)
    #expect(links[2].host == "example.com")
  }

  @Test func pendingAssetsAreReported() {
    let doc = render("![x](https://img.test/a.png)\n\n```mermaid\ngraph TD; A-->B\n```")
    #expect(doc.pendingImages == [URL(string: "https://img.test/a.png")!])
    #expect(doc.pendingDiagrams == ["graph TD; A-->B"])
  }

  @Test func failedDiagramsShowTheirSource() {
    let doc = render("```mermaid\ngraph TD; A-->B\n```", assets: RenderAssets(image: { _ in .failed }, diagram: { _ in .failed }))
    #expect(doc.text.string.contains("graph TD; A-->B"))
    #expect(doc.pendingDiagrams.isEmpty)
  }

  @Test func alertsDropTheirMarker() {
    let doc = render("> [!TIP]\n> Use the sidebar.")
    #expect(doc.text.string.contains("Tip"))
    #expect(!doc.text.string.contains("[!TIP]"))
    #expect(doc.text.string.contains("Use the sidebar."))
  }

  @Test func tablesUseTextTables() {
    let doc = render("| a | b |\n|---|--:|\n| 1 | 2 |")
    let index = (doc.text.string as NSString).range(of: "2").location
    let style = doc.text.attribute(.paragraphStyle, at: index, effectiveRange: nil) as? NSParagraphStyle
    #expect(style?.textBlocks.last is NSTextTableBlock)
    #expect(style?.alignment == .right)
  }

  @Test func largeDocumentsRenderQuickly() {
    var markdown = ""
    for index in 0..<4000 {
      markdown += "## Section \(index)\n\nSome **bold** text with a [link](x.md) and `code`.\n\n- one\n- two\n\n```js\nconst a = \(index)\n```\n\n"
    }
    let start = Date()
    let doc = render(markdown)
    #expect(doc.headings.count == 4000)
    #expect(Date().timeIntervalSince(start) < 20)
  }
}

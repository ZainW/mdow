import AppKit
import Testing

@testable import MdowKit

@Suite struct MathExtractionTests {
  func tex(_ markdown: String) -> [String] {
    MathSpans.extract(from: markdown).1.spans.map(\.tex)
  }

  @Test func inlineFollowsGitHubRules() {
    #expect(tex("Area $\\pi r^2$ here") == ["\\pi r^2"])
    #expect(tex("It costs $5 and $10.") == [])
    #expect(tex("$ x$ and $x $") == [])
    #expect(tex("$x$1") == [])
    #expect(tex("\\$x\\$ and \\$y$") == [])
    #expect(tex("$`a_b`$ and $``x`y``$") == ["a_b", "x`y"])
    #expect(tex("$a$$b$") == ["a", "b"])
    #expect(tex("| $\\left. x \\right|$ | $\\Bigg($ | $\\big($ |") == ["\\left. x \\right|", "\\Bigg(", "\\big("])
  }

  @Test func codeIsLeftAlone() {
    #expect(tex("`$x$` and ``$y$``") == [])
    #expect(tex("```\n$x$\n```\n\n~~~js\n$$y$$\n~~~\n") == [])
    #expect(tex("text\n\n    $x$ code\n\nafter $y$") == ["y"])
    #expect(tex("- item\n\n    $x$ continued") == ["x"])
    #expect(tex("> ```\n> $x$\n> ```") == [])
  }

  @Test func displayMath() {
    let (body, math) = MathSpans.extract(from: "$$\n\\frac{a}{b}\n$$\n\nOne $$x$$ line\n\n> $$\n> a \\\\\n> b\n> $$")
    #expect(math.spans.map(\.tex) == ["\\frac{a}{b}", "x", "a \\\\\nb"])
    #expect(math.spans.allSatisfy { $0.display })
    #expect(!body.contains("$"))
    #expect(math.restoringSource(body).contains("> $$\n> a \\\\\n> b\n> $$"))
  }

  @Test func placeholdersRoundTrip() {
    let source = "A $x_1$ b $$y$$ c"
    let (body, math) = MathSpans.extract(from: source)
    #expect(math.restoringSource(body) == source)
    #expect(math.replacingWithTeX(body) == "A x_1 b y c")
    // Private-use characters that aren't ours are kept.
    #expect(math.restoringSource("\u{E000}9\u{E001} \u{E000}x") == "\u{E000}9\u{E001} \u{E000}x")
  }
}

@Suite struct MathRenderTests {
  func render(_ markdown: String) -> RenderedDocument {
    MarkdownRenderer.render(MarkdownSource(markdown), style: ReaderStyle(), baseURL: nil)
  }

  func formulas(_ doc: RenderedDocument) -> [(MathSource, NSTextAttachment?, NSRange)] {
    var found: [(MathSource, NSTextAttachment?, NSRange)] = []
    doc.text.enumerateAttribute(.mdowMathSource, in: NSRange(location: 0, length: doc.text.length)) {
      value, range, _ in
      guard let source = value as? MathSource else { return }
      let attachment = doc.text.attribute(.attachment, at: range.location, effectiveRange: nil)
      found.append((source, attachment as? NSTextAttachment, range))
    }
    return found
  }

  @Test func texSurvivesMarkdown() {
    let doc = render("Sum $a_1 * b_2 * c_3$ and $\\{x\\}$ and $a\\,b$.")
    let found = formulas(doc)
    #expect(found.map(\.0.tex) == ["a_1 * b_2 * c_3", "\\{x\\}", "a\\,b"])
    #expect(found.allSatisfy { $0.1 != nil })
    #expect(!doc.text.string.contains("\u{E000}"))
  }

  @Test func inlineMathSitsOnTheBaseline() throws {
    let doc = render("Text $y_q$ more")
    let attachment = try #require(formulas(doc).first?.1)
    // A descender drops below the baseline.
    #expect(attachment.bounds.minY < 0)
    #expect(attachment.bounds.height > 0)
  }

  @Test func displayMathIsCentredOnItsOwnLine() throws {
    let doc = render("Before\n\n$$\n\\sum_{i=1}^n i\n$$\n\nText $$x^2$$ after\n\n```math\n\\frac12\n```")
    let found = formulas(doc)
    #expect(found.count == 3)
    for (source, attachment, range) in found {
      #expect(source.display)
      #expect(attachment != nil)
      let style = doc.text.attribute(.paragraphStyle, at: range.location, effectiveRange: nil) as? NSParagraphStyle
      #expect(style?.alignment == .center)
    }
    let lines = doc.text.string.components(separatedBy: "\n")
    #expect(lines.contains("Text"))
    #expect(lines.contains("after"))
  }

  @Test func headingsUseTheTeX() {
    let doc = render("# Area $\\pi r^2$\n")
    #expect(doc.headings.first?.title == "Area \\pi r^2")
    #expect(doc.title == "Area \\pi r^2")
    #expect(doc.headings.first?.anchor == "area-pi-r2")
  }

  @Test func footnotesKeepTheirMath() {
    let doc = render("See[^n] and $[^n]$.\n\n[^n]: Variance $\\sigma^2$.\n")
    #expect(formulas(doc).map(\.0.tex) == ["[^n]", "\\sigma^2"])
  }

  @Test func mathInCodeShowsItsSource() {
    let doc = render("`$x$`\n\n<span title=\"$y$\">z</span>\n\n```\n$$a$$\n```")
    #expect(doc.text.string.contains("$x$"))
    #expect(doc.text.string.contains("$$a$$"))
    #expect(formulas(doc).isEmpty)
  }

  @Test func brokenMathShowsItsSource() {
    let doc = render("Bad $\\frac{1}{$ inline\n\n$$\n\\begin{nope} x \\end{nope}\n$$\n\n```math\n\\left(\n```")
    let string = doc.text.string
    #expect(string.contains("$\\frac{1}{$"))
    #expect(string.contains("\\begin{nope} x \\end{nope}"))
    #expect(string.contains("\\left("))
    let index = (string as NSString).range(of: "\\begin{nope}").location
    let style = doc.text.attribute(.paragraphStyle, at: index, effectiveRange: nil) as? NSParagraphStyle
    #expect(style?.textBlocks.last is CodeTextBlock)
  }

  @Test func malformedTeXNeverCrashes() {
    let inputs = [
      "\\", "{", "}", "^", "_", "^^", "x__y", "\\left", "\\right)", "\\frac", "\\sqrt[", "&", "\\\\",
      "\\begin{matrix}", "\\end{matrix}", "\\begin{aligned} a &= b", "\\color{red}", "\\mathbb",
      "\\text{", "\\overbrace{x}^", "\\unknowncommand", "a & b \\\\ c", "%comment", "#", "~",
    ]
    for input in inputs {
      _ = render("$\(input)$ and $$\(input)$$")
    }
  }

  @Test func commonKaTeXCommandsAreRewritten() {
    #expect(MathFormula.compatible("\\dfrac{1}{2} \\operatorname{tr} A") == "\\frac{1}{2} \\mathrm{tr} A")
    #expect(MathFormula.compatible("a \\not= b \\pmod{n}") == "a \\neq b \\quad(\\mathrm{mod}\\ n)")
    #expect(MathFormula.compatible("\\begin{align*} a \\end{align*}") == "\\begin{aligned} a \\end{aligned}")
    #expect(MathFormula.compatible("\\ltimes \\bigcup") == "\\ltimes \\bigcup")
    let doc = render("$\\dfrac{a}{b}$ $\\boldsymbol{x}$ $\\big( x \\big)$")
    #expect(formulas(doc).allSatisfy { $0.1 != nil })
  }

  @Test func copyingGivesTheTeX() {
    let doc = render("Area $\\pi r^2$ and\n\n$$\nE = mc^2\n$$")
    let text = doc.text.mdowPlainText(in: NSRange(location: 0, length: doc.text.length))
    #expect(text.contains("Area $\\pi r^2$ and"))
    #expect(text.contains("$$E = mc^2$$"))
  }

  @Test func rendersFromManyThreads() {
    let markdown = (0..<40).map { "Line $x_{\($0)} + \\frac{\($0)}{2}$ and $$\\sum_{i=\($0)} i$$" }
      .joined(separator: "\n\n")
    DispatchQueue.concurrentPerform(iterations: 8) { _ in
      _ = MarkdownRenderer.render(MarkdownSource(markdown), style: ReaderStyle(), baseURL: nil)
    }
  }

  /// The same image draws dark text in light mode and light text in dark mode.
  @Test func formulasFollowTheAppearance() throws {
    let formula = try MathFormula.make(
      "\\mathbf{WM}\\bullet", fontSize: 40, display: false, color: ReaderColors.text
    ).get()
    func inkBrightness(_ name: NSAppearance.Name) -> CGFloat {
      let size = formula.size
      let rep = NSBitmapImageRep(
        bitmapDataPlanes: nil, pixelsWide: Int(size.width), pixelsHigh: Int(size.height),
        bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true, isPlanar: false,
        colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0)!
      NSGraphicsContext.saveGraphicsState()
      NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: rep)
      NSAppearance(named: name)!.performAsCurrentDrawingAppearance {
        formula.image.draw(in: NSRect(origin: .zero, size: size))
      }
      NSGraphicsContext.restoreGraphicsState()
      // The most opaque pixel is solid ink.
      var best: (alpha: CGFloat, brightness: CGFloat) = (0, 0)
      for x in 0..<rep.pixelsWide {
        for y in 0..<rep.pixelsHigh {
          guard let color = rep.colorAt(x: x, y: y), color.alphaComponent > best.alpha else { continue }
          best = (color.alphaComponent, color.brightnessComponent)
        }
      }
      return best.brightness
    }
    #expect(inkBrightness(.aqua) < 0.3)
    #expect(inkBrightness(.darkAqua) > 0.7)
  }
}

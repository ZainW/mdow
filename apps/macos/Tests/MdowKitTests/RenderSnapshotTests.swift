import AppKit
import Testing

@testable import MdowKit

/// Draws a document offscreen. Set MDOW_SNAPSHOT=/path/file.md to render that file to
/// /tmp/mdow-snapshot.png for a look at layout changes without launching the app.
@MainActor @Suite struct RenderSnapshotTests {
  @Test func codeBlocksGetABlock() {
    let source = MarkdownSource("Text\n\n```swift\nlet a = 1\n  let b = 2\n}\n```\n")
    let rendered = MarkdownRenderer.render(source, style: ReaderStyle(), baseURL: nil)
    let string = rendered.text.string as NSString
    let index = string.range(of: "let b").location
    let style = rendered.text.attribute(.paragraphStyle, at: index, effectiveRange: nil) as? NSParagraphStyle
    #expect(style?.textBlocks.last is CodeTextBlock)
    let close = string.range(of: "}").location
    let closeStyle = rendered.text.attribute(.paragraphStyle, at: close, effectiveRange: nil) as? NSParagraphStyle
    #expect(closeStyle?.textBlocks.last === style?.textBlocks.last)
  }

  @Test func snapshot() throws {
    guard let path = ProcessInfo.processInfo.environment["MDOW_SNAPSHOT"] else { return }
    let url = URL(fileURLWithPath: path)
    let dark = ProcessInfo.processInfo.environment["MDOW_SNAPSHOT_DARK"] != nil
    var style = ReaderStyle()
    style.maxContentWidth = 720
    let rendered = MarkdownRenderer.render(
      MarkdownSource(try String(contentsOf: url, encoding: .utf8)), style: style,
      baseURL: url.deletingLastPathComponent(),
      assets: RenderAssets(
        image: { url in NSImage(contentsOf: url).map(AssetState.ready) ?? .failed },
        diagram: { _ in .failed }))
    let storage = NSTextStorage(attributedString: rendered.text)
    let layout = ReaderLayoutManager()
    storage.addLayoutManager(layout)
    let container = NSTextContainer(size: NSSize(width: 720, height: CGFloat.greatestFiniteMagnitude))
    container.lineFragmentPadding = 0
    layout.addTextContainer(container)
    let view = NSTextView(frame: NSRect(x: 0, y: 0, width: 800, height: 100), textContainer: container)
    view.appearance = NSAppearance(named: dark ? .darkAqua : .aqua)
    view.textContainerInset = NSSize(width: 40, height: 30)
    view.backgroundColor = NSColor.textBackgroundColor
    layout.ensureLayout(for: container)
    let height = layout.usedRect(for: container).height + 60
    view.frame = NSRect(x: 0, y: 0, width: 800, height: min(height, 6000))
    let rep = view.bitmapImageRepForCachingDisplay(in: view.bounds)!
    view.appearance?.performAsCurrentDrawingAppearance {
      view.cacheDisplay(in: view.bounds, to: rep)
    }
    try rep.representation(using: .png, properties: [:])!.write(to: URL(fileURLWithPath: "/tmp/mdow-snapshot.png"))
  }
}

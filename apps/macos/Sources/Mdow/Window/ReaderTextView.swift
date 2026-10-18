import AppKit
import MdowKit

/// The document view: a read-only TextKit 1 text view whose text sits in a centred column.
/// TextKit 1 because it lays out `NSTextTable` and `NSTextBlock`, which the renderer uses for
/// tables, code blocks and quotes.
final class ReaderTextView: NSTextView {
  /// Returns the column width for the available width.
  var columnWidth: (CGFloat) -> CGFloat = { $0 }
  var onOpenFiles: (([URL]) -> Void)?
  var minimumMargin: CGFloat = 32

  private var hovered: (block: CodeTextBlock, info: CodeBlockInfo, rect: NSRect)?
  private lazy var copyButton: NSButton = {
    let button = NSButton(
      title: "Copy", image: NSImage(systemSymbolName: "doc.on.doc", accessibilityDescription: nil)!,
      target: self, action: #selector(copyHoveredCode))
    button.bezelStyle = .accessoryBarAction
    button.controlSize = .small
    button.font = .systemFont(ofSize: NSFont.smallSystemFontSize)
    button.imagePosition = .imageLeading
    button.isHidden = true
    button.toolTip = "Copy code"
    addSubview(button)
    return button
  }()
  private var copyResetTask: Task<Void, Never>?

  static func make() -> ReaderTextView {
    let storage = NSTextStorage()
    let layout = ReaderLayoutManager()
    layout.allowsNonContiguousLayout = true
    layout.usesFontLeading = true
    storage.addLayoutManager(layout)
    let container = NSTextContainer(size: NSSize(width: 0, height: CGFloat.greatestFiniteMagnitude))
    container.widthTracksTextView = true
    container.lineFragmentPadding = 0
    layout.addTextContainer(container)
    let view = ReaderTextView(frame: .zero, textContainer: container)
    view.configure()
    return view
  }

  private func configure() {
    isEditable = false
    isSelectable = true
    isRichText = true
    importsGraphics = false
    allowsUndo = false
    usesFindBar = true
    isIncrementalSearchingEnabled = true
    drawsBackground = true
    backgroundColor = Palette.paper
    selectedTextAttributes = [.backgroundColor: Palette.textSelection]
    isVerticallyResizable = true
    isHorizontallyResizable = false
    autoresizingMask = [.width]
    maxSize = NSSize(width: CGFloat.greatestFiniteMagnitude, height: CGFloat.greatestFiniteMagnitude)
    displaysLinkToolTips = false
    linkTextAttributes = [
      .foregroundColor: Palette.accent,
      .cursor: NSCursor.pointingHand,
    ]
    setAccessibilityLabel("Document")
    registerForDraggedTypes([.fileURL])
  }

  // MARK: Column

  override func setFrameSize(_ newSize: NSSize) {
    super.setFrameSize(newSize)
    updateInsets()
  }

  func updateInsets() {
    let available = max(bounds.width - minimumMargin * 2, 200)
    let column = columnWidth(available)
    let side = max(minimumMargin, ((bounds.width - column) / 2).rounded(.down))
    let inset = NSSize(width: side, height: 30)
    if textContainerInset != inset { textContainerInset = inset }
  }

  var currentColumnWidth: CGFloat {
    max(bounds.width - textContainerInset.width * 2, 200)
  }

  // MARK: Code block copy button

  override func updateTrackingAreas() {
    super.updateTrackingAreas()
    for area in trackingAreas where area.owner === self && area.userInfo?["mdow"] != nil {
      removeTrackingArea(area)
    }
    addTrackingArea(
      NSTrackingArea(
        rect: .zero, options: [.mouseMoved, .mouseEnteredAndExited, .activeInKeyWindow, .inVisibleRect],
        owner: self, userInfo: ["mdow": true]))
  }

  override func mouseMoved(with event: NSEvent) {
    super.mouseMoved(with: event)
    updateHover(at: convert(event.locationInWindow, from: nil))
  }

  override func mouseExited(with event: NSEvent) {
    super.mouseExited(with: event)
    let point = convert(event.locationInWindow, from: nil)
    if !copyButton.frame.contains(point) { setHovered(nil) }
  }

  /// Re-checks hover after scrolling moved the text under a still pointer.
  func refreshHover() {
    guard let window else { return }
    updateHover(at: convert(window.mouseLocationOutsideOfEventStream, from: nil))
  }

  func codeBlock(at point: NSPoint) -> (block: CodeTextBlock, info: CodeBlockInfo, rect: NSRect)? {
    guard let layoutManager, let textContainer, let storage = textStorage, storage.length > 0 else {
      return nil
    }
    let containerPoint = NSPoint(x: point.x - textContainerOrigin.x, y: point.y - textContainerOrigin.y)
    let glyph = layoutManager.glyphIndex(for: containerPoint, in: textContainer)
    let index = layoutManager.characterIndexForGlyph(at: glyph)
    guard index < storage.length,
      let info = storage.attribute(.mdowCodeBlock, at: index, effectiveRange: nil) as? CodeBlockInfo,
      let style = storage.attribute(.paragraphStyle, at: index, effectiveRange: nil) as? NSParagraphStyle,
      let block = style.textBlocks.last as? CodeTextBlock
    else { return nil }
    var rect = layoutManager.boundsRect(for: block, at: glyph, effectiveRange: nil)
    rect.origin.x += textContainerOrigin.x
    rect.origin.y += textContainerOrigin.y
    rect.size.height -= block.width(for: .margin, edge: .maxY)
    rect.origin.x += block.width(for: .margin, edge: .minX)
    rect.size.width -= block.width(for: .margin, edge: .minX)
    return rect.contains(point) ? (block, info, rect) : nil
  }

  private func updateHover(at point: NSPoint) {
    if let hovered, copyButton.frame.contains(point), !copyButton.isHidden {
      _ = hovered
      return
    }
    setHovered(codeBlock(at: point))
  }

  private func setHovered(_ next: (block: CodeTextBlock, info: CodeBlockInfo, rect: NSRect)?) {
    if hovered?.info === next?.info, hovered?.rect == next?.rect { return }
    if let hovered {
      hovered.block.hidesLabel = false
      setNeedsDisplay(hovered.rect)
    }
    hovered = next
    guard let next else {
      copyButton.isHidden = true
      return
    }
    next.block.hidesLabel = true
    setNeedsDisplay(next.rect)
    resetCopyButton()
    copyButton.sizeToFit()
    let size = copyButton.frame.size
    copyButton.frame = NSRect(
      x: next.rect.maxX - size.width - 6, y: next.rect.minY + 5, width: size.width, height: size.height)
    copyButton.isHidden = false
  }

  @objc private func copyHoveredCode() {
    guard let hovered else { return }
    copyToPasteboard(hovered.info.code)
    copyButton.title = "Copied"
    copyButton.image = NSImage(systemSymbolName: "checkmark", accessibilityDescription: nil)
    copyButton.sizeToFit()
    let size = copyButton.frame.size
    copyButton.frame = NSRect(
      x: hovered.rect.maxX - size.width - 6, y: hovered.rect.minY + 5, width: size.width, height: size.height)
    copyResetTask?.cancel()
    copyResetTask = Task { [weak self] in
      try? await Task.sleep(for: .seconds(1.6))
      guard !Task.isCancelled else { return }
      self?.resetCopyButton()
    }
  }

  private func resetCopyButton() {
    copyButton.title = "Copy"
    copyButton.image = NSImage(systemSymbolName: "doc.on.doc", accessibilityDescription: nil)
  }

  private func copyToPasteboard(_ string: String) {
    NSPasteboard.general.clearContents()
    NSPasteboard.general.setString(string, forType: .string)
  }

  /// Called when the text is replaced, so a stale hover doesn't linger.
  func resetHover() {
    hovered = nil
    copyButton.isHidden = true
  }

  // MARK: Menu

  override func menu(for event: NSEvent) -> NSMenu? {
    let menu = super.menu(for: event) ?? NSMenu()
    let point = convert(event.locationInWindow, from: nil)
    if let code = codeBlock(at: point) {
      let item = NSMenuItem(title: "Copy Code Block", action: #selector(copyCodeFromMenu(_:)), keyEquivalent: "")
      item.target = self
      item.representedObject = code.info.code
      menu.insertItem(item, at: 0)
      menu.insertItem(.separator(), at: 1)
    }
    return menu
  }

  @objc private func copyCodeFromMenu(_ sender: NSMenuItem) {
    if let code = sender.representedObject as? String { copyToPasteboard(code) }
  }

  // MARK: Dropping files

  private func droppedURLs(_ info: NSDraggingInfo) -> [URL] {
    let urls =
      info.draggingPasteboard.readObjects(forClasses: [NSURL.self], options: [.urlReadingFileURLsOnly: true])
      as? [URL] ?? []
    return urls.filter { url in
      var isFolder: ObjCBool = false
      FileManager.default.fileExists(atPath: url.path, isDirectory: &isFolder)
      return isFolder.boolValue || FileKind.isOpenable(url)
    }
  }

  override func draggingEntered(_ sender: NSDraggingInfo) -> NSDragOperation {
    droppedURLs(sender).isEmpty ? [] : .copy
  }

  override func draggingUpdated(_ sender: NSDraggingInfo) -> NSDragOperation {
    droppedURLs(sender).isEmpty ? [] : .copy
  }

  override func prepareForDragOperation(_ sender: NSDraggingInfo) -> Bool { true }

  override func performDragOperation(_ sender: NSDraggingInfo) -> Bool {
    let urls = droppedURLs(sender)
    guard !urls.isEmpty else { return false }
    onOpenFiles?(urls)
    return true
  }
}

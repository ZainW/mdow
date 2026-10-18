import AppKit
import MdowKit
import SwiftUI
import WebKit

@MainActor protocol ReaderDelegate: AnyObject {
  func reader(_ reader: ReaderViewController, open url: URL, inNewTab: Bool)
  func reader(_ reader: ReaderViewController, openFiles urls: [URL])
  func reader(_ reader: ReaderViewController, didScrollToHeading index: Int?)
  func readerColumnWidthChanged(_ reader: ReaderViewController)
}

/// The right-hand side of a window: the welcome screen, a markdown document, an HTML page, or
/// a message saying why a file couldn't be shown.
final class ReaderViewController: NSViewController, NSTextViewDelegate, WKNavigationDelegate {
  enum Mode: Equatable {
    case welcome, markdown, html, message
  }

  weak var delegate: ReaderDelegate?
  private(set) var mode: Mode = .welcome
  private(set) var headings: [OutlineHeading] = []

  let textView = ReaderTextView.make()
  let scrollView = NSScrollView()

  private var webView: WKWebView?
  private var hostingView: NSView?
  private var lastColumnWidth: CGFloat = 0
  private var lastHeading: Int?
  private var suppressScrollTracking = false

  override func loadView() {
    let root = DropView()
    root.onOpenFiles = { [weak self] urls in
      guard let self else { return }
      self.delegate?.reader(self, openFiles: urls)
    }
    view = root

    scrollView.hasVerticalScroller = true
    scrollView.hasHorizontalScroller = false
    scrollView.autohidesScrollers = true
    scrollView.drawsBackground = true
    scrollView.backgroundColor = Palette.paper
    scrollView.documentView = textView
    scrollView.contentView.postsBoundsChangedNotifications = true
    scrollView.translatesAutoresizingMaskIntoConstraints = false
    textView.delegate = self
    textView.onFindAction = { [weak self] action in self?.performFind(action) }
    findBar.isHidden = true
    findBar.translatesAutoresizingMaskIntoConstraints = false
    findBar.onSearch = { [weak self] query in self?.search(query) }
    findBar.onStep = { [weak self] backwards in self?.step(backwards: backwards) }
    findBar.onClose = { [weak self] in self?.hideFindBar() }
    textView.columnWidth = { Preferences.shared.columnWidth(available: $0) }
    textView.onOpenFiles = { [weak self] urls in
      guard let self else { return }
      self.delegate?.reader(self, openFiles: urls)
    }
    NotificationCenter.default.addObserver(
      self, selector: #selector(didScroll), name: NSView.boundsDidChangeNotification,
      object: scrollView.contentView)
    NotificationCenter.default.addObserver(
      self, selector: #selector(didResize), name: NSView.frameDidChangeNotification, object: textView)
    // The welcome screen is built when the view appears, so a window that opens straight to a
    // document never lays it out.
  }

  override func viewWillAppear() {
    super.viewWillAppear()
    if mode == .welcome, hostingView == nil { showWelcome() }
  }

  /// Clears the welcome screen while a document loads, so it doesn't flash up first.
  func showLoading() {
    guard mode == .welcome else { return }
    hostingView = nil
    for subview in view.subviews where subview !== findBar { subview.removeFromSuperview() }
    mode = .message
  }

  /// The text column's width, also while something else is showing and the text view has no
  /// width of its own yet. Rendering sizes images and diagrams to it.
  var expectedColumnWidth: CGFloat {
    if view.bounds.width == 0 { view.window?.layoutIfNeeded() }
    return textView.columnWidth(forWidth: view.bounds.width)
  }

  // MARK: Modes

  private func install(_ content: NSView) {
    for subview in view.subviews where subview !== content && subview !== findBar {
      subview.removeFromSuperview()
    }
    if content.superview !== view {
      content.translatesAutoresizingMaskIntoConstraints = false
      view.addSubview(content, positioned: .below, relativeTo: findBar.superview === view ? findBar : nil)
      NSLayoutConstraint.activate([
        content.leadingAnchor.constraint(equalTo: view.leadingAnchor),
        content.trailingAnchor.constraint(equalTo: view.trailingAnchor),
        content.topAnchor.constraint(equalTo: view.topAnchor),
        content.bottomAnchor.constraint(equalTo: view.bottomAnchor),
      ])
    }
    if mode != .markdown, mode != .html { hideFindBar() }
  }

  func showWelcome() {
    mode = .welcome
    headings = []
    webView = nil
    let hosting = NSHostingView(
      rootView: WelcomeView { [weak self] url in
        guard let self else { return }
        self.delegate?.reader(self, open: url, inNewTab: false)
      })
    hostingView = hosting
    install(hosting)
  }

  func showMessage(title: String, message: String, symbol: String = "exclamationmark.triangle") {
    mode = .message
    headings = []
    webView = nil
    let hosting = NSHostingView(
      rootView: MessageView(title: title, message: message, symbol: symbol))
    hostingView = hosting
    install(hosting)
  }

  /// Shows a rendered document. `restoring` is a character index to bring back to the top.
  func showMarkdown(_ document: RenderedDocument, restoring location: Int? = nil) {
    let wasShowing = mode == .markdown
    mode = .markdown
    headings = document.headings
    webView = nil
    hostingView = nil
    install(scrollView)
    textView.updateInsets()
    lastColumnWidth = textView.currentColumnWidth
    textView.resetHover()

    suppressScrollTracking = true
    let selection = textView.selectedRange()
    textView.textStorage?.setAttributedString(document.text)
    // A stale selection makes the text view scroll to it when it gains focus.
    let length = document.text.length
    textView.setSelectedRange(
      wasShowing && NSMaxRange(selection) <= length ? selection : NSRange(location: 0, length: 0))
    if let location, location > 0 {
      scroll(toCharacter: location, animated: false, margin: 0)
    } else if !wasShowing || location == 0 {
      scrollView.contentView.scroll(to: NSPoint(x: 0, y: -scrollView.contentInsets.top))
      scrollView.reflectScrolledClipView(scrollView.contentView)
    }
    suppressScrollTracking = false
    lastHeading = -1
    didScroll()
    // A reload keeps the search going.
    if !findBar.isHidden { search(findBar.query, from: findMatches.indices.contains(findIndex) ? findMatches[findIndex].location : nil) }
  }

  /// Swaps in the images and diagrams of a fresh render of the same document, replacing only
  /// their placeholders, which is far cheaper than replacing the whole text. Returns false if the
  /// documents differ in more than that, in which case nothing useful was done.
  func applyAssets(_ document: RenderedDocument) -> Bool {
    guard mode == .markdown, let storage = textView.textStorage else { return false }
    let old = Self.assetSlots(in: storage)
    let new = Self.assetSlots(in: document.text)
    guard Set(old.keys) == Set(new.keys) else { return false }
    let top = topCharacterIndex
    var shift = 0
    storage.beginEditing()
    for (slot, range) in old.sorted(by: { $0.value.location > $1.value.location }) {
      let replacement = document.text.attributedSubstring(from: new[slot]!)
      if storage.attributedSubstring(from: range).isEqual(to: replacement) { continue }
      if range.location < top { shift += replacement.length - range.length }
      storage.replaceCharacters(in: range, with: replacement)
    }
    storage.endEditing()
    guard storage.string == document.text.string else { return false }
    headings = document.headings
    if top > 0, shift != 0 { scroll(toCharacter: top + shift, animated: false, margin: 0) }
    if !findBar.isHidden { search(findBar.query) }
    refreshHeadingTracking()
    return true
  }

  private static func assetSlots(in text: NSAttributedString) -> [Int: NSRange] {
    var slots: [Int: NSRange] = [:]
    text.enumerateAttribute(.mdowAssetSlot, in: NSRange(location: 0, length: text.length)) {
      value, range, _ in
      guard let slot = value as? Int else { return }
      slots[slot] = slots[slot].map { NSUnionRange($0, range) } ?? range
    }
    return slots
  }

  func showHTML(_ url: URL) {
    mode = .html
    headings = []
    hostingView = nil
    htmlURL = url
    let configuration = WKWebViewConfiguration()
    // Pages are shown, never run. Mdow's own scripts run in a separate world.
    configuration.defaultWebpagePreferences.allowsContentJavaScript = false
    configuration.websiteDataStore = .nonPersistent()
    let web = WKWebView(frame: view.bounds, configuration: configuration)
    web.navigationDelegate = self
    web.allowsMagnification = true
    web.pageZoom = Preferences.shared.zoom
    web.setAccessibilityLabel("Document")
    webView = web

    install(web)
    web.loadFileURL(url, allowingReadAccessTo: url.deletingLastPathComponent())
  }

  func reloadHTML() {
    guard mode == .html, let webView, let htmlURL else { return }
    webView.loadFileURL(htmlURL, allowingReadAccessTo: htmlURL.deletingLastPathComponent())
  }

  func applyZoomToHTML() {
    webView?.pageZoom = Preferences.shared.zoom
  }

  /// The window's appearance, which updates before its views' when the appearance changes.
  var isDark: Bool {
    (view.window?.effectiveAppearance ?? view.effectiveAppearance)
      .bestMatch(from: [.aqua, .darkAqua]) == .darkAqua
  }

  /// Pages that don't support dark mode are darkened in it, unless that's turned off.
  func updateHTMLAppearance() {
    guard let webView else { return }
    let on = isDark && Preferences.shared.darkenHTML
    // User scripts don't run with page JavaScript off, so this is evaluated directly.
    webView.evaluateJavaScript(
      HTMLDarkMode.script + "\nwindow.__mdowSetDark(\(on)) ?? false", in: nil, in: .defaultClient
    ) { [weak webView] result in
      // WebKit fills the overscroll area with the page's own colour, sampled before the page
      // was darkened; a fixed colour has to stand in. (A dynamic one blanks the page.)
      let darkened = (try? result.get()) as? Bool == true
      webView?.underPageBackgroundColor = darkened ? HTMLDarkMode.background : nil
    }
  }

  func webView(_ webView: WKWebView, didFinish navigation: WKNavigation!) {
    updateHTMLAppearance()
  }

  // MARK: Find

  private let findBar = FindBar()
  private var htmlURL: URL?
  private var findMatches: [NSRange] = []
  private var findIndex = -1

  /// Find menu commands, for Markdown and HTML alike.
  func performFind(_ action: NSTextFinder.Action) {
    guard mode == .markdown || mode == .html else { return }
    switch action {
    case .showFindInterface:
      showFindBar()
    case .nextMatch, .previousMatch:
      if findBar.isHidden { showFindBar() }
      step(backwards: action == .previousMatch)
    case .setSearchString:
      if mode == .markdown {
        let range = textView.selectedRange()
        guard range.length > 0, let storage = textView.textStorage else { return }
        findBar.query = (storage.string as NSString).substring(with: range)
      } else {
        webView?.evaluateJavaScript(
          "window.getSelection().toString()", in: nil, in: .defaultClient
        ) { [weak self] result in
          guard let self, case .success(let value) = result, let text = value as? String, !text.isEmpty
          else { return }
          self.findBar.query = text
        }
      }
    case .hideFindInterface:
      hideFindBar()
    default:
      break
    }
  }

  private func showFindBar() {
    if findBar.superview !== view {
      view.addSubview(findBar)
      NSLayoutConstraint.activate([
        findBar.topAnchor.constraint(equalTo: view.topAnchor, constant: 10),
        findBar.trailingAnchor.constraint(equalTo: view.trailingAnchor, constant: -12),
        findBar.widthAnchor.constraint(equalToConstant: 360),
      ])
    }
    let wasHidden = findBar.isHidden
    findBar.isHidden = false
    findBar.focus()
    if wasHidden, !findBar.query.isEmpty { search(findBar.query) }
  }

  private func hideFindBar() {
    guard !findBar.isHidden else { return }
    let hadFocus = (view.window?.firstResponder as? NSView)?.isDescendant(of: findBar) == true
    findBar.isHidden = true
    clearHighlights()
    findMatches = []
    findIndex = -1
    if hadFocus { view.window?.makeFirstResponder(mode == .html ? webView : textView) }
  }

  /// Finds every match in the document, starting at the reading position (or `location`).
  private func search(_ query: String, from location: Int? = nil) {
    PerfLog.measure("find “\(query)”") {
      runSearch(query, from: location)
    }
  }

  private func runSearch(_ query: String, from location: Int?) {
    if mode == .html {
      findInHTML(query, backwards: false)
      return
    }
    clearHighlights()
    findMatches = []
    findIndex = -1
    guard !query.isEmpty, let storage = textView.textStorage else {
      findBar.setResult(current: nil, total: nil)
      return
    }
    let string = storage.string as NSString
    var range = NSRange(location: 0, length: string.length)
    while findMatches.count < 5000 {
      let match = string.range(of: query, options: [.caseInsensitive, .diacriticInsensitive], range: range)
      guard match.location != NSNotFound, match.length > 0 else { break }
      findMatches.append(match)
      range = NSRange(location: NSMaxRange(match), length: string.length - NSMaxRange(match))
    }
    guard !findMatches.isEmpty else {
      findBar.setResult(current: nil, total: 0)
      return
    }
    let start = location ?? topCharacterIndex
    showMatch(findMatches.firstIndex { $0.location >= start } ?? 0)
  }

  private func step(backwards: Bool) {
    if mode == .html {
      findInHTML(findBar.query, backwards: backwards)
      return
    }
    guard !findMatches.isEmpty else { return }
    let count = findMatches.count
    showMatch(((findIndex + (backwards ? -1 : 1)) % count + count) % count)
  }

  private func showMatch(_ index: Int) {
    guard let layout = textView.layoutManager else { return }
    if findMatches.indices.contains(findIndex), highlighted.location != NSNotFound {
      layout.addTemporaryAttribute(
        .backgroundColor, value: ReaderColors.findMatch, forCharacterRange: findMatches[findIndex])
    }
    findIndex = index
    scroll(toCharacter: findMatches[index].location, animated: false, margin: scrollView.contentView.bounds.height * 0.3)
    highlightVisibleMatches(force: true)
    findBar.setResult(current: index + 1, total: findMatches.count)
  }

  /// The characters whose matches are highlighted. Only matches around the visible text are, so
  /// a search with thousands of matches stays instant; scrolling moves the window along.
  private var highlighted = NSRange(location: NSNotFound, length: 0)

  private func highlightVisibleMatches(force: Bool = false) {
    guard !findMatches.isEmpty, let layout = textView.layoutManager, let container = textView.textContainer,
      let storage = textView.textStorage, storage.length > 0
    else { return }
    var rect = textView.visibleRect
    rect.origin.x -= textView.textContainerOrigin.x
    rect.origin.y -= textView.textContainerOrigin.y
    let glyphs = layout.glyphRange(forBoundingRect: rect, in: container)
    let visible = layout.characterRange(forGlyphRange: glyphs, actualGlyphRange: nil)
    if !force, highlighted.location != NSNotFound, NSIntersectionRange(visible, highlighted) == visible {
      return
    }
    clearHighlights()
    let margin = max(visible.length, 2000)
    let start = max(0, visible.location - margin)
    highlighted = NSRange(location: start, length: min(storage.length, NSMaxRange(visible) + margin) - start)
    // Matches are in order, so the first one in the window is found by bisection.
    var low = 0
    var high = findMatches.count
    while low < high {
      let middle = (low + high) / 2
      if NSMaxRange(findMatches[middle]) <= highlighted.location { low = middle + 1 } else { high = middle }
    }
    var index = low
    while index < findMatches.count, findMatches[index].location < NSMaxRange(highlighted) {
      let color = index == findIndex ? ReaderColors.findCurrent : ReaderColors.findMatch
      layout.addTemporaryAttribute(.backgroundColor, value: color, forCharacterRange: findMatches[index])
      index += 1
    }
  }

  private func clearHighlights() {
    guard highlighted.location != NSNotFound, let layout = textView.layoutManager,
      let storage = textView.textStorage
    else { return }
    let range = NSIntersectionRange(highlighted, NSRange(location: 0, length: storage.length))
    layout.removeTemporaryAttribute(.backgroundColor, forCharacterRange: range)
    highlighted = NSRange(location: NSNotFound, length: 0)
  }

  private func findInHTML(_ query: String, backwards: Bool) {
    guard let webView, !query.isEmpty else {
      findBar.setResult(current: nil, total: nil)
      return
    }
    let configuration = WKFindConfiguration()
    configuration.backwards = backwards
    configuration.caseSensitive = false
    configuration.wraps = true
    webView.find(query, configuration: configuration) { [weak self] result in
      self?.findBar.setResult(current: nil, total: result.matchFound ? nil : 0)
    }
  }

  /// Something that can be printed: the document laid out for paper, light, and without the
  /// on-screen margins.
  func printableView(width: CGFloat) -> NSView? {
    switch mode {
    case .markdown:
      let view = ReaderTextView.make()
      view.minimumMargin = 0
      view.columnWidth = { $0 }
      view.appearance = NSAppearance(named: .aqua)
      view.backgroundColor = .white
      view.frame = NSRect(x: 0, y: 0, width: width, height: 100)
      view.textContainerInset = .zero
      view.textStorage?.setAttributedString(textView.attributedString())
      view.sizeToFit()
      return view
    case .html:
      return webView
    default:
      return nil
    }
  }

  // MARK: Scrolling

  /// The first character at the top of the visible text.
  var topCharacterIndex: Int {
    guard mode == .markdown, let layoutManager = textView.layoutManager,
      let container = textView.textContainer, let storage = textView.textStorage, storage.length > 0
    else { return 0 }
    let visible = scrollView.contentView.bounds
    if visible.minY <= -scrollView.contentInsets.top + 1 { return 0 }
    let y = visible.minY + scrollView.contentInsets.top - textView.textContainerOrigin.y + 2
    if y <= 0 { return 0 }
    // The glyph under a point; `glyphRange(forBoundingRect:)` would answer with the start of
    // whatever code block or table contains it.
    let glyph = layoutManager.glyphIndex(for: NSPoint(x: container.size.width / 2, y: y), in: container)
    return min(layoutManager.characterIndexForGlyph(at: glyph), storage.length - 1)
  }

  func scroll(toCharacter location: Int, animated: Bool, margin: CGFloat = 14) {
    guard let layoutManager = textView.layoutManager, let container = textView.textContainer,
      let storage = textView.textStorage, storage.length > 0
    else { return }
    let location = min(location, storage.length - 1)
    layoutManager.ensureLayout(forCharacterRange: NSRange(location: 0, length: location + 1))
    let glyphs = layoutManager.glyphRange(
      forCharacterRange: NSRange(location: location, length: 1), actualCharacterRange: nil)
    let rect = layoutManager.boundingRect(forGlyphRange: glyphs, in: container)
    let insets = scrollView.contentInsets
    let maxY = max(-insets.top, textView.frame.height - scrollView.contentView.bounds.height + insets.bottom)
    let y = min(max(rect.minY + textView.textContainerOrigin.y - insets.top - margin, -insets.top), maxY)
    let target = NSPoint(x: 0, y: y.rounded())
    if animated, !NSWorkspace.shared.accessibilityDisplayShouldReduceMotion {
      NSAnimationContext.runAnimationGroup { context in
        context.duration = 0.3
        context.timingFunction = CAMediaTimingFunction(name: .easeInEaseOut)
        scrollView.contentView.animator().setBoundsOrigin(target)
      } completionHandler: { [weak self] in
        MainActor.assumeIsolated {
          self?.scrollView.reflectScrolledClipView(self!.scrollView.contentView)
          self?.refreshHeadingTracking()
        }
      }
    } else {
      scrollView.contentView.scroll(to: target)
    }
    scrollView.reflectScrolledClipView(scrollView.contentView)
  }

  func scroll(toAnchor anchor: String) -> Bool {
    guard mode == .markdown, let storage = textView.textStorage else { return false }
    let wanted = anchor.removingPercentEncoding ?? anchor
    var found: Int?
    storage.enumerateAttribute(.mdowAnchor, in: NSRange(location: 0, length: storage.length)) {
      value, range, stop in
      if let value = value as? String, value == wanted || value == Slugger.slug(wanted) {
        found = range.location
        stop.pointee = true
      }
    }
    guard let found else { return false }
    scroll(toCharacter: found, animated: true)
    return true
  }

  func scroll(toHeading index: Int) {
    guard headings.indices.contains(index) else { return }
    scroll(toCharacter: headings[index].location, animated: true)
  }

  func scrollToTop() {
    switch mode {
    case .markdown: scroll(toCharacter: 0, animated: true, margin: 40)
    default: break
    }
  }

  @objc private func didScroll() {
    // Scrolling the reader decides whether it's held at the top; a resize moving the text
    // doesn't (see `keepAtTop`).
    if mode == .markdown {
      // The text view settles its own scroll position shortly after reflowing.
      if textView.frame.width == scrolledWidth, CACurrentMediaTime() - resizedAt > 0.3 {
        pinnedToTop = isAtTop
      } else {
        keepAtTop()
      }
    }
    textView.refreshHover()
    if !findMatches.isEmpty { highlightVisibleMatches() }
    guard !suppressScrollTracking, mode == .markdown else { return }
    let top = topCharacterIndex
    // A heading counts as current once it reaches the top quarter of the view.
    let lookahead = scrollView.contentView.bounds.height * 0.25
    var current: Int?
    if let layoutManager = textView.layoutManager, let container = textView.textContainer,
      let storage = textView.textStorage, storage.length > 0
    {
      let y = scrollView.contentView.bounds.minY + scrollView.contentInsets.top + lookahead
        - textView.textContainerOrigin.y
      let glyph = layoutManager.glyphIndex(
        for: NSPoint(x: container.size.width / 2, y: max(y, 0)), in: container)
      let probe = max(top, layoutManager.characterIndexForGlyph(at: glyph))
      current = headings.lastIndex { $0.location <= probe }
    }
    if current != lastHeading {
      lastHeading = current
      delegate?.reader(self, didScrollToHeading: current)
    }
  }

  func refreshHeadingTracking() {
    lastHeading = -1
    didScroll()
  }

  private var isAtTop: Bool {
    scrollView.contentView.bounds.minY <= -scrollView.contentInsets.top + 1
  }
  private var pinnedToTop = true
  private var scrolledWidth: CGFloat = 0
  private var resizedAt: CFTimeInterval = 0

  /// Width changes (a sidebar sliding open, a split) reflow the text and nudge it; a document
  /// read from the top stays at the top.
  private func keepAtTop() {
    if textView.frame.width != scrolledWidth { resizedAt = CACurrentMediaTime() }
    scrolledWidth = textView.frame.width
    guard mode == .markdown, pinnedToTop, !isAtTop else { return }
    scrollView.contentView.scroll(to: NSPoint(x: 0, y: -scrollView.contentInsets.top))
    scrollView.reflectScrolledClipView(scrollView.contentView)
  }
  @objc private func didResize() {
    keepAtTop()
    let width = textView.currentColumnWidth
    if mode == .markdown, abs(width - lastColumnWidth) > 1 {
      lastColumnWidth = width
      delegate?.readerColumnWidthChanged(self)
    }
  }

  // MARK: Links

  func textView(_ textView: NSTextView, clickedOnLink link: Any, at charIndex: Int) -> Bool {
    let url: URL? = (link as? URL) ?? (link as? String).flatMap(URL.init(string:))
    guard let url else { return false }
    let newTab = NSApp.currentEvent?.modifierFlags.contains(.command) == true
    delegate?.reader(self, open: url, inNewTab: newTab)
    return true
  }

  func webView(
    _ webView: WKWebView, decidePolicyFor navigationAction: WKNavigationAction,
    decisionHandler: @escaping @MainActor (WKNavigationActionPolicy) -> Void
  ) {
    guard navigationAction.navigationType == .linkActivated, let url = navigationAction.request.url else {
      decisionHandler(.allow)
      return
    }
    // Same-page anchors scroll in place; everything else goes through the app.
    if let current = webView.url, url.fragment != nil,
      url.absoluteString.hasPrefix(current.absoluteString.components(separatedBy: "#")[0])
    {
      decisionHandler(.allow)
      return
    }
    decisionHandler(.cancel)
    delegate?.reader(self, open: url, inNewTab: navigationAction.modifierFlags.contains(.command))
  }
}

/// Accepts dropped documents and folders anywhere in the window's content.
final class DropView: NSView {
  var onOpenFiles: (([URL]) -> Void)?

  override init(frame: NSRect) {
    super.init(frame: frame)
    registerForDraggedTypes([.fileURL])
  }

  required init?(coder: NSCoder) { fatalError() }

  private func urls(_ info: NSDraggingInfo) -> [URL] {
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
    urls(sender).isEmpty ? [] : .copy
  }

  override func performDragOperation(_ sender: NSDraggingInfo) -> Bool {
    let urls = urls(sender)
    guard !urls.isEmpty else { return false }
    onOpenFiles?(urls)
    return true
  }
}

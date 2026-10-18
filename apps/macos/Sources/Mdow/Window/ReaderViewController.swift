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
  private let scrollView = NSScrollView()

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
    scrollView.backgroundColor = .textBackgroundColor
    scrollView.documentView = textView
    scrollView.contentView.postsBoundsChangedNotifications = true
    scrollView.translatesAutoresizingMaskIntoConstraints = false
    scrollView.findBarPosition = .aboveContent
    textView.delegate = self
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
    showWelcome()
  }

  // MARK: Modes

  private func install(_ content: NSView) {
    for subview in view.subviews where subview !== content { subview.removeFromSuperview() }
    if content.superview !== view {
      content.translatesAutoresizingMaskIntoConstraints = false
      view.addSubview(content)
      NSLayoutConstraint.activate([
        content.leadingAnchor.constraint(equalTo: view.leadingAnchor),
        content.trailingAnchor.constraint(equalTo: view.trailingAnchor),
        content.topAnchor.constraint(equalTo: view.topAnchor),
        content.bottomAnchor.constraint(equalTo: view.bottomAnchor),
      ])
    }
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
      rootView: ContentUnavailableView(title, systemImage: symbol, description: Text(message))
        .frame(maxWidth: .infinity, maxHeight: .infinity))
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

    let container = NSView()
    web.translatesAutoresizingMaskIntoConstraints = false
    htmlFindBar.translatesAutoresizingMaskIntoConstraints = false
    htmlFindBar.isHidden = true
    htmlFindBar.onFind = { [weak self] query, backwards in self?.findInHTML(query, backwards: backwards) }
    htmlFindBar.onClose = { [weak self] in self?.hideHTMLFindBar() }
    container.addSubview(web)
    container.addSubview(htmlFindBar)
    let findTop = htmlFindBar.topAnchor.constraint(equalTo: container.safeAreaLayoutGuide.topAnchor)
    NSLayoutConstraint.activate([
      findTop,
      htmlFindBar.leadingAnchor.constraint(equalTo: container.leadingAnchor),
      htmlFindBar.trailingAnchor.constraint(equalTo: container.trailingAnchor),
      web.leadingAnchor.constraint(equalTo: container.leadingAnchor),
      web.trailingAnchor.constraint(equalTo: container.trailingAnchor),
      web.bottomAnchor.constraint(equalTo: container.bottomAnchor),
    ])
    htmlWebTop = web.topAnchor.constraint(equalTo: container.topAnchor)
    htmlWebTop?.isActive = true
    install(container)
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
      // WebKit fills the area under the toolbar with the page's own colour, sampled before the
      // page was darkened; a fixed colour has to stand in. (A dynamic one blanks the page.)
      let darkened = (try? result.get()) as? Bool == true
      webView?.underPageBackgroundColor = darkened ? HTMLDarkMode.background : nil
    }
  }

  func webView(_ webView: WKWebView, didFinish navigation: WKNavigation!) {
    updateHTMLAppearance()
  }

  // MARK: Find in HTML

  private let htmlFindBar = HTMLFindBar()
  private var htmlURL: URL?
  private var htmlWebTop: NSLayoutConstraint?

  /// Find for HTML pages, which `NSTextFinder` can't search.
  func performHTMLFind(_ action: NSTextFinder.Action) {
    guard mode == .html else { return }
    switch action {
    case .showFindInterface:
      showHTMLFindBar()
    case .nextMatch, .previousMatch:
      if htmlFindBar.isHidden { showHTMLFindBar() }
      findInHTML(htmlFindBar.query, backwards: action == .previousMatch)
    case .setSearchString:
      webView?.evaluateJavaScript(
        "window.getSelection().toString()", in: nil, in: .defaultClient
      ) { [weak self] result in
        guard let self, case .success(let value) = result, let text = value as? String, !text.isEmpty
        else { return }
        self.htmlFindBar.query = text
      }
    case .hideFindInterface:
      hideHTMLFindBar()
    default:
      break
    }
  }

  private func showHTMLFindBar() {
    htmlFindBar.isHidden = false
    if #available(macOS 26, *), let webView, let container = webView.superview {
      // Moving a web view out from under the toolbar blanks it, so it stays put and is told
      // what covers it instead.
      container.layoutSubtreeIfNeeded()
      webView.obscuredContentInsets = NSEdgeInsets(
        top: container.safeAreaInsets.top + htmlFindBar.fittingSize.height, left: 0, bottom: 0, right: 0)
    } else {
      htmlWebTop?.isActive = false
      htmlWebTop = webView?.topAnchor.constraint(equalTo: htmlFindBar.bottomAnchor)
      htmlWebTop?.isActive = true
    }
    htmlFindBar.focus()
  }

  private func hideHTMLFindBar() {
    htmlFindBar.isHidden = true
    if let webView, let container = webView.superview {
      if #available(macOS 26, *) {
        webView.obscuredContentInsets = NSEdgeInsets(
          top: container.safeAreaInsets.top, left: 0, bottom: 0, right: 0)
      } else {
        htmlWebTop?.isActive = false
        htmlWebTop = webView.topAnchor.constraint(equalTo: container.topAnchor)
        htmlWebTop?.isActive = true
      }
      view.window?.makeFirstResponder(webView)
    }
  }

  private func findInHTML(_ query: String, backwards: Bool) {
    guard let webView, !query.isEmpty else {
      htmlFindBar.notFound = false
      return
    }
    let configuration = WKFindConfiguration()
    configuration.backwards = backwards
    configuration.caseSensitive = false
    configuration.wraps = true
    webView.find(query, configuration: configuration) { [weak self] result in
      self?.htmlFindBar.notFound = !result.matchFound
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
    textView.refreshHover()
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
  private var wasAtTop = false

  // Narrowing the text view (e.g. opening a split) nudges it down by its inset; a document
  // read from the top stays at the top.
  override func viewWillLayout() {
    super.viewWillLayout()
    wasAtTop = mode == .markdown && isAtTop
  }

  override func viewDidLayout() {
    super.viewDidLayout()
    if wasAtTop, !isAtTop {
      scrollView.contentView.scroll(to: NSPoint(x: 0, y: -scrollView.contentInsets.top))
      scrollView.reflectScrolledClipView(scrollView.contentView)
    }
    wasAtTop = false
  }

  @objc private func didResize() {
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

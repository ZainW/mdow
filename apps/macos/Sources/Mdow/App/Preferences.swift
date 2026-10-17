import AppKit
import MdowKit
import Observation

enum AppearanceMode: String, CaseIterable, Identifiable {
  case system, light, dark

  var id: String { rawValue }

  var displayName: String {
    switch self {
    case .system: "Automatic"
    case .light: "Light"
    case .dark: "Dark"
    }
  }

  var appearance: NSAppearance? {
    switch self {
    case .system: nil
    case .light: NSAppearance(named: .aqua)
    case .dark: NSAppearance(named: .darkAqua)
    }
  }
}

enum SidebarMode: String, CaseIterable, Identifiable {
  case outline, files, recents

  var id: String { rawValue }

  var displayName: String {
    switch self {
    case .outline: "Outline"
    case .files: "Files"
    case .recents: "Recents"
    }
  }

  var symbol: String {
    switch self {
    case .outline: "list.bullet.indent"
    case .files: "folder"
    case .recents: "clock"
    }
  }
}

extension Notification.Name {
  /// Posted when anything that affects how documents render changes.
  static let readerStyleChanged = Notification.Name("MdowReaderStyleChanged")
  static let recentsChanged = Notification.Name("MdowRecentsChanged")
}

/// App settings, stored in user defaults.
@MainActor @Observable
final class Preferences {
  static let shared = Preferences()

  static let defaultTextSize: Double = 15
  static let zoomSteps: [Double] = [0.5, 0.67, 0.75, 0.8, 0.9, 1, 1.1, 1.25, 1.5, 1.75, 2, 2.5, 3]

  @ObservationIgnored private let defaults = UserDefaults.standard

  var appearance: AppearanceMode {
    didSet { store(appearance.rawValue, "appearance"); NSApp.appearance = appearance.appearance }
  }
  var bodyFont: BodyFont { didSet { store(bodyFont.rawValue, "bodyFont"); styleChanged() } }
  var codeFont: CodeFont { didSet { store(codeFont.rawValue, "codeFont"); styleChanged() } }
  var lineSpacing: LineSpacing {
    didSet { store(lineSpacing.rawValue, "lineSpacing"); styleChanged() }
  }
  var readingWidth: ReadingWidth {
    didSet { store(readingWidth.rawValue, "readingWidth"); styleChanged() }
  }
  var textSize: Double { didSet { store(textSize, "textSize"); styleChanged() } }
  var zoom: Double { didSet { store(zoom, "zoom"); styleChanged() } }
  var openInTabs: Bool { didSet { store(openInTabs, "openInTabs") } }
  var reloadOnChange: Bool { didSet { store(reloadOnChange, "reloadOnChange") } }
  var sidebarMode: SidebarMode { didSet { store(sidebarMode.rawValue, "sidebarMode") } }
  private(set) var recents: [URL] = []

  private init() {
    func value<T: RawRepresentable>(_ key: String, _ fallback: T) -> T where T.RawValue == String {
      guard let raw = UserDefaults.standard.string(forKey: key), let value = T(rawValue: raw) else {
        return fallback
      }
      return value
    }
    let defaults = UserDefaults.standard
    appearance = value("appearance", AppearanceMode.system)
    bodyFont = value("bodyFont", BodyFont.system)
    codeFont = value("codeFont", CodeFont.sfMono)
    lineSpacing = value("lineSpacing", LineSpacing.standard)
    readingWidth = value("readingWidth", ReadingWidth.standard)
    sidebarMode = value("sidebarMode", SidebarMode.outline)
    textSize = defaults.object(forKey: "textSize") as? Double ?? Self.defaultTextSize
    zoom = defaults.object(forKey: "zoom") as? Double ?? 1
    openInTabs = defaults.object(forKey: "openInTabs") as? Bool ?? true
    reloadOnChange = defaults.object(forKey: "reloadOnChange") as? Bool ?? true
    recents = (defaults.stringArray(forKey: "recents") ?? []).map { URL(fileURLWithPath: $0) }
  }

  private func store(_ value: Any, _ key: String) { defaults.set(value, forKey: key) }

  private func styleChanged() {
    NotificationCenter.default.post(name: .readerStyleChanged, object: nil)
  }

  /// The reader style at the current zoom, for a text column of the given width.
  func readerStyle(columnWidth: CGFloat) -> ReaderStyle {
    var style = ReaderStyle()
    style.bodyFont = bodyFont
    style.codeFont = codeFont
    style.lineSpacing = lineSpacing
    style.fontSize = (textSize * zoom).rounded()
    style.maxContentWidth = columnWidth
    return style
  }

  /// The text column's width; it grows with the text so lines keep their length when zoomed.
  func columnWidth(available: CGFloat) -> CGFloat {
    guard let points = readingWidth.points else { return available }
    return min(available, points * textSize * zoom / Self.defaultTextSize)
  }

  func resetReading() {
    bodyFont = .system
    codeFont = .sfMono
    lineSpacing = .standard
    readingWidth = .standard
    textSize = Self.defaultTextSize
    zoom = 1
  }

  // MARK: Zoom

  var canZoomIn: Bool { zoom < Self.zoomSteps.last! }
  var canZoomOut: Bool { zoom > Self.zoomSteps.first! }

  func zoomIn() { zoom = Self.zoomSteps.first { $0 > zoom + 0.001 } ?? zoom }
  func zoomOut() { zoom = Self.zoomSteps.last { $0 < zoom - 0.001 } ?? zoom }
  func resetZoom() { zoom = 1 }

  // MARK: Recents

  func noteRecent(_ url: URL) {
    let url = url.standardizedFileURL
    recents.removeAll { $0 == url }
    recents.insert(url, at: 0)
    if recents.count > 30 { recents.removeLast(recents.count - 30) }
    saveRecents()
    // Also shows up in the Dock menu and system-wide recent items.
    NSDocumentController.shared.noteNewRecentDocumentURL(url)
  }

  func removeRecent(_ url: URL) {
    recents.removeAll { $0 == url }
    saveRecents()
  }

  func clearRecents() {
    recents.removeAll()
    saveRecents()
    NSDocumentController.shared.clearRecentDocuments(nil)
  }

  private func saveRecents() {
    defaults.set(recents.map(\.path), forKey: "recents")
    NotificationCenter.default.post(name: .recentsChanged, object: nil)
  }
}

import Foundation

public enum AppTheme: String, CaseIterable, Identifiable, Sendable {
    case system
    case light
    case dark

    public var id: String { rawValue }

    public var title: String {
        switch self {
        case .system: "System"
        case .light: "Light"
        case .dark: "Dark"
        }
    }
}

public enum ContentFontPreset: String, CaseIterable, Identifiable, Sendable {
    case inter
    case charter
    case systemSans = "system-sans"
    case georgia

    public var id: String { rawValue }

    public var title: String {
        switch self {
        case .inter: "Inter"
        case .charter: "Charter"
        case .systemSans: "System"
        case .georgia: "Georgia"
        }
    }

    public var nativeFontName: String? {
        switch self {
        case .inter: "Inter"
        case .charter: "Charter"
        case .systemSans: nil
        case .georgia: "Georgia"
        }
    }
}

public enum CodeFontPreset: String, CaseIterable, Identifiable, Sendable {
    case geistMono = "geist-mono"
    case systemMono = "system-mono"
    case sfMono = "sf-mono"
    case jetbrainsMono = "jetbrains-mono"

    public var id: String { rawValue }

    public var title: String {
        switch self {
        case .geistMono: "Geist"
        case .systemMono: "System"
        case .sfMono: "SF Mono"
        case .jetbrainsMono: "JetBrains"
        }
    }

    public var nativeFontName: String? {
        switch self {
        case .geistMono: "Geist Mono"
        case .systemMono: nil
        case .sfMono: "SF Mono"
        case .jetbrainsMono: "JetBrains Mono"
        }
    }
}

public enum ReadingWidth: String, CaseIterable, Identifiable, Sendable {
    case standard
    case comfortable
    case wide

    public var id: String { rawValue }

    public var title: String {
        switch self {
        case .standard: "Standard"
        case .comfortable: "Comfortable"
        case .wide: "Wide"
        }
    }

    public var documentMaxWidth: Int {
        switch self {
        case .standard: 768
        case .comfortable: 896
        case .wide: 1088
        }
    }
}

public enum InterfaceScale: String, CaseIterable, Identifiable, Sendable {
    case compact
    case comfortable
    case large

    public var id: String { rawValue }

    public var title: String {
        switch self {
        case .compact: "Compact"
        case .comfortable: "Comfortable"
        case .large: "Large"
        }
    }

    public var controlFontSize: Int {
        switch self {
        case .compact: 12
        case .comfortable: 13
        case .large: 14
        }
    }

    public var secondaryFontSize: Int {
        switch self {
        case .compact: 11
        case .comfortable: 12
        case .large: 13
        }
    }

    public var sidebarWidth: Int {
        switch self {
        case .compact: 244
        case .comfortable: 264
        case .large: 280
        }
    }

    public var tabBarHeight: Int {
        switch self {
        case .compact: 36
        case .comfortable: 40
        case .large: 44
        }
    }

    public var tabHeight: Int {
        switch self {
        case .compact: 28
        case .comfortable: 32
        case .large: 36
        }
    }

    public var breadcrumbHeight: Int {
        switch self {
        case .compact: 28
        case .comfortable: 32
        case .large: 36
        }
    }
}

public struct ReaderChromePreferences: Equatable, Sendable {
    public let appTheme: AppTheme
    public let contentFont: ContentFontPreset
    public let codeFont: CodeFontPreset
    public let sidebarOpen: Bool
    public let zoomLevel: Int
    public let readingWidth: ReadingWidth
    public let interfaceScale: InterfaceScale

    public init(
        appTheme: AppTheme = .system,
        contentFont: ContentFontPreset = .inter,
        codeFont: CodeFontPreset = .geistMono,
        sidebarOpen: Bool = true,
        zoomLevel: Int = 100,
        readingWidth: ReadingWidth = .standard,
        interfaceScale: InterfaceScale = .compact
    ) {
        self.appTheme = appTheme
        self.contentFont = contentFont
        self.codeFont = codeFont
        self.sidebarOpen = sidebarOpen
        self.zoomLevel = Self.clampedZoomLevel(zoomLevel)
        self.readingWidth = readingWidth
        self.interfaceScale = interfaceScale
    }

    public var documentMaxWidth: Int {
        readingWidth.documentMaxWidth
    }

    public var controlFontSize: Int {
        interfaceScale.controlFontSize
    }

    public var sidebarWidth: Int {
        interfaceScale.sidebarWidth
    }

    public func toggledSidebar() -> ReaderChromePreferences {
        ReaderChromePreferences(
            appTheme: appTheme,
            contentFont: contentFont,
            codeFont: codeFont,
            sidebarOpen: !sidebarOpen,
            zoomLevel: zoomLevel,
            readingWidth: readingWidth,
            interfaceScale: interfaceScale
        )
    }

    public func zoomedIn() -> ReaderChromePreferences {
        ReaderChromePreferences(
            appTheme: appTheme,
            contentFont: contentFont,
            codeFont: codeFont,
            sidebarOpen: sidebarOpen,
            zoomLevel: zoomLevel + 10,
            readingWidth: readingWidth,
            interfaceScale: interfaceScale
        )
    }

    public func zoomedOut() -> ReaderChromePreferences {
        ReaderChromePreferences(
            appTheme: appTheme,
            contentFont: contentFont,
            codeFont: codeFont,
            sidebarOpen: sidebarOpen,
            zoomLevel: zoomLevel - 10,
            readingWidth: readingWidth,
            interfaceScale: interfaceScale
        )
    }

    public func resetZoom() -> ReaderChromePreferences {
        ReaderChromePreferences(
            appTheme: appTheme,
            contentFont: contentFont,
            codeFont: codeFont,
            sidebarOpen: sidebarOpen,
            zoomLevel: 100,
            readingWidth: readingWidth,
            interfaceScale: interfaceScale
        )
    }

    public func withReadingWidth(_ readingWidth: ReadingWidth) -> ReaderChromePreferences {
        ReaderChromePreferences(
            appTheme: appTheme,
            contentFont: contentFont,
            codeFont: codeFont,
            sidebarOpen: sidebarOpen,
            zoomLevel: zoomLevel,
            readingWidth: readingWidth,
            interfaceScale: interfaceScale
        )
    }

    public func withInterfaceScale(_ interfaceScale: InterfaceScale) -> ReaderChromePreferences {
        ReaderChromePreferences(
            appTheme: appTheme,
            contentFont: contentFont,
            codeFont: codeFont,
            sidebarOpen: sidebarOpen,
            zoomLevel: zoomLevel,
            readingWidth: readingWidth,
            interfaceScale: interfaceScale
        )
    }

    public func withAppTheme(_ appTheme: AppTheme) -> ReaderChromePreferences {
        ReaderChromePreferences(
            appTheme: appTheme,
            contentFont: contentFont,
            codeFont: codeFont,
            sidebarOpen: sidebarOpen,
            zoomLevel: zoomLevel,
            readingWidth: readingWidth,
            interfaceScale: interfaceScale
        )
    }

    public func withContentFont(_ contentFont: ContentFontPreset) -> ReaderChromePreferences {
        ReaderChromePreferences(
            appTheme: appTheme,
            contentFont: contentFont,
            codeFont: codeFont,
            sidebarOpen: sidebarOpen,
            zoomLevel: zoomLevel,
            readingWidth: readingWidth,
            interfaceScale: interfaceScale
        )
    }

    public func withCodeFont(_ codeFont: CodeFontPreset) -> ReaderChromePreferences {
        ReaderChromePreferences(
            appTheme: appTheme,
            contentFont: contentFont,
            codeFont: codeFont,
            sidebarOpen: sidebarOpen,
            zoomLevel: zoomLevel,
            readingWidth: readingWidth,
            interfaceScale: interfaceScale
        )
    }

    private static func clampedZoomLevel(_ zoomLevel: Int) -> Int {
        min(max(zoomLevel, 60), 200)
    }
}

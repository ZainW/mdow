import AppKit
import SwiftUI

enum MdowStyle {
    static let backgroundLight = NSColor(red: 0.992, green: 0.992, blue: 0.988, alpha: 1)
    static let backgroundDark = NSColor(red: 0.070, green: 0.070, blue: 0.075, alpha: 1)
    static let background = adaptiveColor(light: backgroundLight, dark: backgroundDark)
    static let chrome = adaptiveColor(
        light: NSColor(red: 0.965, green: 0.965, blue: 0.957, alpha: 1),
        dark: NSColor(red: 0.095, green: 0.095, blue: 0.105, alpha: 1)
    )
    static let sidebar = adaptiveColor(
        light: NSColor(red: 0.952, green: 0.952, blue: 0.944, alpha: 1),
        dark: NSColor(red: 0.088, green: 0.088, blue: 0.096, alpha: 1)
    )
    static let elevatedSurface = adaptiveColor(
        light: NSColor(red: 1.000, green: 1.000, blue: 0.996, alpha: 1),
        dark: NSColor(red: 0.122, green: 0.122, blue: 0.132, alpha: 1)
    )
    static let foreground = adaptiveColor(
        light: NSColor(red: 0.096, green: 0.096, blue: 0.110, alpha: 1),
        dark: NSColor(red: 0.948, green: 0.948, blue: 0.956, alpha: 1)
    )
    static let muted = adaptiveColor(
        light: NSColor(red: 0.928, green: 0.928, blue: 0.920, alpha: 1),
        dark: NSColor(red: 0.156, green: 0.156, blue: 0.168, alpha: 1)
    )
    static let mutedForeground = adaptiveColor(
        light: NSColor(red: 0.445, green: 0.445, blue: 0.485, alpha: 1),
        dark: NSColor(red: 0.640, green: 0.640, blue: 0.690, alpha: 1)
    )
    static let border = adaptiveColor(
        light: NSColor(red: 0.870, green: 0.870, blue: 0.890, alpha: 1),
        dark: NSColor(red: 0.245, green: 0.245, blue: 0.275, alpha: 1)
    )
    static let borderSubtle = adaptiveColor(
        light: NSColor(red: 0.915, green: 0.915, blue: 0.925, alpha: 1),
        dark: NSColor(red: 0.185, green: 0.185, blue: 0.205, alpha: 1)
    )
    static let sidebarAccent = adaptiveColor(
        light: NSColor(red: 0.890, green: 0.915, blue: 0.980, alpha: 1),
        dark: NSColor(red: 0.145, green: 0.165, blue: 0.225, alpha: 1)
    )
    static let sidebarHover = adaptiveColor(
        light: NSColor(red: 0.925, green: 0.925, blue: 0.918, alpha: 1),
        dark: NSColor(red: 0.130, green: 0.130, blue: 0.145, alpha: 1)
    )
    static let primary = adaptiveColor(
        light: NSColor(red: 0.145, green: 0.355, blue: 0.820, alpha: 1),
        dark: NSColor(red: 0.420, green: 0.640, blue: 1.000, alpha: 1)
    )
    static let accent = adaptiveColor(
        light: NSColor(red: 0.820, green: 0.390, blue: 0.070, alpha: 1),
        dark: NSColor(red: 0.980, green: 0.670, blue: 0.260, alpha: 1)
    )
    static let searchHighlight = adaptiveColor(
        light: NSColor(red: 1.000, green: 0.870, blue: 0.360, alpha: 0.85),
        dark: NSColor(red: 0.700, green: 0.520, blue: 0.120, alpha: 0.85)
    )
    static let searchHighlightForeground = adaptiveColor(
        light: NSColor(red: 0.140, green: 0.105, blue: 0.030, alpha: 1),
        dark: NSColor(red: 1.000, green: 0.960, blue: 0.820, alpha: 1)
    )
    static let searchCurrentHighlight = adaptiveColor(
        light: NSColor(red: 0.980, green: 0.510, blue: 0.120, alpha: 0.96),
        dark: NSColor(red: 1.000, green: 0.670, blue: 0.220, alpha: 0.96)
    )
    static let searchCurrentHighlightForeground = adaptiveColor(
        light: NSColor(red: 0.105, green: 0.060, blue: 0.015, alpha: 1),
        dark: NSColor(red: 0.120, green: 0.075, blue: 0.015, alpha: 1)
    )

    static func adaptiveColor(light: NSColor, dark: NSColor) -> Color {
        Color(nsColor: NSColor(name: nil) { appearance in
            let best = appearance.bestMatch(from: [.darkAqua, .aqua])
            return best == .darkAqua ? dark : light
        })
    }
}

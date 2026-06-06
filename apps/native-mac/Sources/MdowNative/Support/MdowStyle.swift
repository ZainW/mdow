import AppKit
import SwiftUI

enum MdowStyle {
    static let backgroundLight = NSColor(red: 0.985, green: 0.982, blue: 0.972, alpha: 1)
    static let backgroundDark = NSColor(red: 0.140, green: 0.140, blue: 0.140, alpha: 1)
    static let background = adaptiveColor(light: backgroundLight, dark: backgroundDark)
    static let chrome = adaptiveColor(
        light: NSColor(red: 0.972, green: 0.969, blue: 0.957, alpha: 1),
        dark: NSColor(red: 0.165, green: 0.165, blue: 0.165, alpha: 1)
    )
    static let sidebar = adaptiveColor(
        light: NSColor(red: 0.960, green: 0.957, blue: 0.945, alpha: 1),
        dark: NSColor(red: 0.155, green: 0.155, blue: 0.155, alpha: 1)
    )
    static let elevatedSurface = adaptiveColor(
        light: NSColor(red: 0.998, green: 0.996, blue: 0.988, alpha: 1),
        dark: NSColor(red: 0.190, green: 0.190, blue: 0.190, alpha: 1)
    )
    static let foreground = adaptiveColor(
        light: NSColor(red: 0.145, green: 0.120, blue: 0.095, alpha: 1),
        dark: NSColor(red: 0.920, green: 0.920, blue: 0.920, alpha: 1)
    )
    static let muted = adaptiveColor(
        light: NSColor(red: 0.940, green: 0.936, blue: 0.920, alpha: 1),
        dark: NSColor(red: 0.200, green: 0.200, blue: 0.200, alpha: 1)
    )
    static let mutedForeground = adaptiveColor(
        light: NSColor(red: 0.440, green: 0.390, blue: 0.330, alpha: 1),
        dark: NSColor(red: 0.650, green: 0.650, blue: 0.650, alpha: 1)
    )
    static let border = adaptiveColor(
        light: NSColor(red: 0.875, green: 0.855, blue: 0.810, alpha: 1),
        dark: NSColor(red: 0.270, green: 0.270, blue: 0.270, alpha: 1)
    )
    static let borderSubtle = adaptiveColor(
        light: NSColor(red: 0.905, green: 0.895, blue: 0.870, alpha: 1),
        dark: NSColor(red: 0.220, green: 0.220, blue: 0.220, alpha: 1)
    )
    static let sidebarAccent = adaptiveColor(
        light: NSColor(red: 0.925, green: 0.918, blue: 0.895, alpha: 1),
        dark: NSColor(red: 0.225, green: 0.225, blue: 0.225, alpha: 1)
    )
    static let sidebarHover = adaptiveColor(
        light: NSColor(red: 0.935, green: 0.930, blue: 0.912, alpha: 1),
        dark: NSColor(red: 0.205, green: 0.205, blue: 0.205, alpha: 1)
    )
    static let primary = adaptiveColor(
        light: NSColor(red: 0.320, green: 0.400, blue: 0.780, alpha: 1),
        dark: NSColor(red: 0.560, green: 0.630, blue: 0.960, alpha: 1)
    )
    static let accent = adaptiveColor(
        light: NSColor(red: 0.760, green: 0.470, blue: 0.170, alpha: 1),
        dark: NSColor(red: 0.880, green: 0.720, blue: 0.330, alpha: 1)
    )
    static let searchHighlight = adaptiveColor(
        light: NSColor(red: 1.000, green: 0.870, blue: 0.360, alpha: 0.85),
        dark: NSColor(red: 0.700, green: 0.520, blue: 0.120, alpha: 0.85)
    )
    static let searchHighlightForeground = adaptiveColor(
        light: NSColor(red: 0.140, green: 0.105, blue: 0.030, alpha: 1),
        dark: NSColor(red: 1.000, green: 0.960, blue: 0.820, alpha: 1)
    )

    static func adaptiveColor(light: NSColor, dark: NSColor) -> Color {
        Color(nsColor: NSColor(name: nil) { appearance in
            let best = appearance.bestMatch(from: [.darkAqua, .aqua])
            return best == .darkAqua ? dark : light
        })
    }
}

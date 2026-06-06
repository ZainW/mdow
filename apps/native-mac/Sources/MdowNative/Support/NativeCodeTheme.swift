import AppKit
import SwiftUI

enum NativeCodeTheme {
    static let surface = color(
        light: NSColor(red: 0.982, green: 0.982, blue: 0.982, alpha: 1),
        dark: NSColor(red: 0.118, green: 0.118, blue: 0.118, alpha: 1)
    )
    static let border = color(
        light: NSColor(red: 0.870, green: 0.870, blue: 0.870, alpha: 1),
        dark: NSColor(red: 0.205, green: 0.205, blue: 0.205, alpha: 1)
    )
    static let foreground = nsColor(
        light: NSColor(red: 0.118, green: 0.118, blue: 0.118, alpha: 1),
        dark: NSColor(red: 0.831, green: 0.831, blue: 0.831, alpha: 1)
    )
    static let keyword = nsColor(
        light: NSColor(red: 0.000, green: 0.000, blue: 0.800, alpha: 1),
        dark: NSColor(red: 0.337, green: 0.612, blue: 0.839, alpha: 1)
    )
    static let string = nsColor(
        light: NSColor(red: 0.639, green: 0.082, blue: 0.082, alpha: 1),
        dark: NSColor(red: 0.808, green: 0.569, blue: 0.471, alpha: 1)
    )
    static let comment = nsColor(
        light: NSColor(red: 0.000, green: 0.502, blue: 0.000, alpha: 1),
        dark: NSColor(red: 0.416, green: 0.600, blue: 0.333, alpha: 1)
    )
    static let function = nsColor(
        light: NSColor(red: 0.475, green: 0.369, blue: 0.149, alpha: 1),
        dark: NSColor(red: 0.863, green: 0.804, blue: 0.467, alpha: 1)
    )
    static let type = nsColor(
        light: NSColor(red: 0.149, green: 0.498, blue: 0.600, alpha: 1),
        dark: NSColor(red: 0.306, green: 0.788, blue: 0.690, alpha: 1)
    )
    static let number = nsColor(
        light: NSColor(red: 0.035, green: 0.525, blue: 0.408, alpha: 1),
        dark: NSColor(red: 0.710, green: 0.808, blue: 0.659, alpha: 1)
    )
    static let attribute = nsColor(
        light: NSColor(red: 0.502, green: 0.000, blue: 0.502, alpha: 1),
        dark: NSColor(red: 0.776, green: 0.471, blue: 0.867, alpha: 1)
    )

    private static func color(light: NSColor, dark: NSColor) -> Color {
        Color(nsColor: nsColor(light: light, dark: dark))
    }

    private static func nsColor(light: NSColor, dark: NSColor) -> NSColor {
        NSColor(name: nil) { appearance in
            appearance.bestMatch(from: [.darkAqua, .aqua]) == .darkAqua ? dark : light
        }
    }
}

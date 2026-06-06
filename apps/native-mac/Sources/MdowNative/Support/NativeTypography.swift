import MdowNativeCore
import AppKit
import SwiftUI

extension ContentFontPreset {
    func font(size: CGFloat, weight: Font.Weight = .regular) -> Font {
        guard let nativeFontName else {
            return .system(size: size, weight: weight)
        }

        return .custom(nativeFontName, size: size).weight(weight)
    }

    func nsFont(size: CGFloat, weight: NSFont.Weight = .regular) -> NSFont {
        guard let nativeFontName,
              let font = NSFont(name: nativeFontName, size: size) else {
            return .systemFont(ofSize: size, weight: weight)
        }

        return NSFontManager.shared.convert(font, toHaveTrait: [])
    }
}

extension CodeFontPreset {
    func font(size: CGFloat, weight: Font.Weight = .regular) -> Font {
        guard let nativeFontName else {
            return .system(size: size, weight: weight, design: .monospaced)
        }

        return .custom(nativeFontName, size: size).weight(weight)
    }

    func nsFont(size: CGFloat, weight: NSFont.Weight = .regular) -> NSFont {
        guard let nativeFontName,
              let font = NSFont(name: nativeFontName, size: size) else {
            return .monospacedSystemFont(ofSize: size, weight: weight)
        }

        return NSFontManager.shared.convert(font, toHaveTrait: [])
    }
}

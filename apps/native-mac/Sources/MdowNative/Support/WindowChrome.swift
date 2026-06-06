import AppKit
import SwiftUI

struct WindowChromeConfigurator: NSViewRepresentable {
    let title: String
    let representedURL: URL?

    func makeNSView(context: Context) -> NSView {
        let view = NSView(frame: .zero)
        DispatchQueue.main.async {
            configure(window: view.window)
        }
        return view
    }

    func updateNSView(_ nsView: NSView, context: Context) {
        DispatchQueue.main.async {
            configure(window: nsView.window)
        }
    }

    private func configure(window: NSWindow?) {
        guard let window else { return }
        window.title = title
        window.representedURL = representedURL
        window.titleVisibility = .hidden
        window.titlebarAppearsTransparent = true
        window.styleMask.insert(.fullSizeContentView)
        window.backgroundColor = resolvedBackgroundColor(for: window)
        window.isOpaque = true
        window.isMovableByWindowBackground = false
        window.minSize = NSSize(width: 720, height: 520)
    }

    private func resolvedBackgroundColor(for window: NSWindow) -> NSColor {
        let bestMatch = window.effectiveAppearance.bestMatch(from: [.aqua, .darkAqua])
        return bestMatch == .darkAqua ? MdowStyle.backgroundDark : MdowStyle.backgroundLight
    }
}

struct TitlebarDragRegion: NSViewRepresentable {
    func makeNSView(context: Context) -> NSView {
        DragView()
    }

    func updateNSView(_ nsView: NSView, context: Context) {}

    private final class DragView: NSView {
        override var mouseDownCanMoveWindow: Bool { true }
    }
}

import AppKit
import MdowKit
import SwiftUI

struct PaletteItem: Identifiable {
  enum Section: Int, CaseIterable {
    case headings, files, recents, commands

    var title: String {
      switch self {
      case .headings: "Headings"
      case .files: "Files"
      case .recents: "Recent"
      case .commands: "Commands"
      }
    }
  }

  let id = UUID()
  var section: Section
  var title: String
  var detail: String
  var symbol: String
  var action: @MainActor () -> Void
}

/// Quick Open (⌘K): a floating search field over the window, like Spotlight, for jumping to
/// headings, files and commands.
final class CommandPalette: NSPanel {
  private static var current: CommandPalette?

  static func present(for controller: DocumentWindowController, in window: NSWindow) {
    if let current, current.parent === window {
      current.close()
      return
    }
    current?.close()
    let palette = CommandPalette(items: controller.paletteItems())
    let size = NSSize(width: 640, height: 440)
    let frame = window.frame
    palette.setFrame(
      NSRect(
        x: frame.midX - size.width / 2, y: frame.maxY - size.height - min(120, frame.height * 0.14),
        width: size.width, height: size.height),
      display: false)
    window.addChildWindow(palette, ordered: .above)
    palette.makeKeyAndOrderFront(nil)
    current = palette
  }

  private init(items: [PaletteItem]) {
    super.init(
      contentRect: .zero, styleMask: [.borderless, .fullSizeContentView], backing: .buffered,
      defer: false)
    isFloatingPanel = true
    hidesOnDeactivate = true
    backgroundColor = .clear
    isOpaque = false
    hasShadow = true
    isMovableByWindowBackground = false
    animationBehavior = .utilityWindow
    let view = PaletteView(items: items) { [weak self] item in
      guard let self else { return }
      let parent = self.parent
      self.close()
      parent?.makeKeyAndOrderFront(nil)
      if let item { item.action() }
    }
    let hosting = NSHostingView(rootView: view)
    hosting.wantsLayer = true
    hosting.layer?.cornerRadius = 14
    hosting.layer?.cornerCurve = .continuous
    hosting.layer?.masksToBounds = true
    contentView = hosting
    setAccessibilityLabel("Quick Open")
  }

  override var canBecomeKey: Bool { true }

  override func resignKey() {
    super.resignKey()
    if isVisible { close() }
  }

  override func close() {
    parent?.removeChildWindow(self)
    super.close()
    if Self.current === self { Self.current = nil }
  }
}

private struct PaletteView: View {
  let items: [PaletteItem]
  var finish: (PaletteItem?) -> Void

  @State private var query = ""
  @State private var selected = 0
  @FocusState private var focused: Bool

  private var results: [(PaletteItem.Section, [PaletteItem])] {
    let query = query.trimmingCharacters(in: .whitespaces)
    if query.isEmpty {
      return PaletteItem.Section.allCases.compactMap { section in
        let limit = section == .commands ? 100 : 6
        let matches = Array(items.filter { $0.section == section }.prefix(limit))
        return matches.isEmpty ? nil : (section, matches)
      }
    }
    let scored = items.compactMap { item -> (PaletteItem, Int)? in
      let haystack = item.section == .files ? item.detail + "/" + item.title : item.title
      guard let score = FuzzyMatch.score(query, in: item.title) ?? FuzzyMatch.score(query, in: haystack)
      else { return nil }
      return (item, score + (item.section == .commands ? 5 : 0))
    }
    let best = scored.sorted { $0.1 > $1.1 }.prefix(60).map(\.0)
    return best.isEmpty ? [] : [(.commands, best)]
  }

  private var flat: [PaletteItem] { results.flatMap(\.1) }

  var body: some View {
    VStack(spacing: 0) {
      HStack(spacing: 12) {
        Image(systemName: "magnifyingglass")
          .font(.system(size: 15, weight: .medium))
          .foregroundStyle(Color.mdowTertiary)
        TextField(
          "", text: $query,
          prompt: Text("Search headings, files and commands").foregroundStyle(Color.mdowTertiary)
        )
        .textFieldStyle(.plain)
        .font(.mdowUI(16))
        .foregroundStyle(Color.mdowText)
        .focused($focused)
        .onSubmit { run() }
        .onKeyPress(.downArrow) { move(1) }
        .onKeyPress(.upArrow) { move(-1) }
        .onKeyPress(.escape) {
          finish(nil)
          return .handled
        }
      }
      .padding(.horizontal, 18)
      .frame(height: 54)
      Rectangle().fill(Color.mdowLine).frame(height: 1)
      if flat.isEmpty {
        Text("Nothing matches “\(query)”")
          .font(.mdowUI(13))
          .foregroundStyle(Color.mdowSecondary)
          .frame(maxWidth: .infinity, maxHeight: .infinity)
      } else {
        ScrollViewReader { proxy in
          ScrollView {
            LazyVStack(alignment: .leading, spacing: 0) {
              let showsHeaders = query.trimmingCharacters(in: .whitespaces).isEmpty
              ForEach(results, id: \.0) { section, items in
                if showsHeaders {
                  Text(section.title)
                    .font(.mdowUI(11.5, .medium))
                    .foregroundStyle(Color.mdowTertiary)
                    .padding(.horizontal, 10)
                    .padding(.top, 12)
                    .padding(.bottom, 4)
                }
                ForEach(items) { item in
                  let index = flat.firstIndex { $0.id == item.id } ?? 0
                  PaletteRow(item: item, selected: index == selected)
                    .id(item.id)
                    .onTapGesture { finish(item) }
                    .onHover { if $0 { selected = index } }
                }
              }
            }
            .padding(.horizontal, 8)
            .padding(.vertical, 6)
          }
          .onChange(of: selected) { _, index in
            guard flat.indices.contains(index) else { return }
            proxy.scrollTo(flat[index].id)
          }
        }
      }
      Rectangle().fill(Color.mdowLine).frame(height: 1)
      HStack(spacing: 16) {
        KeyHint(keys: "↑↓", label: "Move")
        KeyHint(keys: "↵", label: "Open")
        KeyHint(keys: "esc", label: "Close")
        Spacer()
      }
      .padding(.horizontal, 16)
      .frame(height: 34)
    }
    .frame(maxWidth: .infinity, maxHeight: .infinity)
    .background(Color.mdowElevated)
    .overlay(RoundedRectangle(cornerRadius: 14, style: .continuous).strokeBorder(Color.mdowLine))
    .onAppear { focused = true }
    .onChange(of: query) { selected = 0 }
  }

  private func move(_ step: Int) -> KeyPress.Result {
    let count = flat.count
    guard count > 0 else { return .handled }
    selected = (selected + step + count) % count
    return .handled
  }

  private func run() {
    let flat = flat
    guard flat.indices.contains(selected) else { return }
    finish(flat[selected])
  }
}

private struct PaletteRow: View {
  var item: PaletteItem
  var selected: Bool

  var body: some View {
    HStack(spacing: 10) {
      Image(systemName: item.symbol)
        .font(.system(size: 12.5))
        .frame(width: 18)
        .foregroundStyle(selected ? Color.mdowSecondary : Color.mdowTertiary)
      Text(item.title)
        .font(.mdowUI(13.5))
        .foregroundStyle(Color.mdowText)
        .lineLimit(1)
      if item.section != .commands, !item.detail.isEmpty {
        Text(item.detail)
          .font(.mdowMono(11))
          .foregroundStyle(Color.mdowTertiary)
          .lineLimit(1)
          .truncationMode(.head)
      }
      Spacer(minLength: 8)
      if item.section == .commands, !item.detail.isEmpty {
        Text(item.detail)
          .font(.mdowMono(11))
          .foregroundStyle(Color.mdowSecondary)
          .padding(.horizontal, 6)
          .frame(height: 20)
          .background(Color.mdowWell, in: RoundedRectangle(cornerRadius: 5, style: .continuous))
          .overlay(RoundedRectangle(cornerRadius: 5, style: .continuous).strokeBorder(Color.mdowLine))
      }
    }
    .padding(.horizontal, 10)
    .frame(height: 34)
    .background(
      selected ? Color.mdowSelected : .clear, in: RoundedRectangle(cornerRadius: 8, style: .continuous))
    .contentShape(.rect)
  }
}

/// A key and what it does, for the palette's footer.
private struct KeyHint: View {
  var keys: String
  var label: String

  var body: some View {
    HStack(spacing: 6) {
      Text(keys)
        .font(.mdowMono(10.5))
        .foregroundStyle(Color.mdowSecondary)
        .padding(.horizontal, 5)
        .frame(minWidth: 20, minHeight: 18)
        .background(Color.mdowWell, in: RoundedRectangle(cornerRadius: 4, style: .continuous))
        .overlay(RoundedRectangle(cornerRadius: 4, style: .continuous).strokeBorder(Color.mdowLine))
      Text(label)
        .font(.mdowUI(11.5))
        .foregroundStyle(Color.mdowTertiary)
    }
  }
}

import AppKit
import MdowKit
import SwiftUI

struct SettingsView: View {
  @Bindable private var preferences = Preferences.shared

  var body: some View {
    Form {
      Section {
        Picker("Appearance", selection: $preferences.appearance) {
          ForEach(AppearanceMode.allCases) { Text($0.displayName).tag($0) }
        }
        .pickerStyle(.segmented)
      }

      Section("Reading") {
        Picker("Font", selection: $preferences.bodyFont) {
          ForEach(BodyFont.allCases) { font in
            Text(font.displayName)
              .font(Font(font.font(size: NSFont.systemFontSize)))
              .tag(font)
          }
        }
        LabeledContent("Text size") {
          HStack {
            Slider(value: $preferences.textSize, in: 12...24, step: 1) {
              EmptyView()
            } minimumValueLabel: {
              Image(systemName: "textformat.size.smaller")
            } maximumValueLabel: {
              Image(systemName: "textformat.size.larger")
            }
            Text("\(Int(preferences.textSize)) pt")
              .monospacedDigit()
              .foregroundStyle(.secondary)
              .frame(width: 40, alignment: .trailing)
          }
        }
        Picker("Line spacing", selection: $preferences.lineSpacing) {
          ForEach(LineSpacing.allCases) { Text($0.displayName).tag($0) }
        }
        Picker("Text width", selection: $preferences.readingWidth) {
          ForEach(ReadingWidth.allCases) { Text($0.displayName).tag($0) }
        }
        Picker("Code font", selection: $preferences.codeFont) {
          ForEach(CodeFont.allCases) { font in
            Text(font.displayName)
              .font(Font(font.font(size: NSFont.systemFontSize)))
              .tag(font)
          }
        }
      }

      Section {
        PreviewText(preferences: preferences)
      } header: {
        Text("Preview")
      }

      Section("Documents") {
        Toggle("Open documents in tabs", isOn: $preferences.openInTabs)
        Toggle("Reload documents when they change on disk", isOn: $preferences.reloadOnChange)
        Toggle("Darken HTML pages in Dark Mode", isOn: $preferences.darkenHTML)
      }

      Section {
        HStack {
          Spacer()
          Button("Restore Defaults") { preferences.resetReading() }
        }
      }
    }
    .formStyle(.grouped)
    .frame(width: 480)
    .fixedSize(horizontal: false, vertical: true)
  }
}

private struct PreviewText: View {
  var preferences: Preferences

  var body: some View {
    let size = preferences.textSize
    VStack(alignment: .leading, spacing: size * 0.5) {
      Text("The quick brown fox")
        .font(Font(preferences.bodyFont.font(size: (size * 1.5).rounded(), weight: .bold)))
      Text(
        "Markdown is a lightweight markup language for creating formatted text. Mdow shows it the way it was meant to be read."
      )
      .font(Font(preferences.bodyFont.font(size: size)))
      .lineSpacing(size * (preferences.lineSpacing.multiple - 1))
      Text("let greeting = \"Hello, world\"")
        .font(Font(preferences.codeFont.font(size: (size * 0.86).rounded())))
        .padding(.horizontal, 10)
        .padding(.vertical, 6)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(.quaternary.opacity(0.6), in: .rect(cornerRadius: 6))
    }
    .padding(.vertical, 4)
  }
}

struct ShortcutsView: View {
  struct Shortcut: Identifiable {
    var title: String
    var keys: String
    var id: String { title }
  }

  struct Group: Identifiable {
    var title: String
    var items: [Shortcut]
    var id: String { title }
  }

  static let groups: [Group] = [
    Group(
      title: "Files",
      items: [
        .init(title: "Open file", keys: "⌘O"),
        .init(title: "Open folder", keys: "⇧⌘O"),
        .init(title: "Quick Open", keys: "⌘K"),
        .init(title: "New tab", keys: "⌘T"),
        .init(title: "Close tab", keys: "⌘W"),
        .init(title: "Reload", keys: "⌘R"),
        .init(title: "Show in Finder", keys: "⌥⌘R"),
        .init(title: "Print", keys: "⌘P"),
      ]),
    Group(
      title: "Reading",
      items: [
        .init(title: "Find", keys: "⌘F"),
        .init(title: "Find next / previous", keys: "⌘G  ⇧⌘G"),
        .init(title: "Back / forward", keys: "⌘[  ⌘]"),
        .init(title: "Previous / next heading", keys: "⌥⌘↑  ⌥⌘↓"),
        .init(title: "Top / bottom", keys: "⌘↑  ⌘↓"),
      ]),
    Group(
      title: "View",
      items: [
        .init(title: "Show or hide sidebar", keys: "⌃⌘S"),
        .init(title: "Outline, files, recents", keys: "⌃⌘1  ⌃⌘2  ⌃⌘3"),
        .init(title: "Zoom in / out", keys: "⌘+  ⌘−"),
        .init(title: "Actual size", keys: "⌘0"),
        .init(title: "Full screen", keys: "⌃⌘F"),
      ]),
    Group(
      title: "Tabs",
      items: [
        .init(title: "Next / previous tab", keys: "⌃⇥  ⌃⇧⇥"),
        .init(title: "Go to tab", keys: "⌘1 – ⌘9"),
        .init(title: "Show all tabs", keys: "⇧⌘\\"),
        .init(title: "Split view", keys: "⌘\\"),
      ]),
  ]

  var body: some View {
    Form {
      ForEach(Self.groups) { group in
        Section(group.title) {
          ForEach(group.items) { item in
            LabeledContent(item.title) {
              Text(item.keys)
                .font(.body.monospaced())
                .foregroundStyle(.secondary)
            }
          }
        }
      }
    }
    .formStyle(.grouped)
    .frame(width: 420)
    .fixedSize(horizontal: false, vertical: true)
  }
}

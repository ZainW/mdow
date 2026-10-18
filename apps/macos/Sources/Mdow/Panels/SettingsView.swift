import AppKit
import MdowKit
import SwiftUI

/// The Settings window: section tabs down the left, the chosen section as cards on the right.
struct SettingsView: View {
  enum Section: String, CaseIterable, Identifiable {
    case general, reading, companion

    var id: String { rawValue }

    var title: String {
      switch self {
      case .general: "General"
      case .reading: "Reading"
      case .companion: "Companion"
      }
    }

    var symbol: String {
      switch self {
      case .general: "gearshape"
      case .reading: "textformat"
      case .companion: "sparkle"
      }
    }
  }

  /// The content below the title bar, which the window keeps clear.
  static let size = CGSize(width: 760, height: 632)

  @AppStorage("settingsSection") private var section: Section = .general

  var body: some View {
    HStack(spacing: 0) {
      SectionTabs(selection: $section)
        .frame(width: 184)
      ScrollView {
        VStack(alignment: .leading, spacing: 0) {
          HStack(alignment: .center) {
            Text(section.title)
              .font(.mdowUI(20, .semibold))
              .foregroundStyle(Color.mdowText)
              .accessibilityAddTraits(.isHeader)
            Spacer()
            if section == .reading {
              Button("Restore Defaults") { Preferences.shared.resetReading() }
                .buttonStyle(GhostButtonStyle())
                .accessibilityHint("Resets the reading fonts, sizes and zoom")
            }
          }
          .frame(height: 28)
          .padding(.bottom, 16)
          switch section {
          case .general: GeneralSettings()
          case .reading: ReadingSettings()
          case .companion: CompanionSettingsSection()
          }
        }
        .padding(.leading, 12)
        .padding(.trailing, 32)
        .padding(.top, 16)
        .padding(.bottom, 28)
        .frame(maxWidth: .infinity, alignment: .leading)
      }
      .scrollIndicators(.automatic)
    }
    .frame(width: Self.size.width, height: Self.size.height)
    .background(Color.mdowChrome)
  }
}

/// The left column: one row per section, styled like the sidebar's rows.
private struct SectionTabs: View {
  @Binding var selection: SettingsView.Section

  var body: some View {
    VStack(alignment: .leading, spacing: 2) {
      ForEach(SettingsView.Section.allCases) { section in
        Button {
          selection = section
        } label: {
          Label {
            Text(section.title)
          } icon: {
            Image(systemName: section.symbol)
              .font(.system(size: 12.5, weight: .medium))
              .frame(width: 16)
          }
        }
        .buttonStyle(SectionTabStyle(selected: section == selection))
        .accessibilityAddTraits(section == selection ? [.isSelected] : [])
      }
      Spacer()
      Text(Self.version)
        .font(.mdowMono(11))
        .foregroundStyle(Color.mdowTertiary)
        .padding(.leading, 10)
        .textSelection(.enabled)
    }
    .padding(.horizontal, 10)
    .padding(.top, 14)
    .padding(.bottom, 16)
    .accessibilityElement(children: .contain)
    .accessibilityLabel("Settings sections")
    .onKeyPress(.upArrow) { move(-1) }
    .onKeyPress(.downArrow) { move(1) }
  }

  private func move(_ offset: Int) -> KeyPress.Result {
    let all = SettingsView.Section.allCases
    guard let index = all.firstIndex(of: selection) else { return .ignored }
    selection = all[(index + offset + all.count) % all.count]
    return .handled
  }

  private static let version: String = {
    let info = Bundle.main.infoDictionary ?? [:]
    let version = info["CFBundleShortVersionString"] as? String ?? "dev"
    return "Mdow \(version)"
  }()
}

private struct SectionTabStyle: ButtonStyle {
  var selected: Bool

  func makeBody(configuration: Configuration) -> some View {
    Tab(configuration: configuration, selected: selected)
  }

  private struct Tab: View {
    var configuration: Configuration
    var selected: Bool
    @State private var hovering = false

    var body: some View {
      configuration.label
        .labelStyle(TabLabelStyle())
        .font(.mdowUI(13, .medium))
        .foregroundStyle(selected || hovering ? Color.mdowText : Color.mdowSecondary)
        .padding(.horizontal, 10)
        .frame(maxWidth: .infinity, minHeight: 30, alignment: .leading)
        .background(
          fill(pressed: configuration.isPressed), in: .rect(cornerRadius: 7, style: .continuous))
        .contentShape(.rect)
        .onHover { hovering = $0 }
        .mdowFade(hovering)
        .mdowFade(selected)
    }

    private func fill(pressed: Bool) -> Color {
      if selected || pressed { return .mdowSelected }
      return hovering ? .mdowHover : .clear
    }
  }

  private struct TabLabelStyle: LabelStyle {
    func makeBody(configuration: Configuration) -> some View {
      HStack(spacing: 9) {
        configuration.icon
        configuration.title
      }
    }
  }
}

// MARK: General

private struct GeneralSettings: View {
  @Bindable private var preferences = Preferences.shared

  var body: some View {
    VStack(alignment: .leading, spacing: 20) {
      VStack(alignment: .leading, spacing: 0) {
        CardHeading("Appearance")
        PanelCard {
          SettingRow(title: "Theme", detail: "Automatic follows your Mac.") {
            PillPicker(
              label: "Theme", selection: $preferences.appearance, options: AppearanceMode.allCases,
              title: \.displayName)
          }
        }
      }

      VStack(alignment: .leading, spacing: 0) {
        CardHeading("Documents")
        PanelCard {
          SettingRow(
            title: "Open documents in tabs",
            detail: "New files join the front window instead of opening their own."
          ) {
            Toggle("Open documents in tabs", isOn: $preferences.openInTabs)
          }
          CardDivider()
          SettingRow(
            title: "Reload when files change",
            detail: "Picks up edits made in other apps as soon as they’re saved."
          ) {
            Toggle("Reload documents when they change on disk", isOn: $preferences.reloadOnChange)
          }
          CardDivider()
          SettingRow(
            title: "Darken HTML pages",
            detail: "Shows HTML files with a dark background in Dark Mode."
          ) {
            Toggle("Darken HTML pages in Dark Mode", isOn: $preferences.darkenHTML)
          }
        }
        .toggleStyle(MdowSwitchStyle())
      }

      if Updater.shared.isAvailable {
        VStack(alignment: .leading, spacing: 0) {
          CardHeading("Updates")
          PanelCard {
            SettingRow(
              title: "Software update", detail: "Mdow checks for new versions on its own."
            ) {
              Button("Check for Updates…") { Updater.shared.checkForUpdates(nil) }
                .buttonStyle(GhostButtonStyle())
            }
          }
        }
      }
    }
  }
}

// MARK: Reading

private struct ReadingSettings: View {
  @Bindable private var preferences = Preferences.shared

  var body: some View {
    VStack(alignment: .leading, spacing: 20) {
      VStack(alignment: .leading, spacing: 0) {
        CardHeading("Preview")
        PreviewText(preferences: preferences)
      }

      VStack(alignment: .leading, spacing: 0) {
        CardHeading("Type")
        PanelCard {
          SettingRow(title: "Font", detail: "Used for the document’s text.") {
            MenuPill(
              label: "Font", value: preferences.bodyFont.displayName,
              valueFont: Font(preferences.bodyFont.font(size: 13, weight: .medium))
            ) {
              BodyFont.allCases.map { font in
                MenuChoice(
                  title: font.displayName, font: font.font(size: 13),
                  isSelected: font == preferences.bodyFont
                ) { preferences.bodyFont = font }
              }
            }
          }
          CardDivider()
          SettingRow(title: "Code font", detail: "Used for code blocks and inline code.") {
            MenuPill(
              label: "Code font", value: preferences.codeFont.displayName,
              valueFont: Font(preferences.codeFont.font(size: 12, weight: .medium))
            ) {
              CodeFont.allCases.map { font in
                MenuChoice(
                  title: font.displayName, font: font.font(size: 12.5),
                  isSelected: font == preferences.codeFont
                ) { preferences.codeFont = font }
              }
            }
          }
          CardDivider()
          SettingRow(title: "Text size") {
            HStack(spacing: 10) {
              Image(systemName: "textformat.size.smaller")
                .font(.system(size: 11, weight: .medium))
                .foregroundStyle(Color.mdowTertiary)
                .accessibilityHidden(true)
              MdowSlider(label: "Text size", value: $preferences.textSize, range: 12...24, step: 1)
                .frame(width: 150)
              Image(systemName: "textformat.size.larger")
                .font(.system(size: 13, weight: .medium))
                .foregroundStyle(Color.mdowTertiary)
                .accessibilityHidden(true)
              Text("\(Int(preferences.textSize)) pt")
                .font(.mdowMono(11.5))
                .monospacedDigit()
                .foregroundStyle(Color.mdowSecondary)
                .frame(width: 40, alignment: .trailing)
                .accessibilityHidden(true)
            }
          }
        }
      }

      VStack(alignment: .leading, spacing: 0) {
        CardHeading("Layout")
        PanelCard {
          SettingRow(title: "Line spacing") {
            PillPicker(
              label: "Line spacing", selection: $preferences.lineSpacing,
              options: LineSpacing.allCases, title: \.displayName)
          }
          CardDivider()
          SettingRow(title: "Text width", detail: "Long lines are harder to follow.") {
            PillPicker(
              label: "Text width", selection: $preferences.readingWidth,
              options: ReadingWidth.allCases, title: { $0 == .full ? "Full" : $0.displayName })
          }
        }
      }
    }
  }
}

/// A short sample in the chosen reading fonts, sitting on paper like a document does.
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
      .fixedSize(horizontal: false, vertical: true)
      Text("let greeting = \"Hello, world\"")
        .font(Font(preferences.codeFont.font(size: (size * 0.86).rounded())))
        .padding(.horizontal, 12)
        .padding(.vertical, 8)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Color.mdowWell, in: .rect(cornerRadius: 7, style: .continuous))
        .hairline(7)
    }
    .foregroundStyle(Color.mdowText)
    .padding(20)
    .frame(maxWidth: .infinity, alignment: .leading)
    .background(Color.mdowPaper, in: .rect(cornerRadius: 10, style: .continuous))
    .hairline(10)
    .accessibilityElement(children: .combine)
    .accessibilityLabel("Preview of the reading fonts")
  }
}

// MARK: Shortcuts

/// The Keyboard Shortcuts window: every shortcut, grouped, with keys drawn as chips.
struct ShortcutsView: View {
  struct Shortcut: Identifiable {
    var title: String
    /// Each entry is one key combination; `joiner` goes between them.
    var keys: [String]
    var joiner = "/"
    var id: String { title }

    init(_ title: String, _ keys: String..., joiner: String = "/") {
      self.title = title
      self.keys = keys
      self.joiner = joiner
    }
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
        .init("Open file", "⌘O"),
        .init("Open folder", "⇧⌘O"),
        .init("Quick Open", "⌘K"),
        .init("New tab", "⌘T"),
        .init("Close tab", "⌘W"),
        .init("Reload", "⌘R"),
        .init("Show in Finder", "⌥⌘R"),
        .init("Print", "⌘P"),
      ]),
    Group(
      title: "Reading",
      items: [
        .init("Find", "⌘F"),
        .init("Find next / previous", "⌘G", "⇧⌘G"),
        .init("Back / forward", "⌘[", "⌘]"),
        .init("Previous / next heading", "⌥⌘↑", "⌥⌘↓"),
        .init("Top / bottom", "⌘↑", "⌘↓"),
      ]),
    Group(
      title: "View",
      items: [
        .init("Show or hide sidebar", "⌃⌘S"),
        .init("Show or hide companion", "⌥⌘0"),
        .init("Outline, files, recents", "⌃⌘1", "⌃⌘2", "⌃⌘3", joiner: ""),
        .init("Zoom in / out", "⌘+", "⌘−"),
        .init("Actual size", "⌘0"),
        .init("Full screen", "⌃⌘F"),
      ]),
    Group(
      title: "Tabs",
      items: [
        .init("Next / previous tab", "⇧⌘]", "⇧⌘["),
        .init("Go to tab", "⌘1", "⌘9", joiner: "–"),
        .init("Split view", "⌘\\"),
      ]),
  ]

  /// The groups split into two columns of roughly equal height.
  private static let columns: [[Group]] = [Array(groups.prefix(2)), Array(groups.dropFirst(2))]

  var body: some View {
    VStack(alignment: .leading, spacing: 0) {
      Text("Keyboard Shortcuts")
        .font(.mdowUI(20, .semibold))
        .foregroundStyle(Color.mdowText)
        .accessibilityAddTraits(.isHeader)
      Text("Most of these are in the menu bar too.")
        .font(.mdowUI(12.5))
        .foregroundStyle(Color.mdowSecondary)
        .padding(.top, 3)
        .padding(.bottom, 20)
      HStack(alignment: .top, spacing: 16) {
        ForEach(Self.columns.indices, id: \.self) { index in
          VStack(alignment: .leading, spacing: 20) {
            ForEach(Self.columns[index]) { ShortcutGroup(group: $0) }
          }
          .frame(width: 330)
        }
      }
    }
    .padding(.horizontal, 24)
    .padding(.top, 14)
    .padding(.bottom, 24)
    .fixedSize()
    .background(Color.mdowChrome)
  }
}

private struct ShortcutGroup: View {
  var group: ShortcutsView.Group

  var body: some View {
    VStack(alignment: .leading, spacing: 0) {
      CardHeading(group.title)
      PanelCard {
        ForEach(Array(group.items.enumerated()), id: \.element.id) { index, item in
          if index > 0 { CardDivider() }
          HStack(spacing: 12) {
            Text(item.title)
              .font(.mdowUI(13))
              .foregroundStyle(Color.mdowText)
              .lineLimit(1)
            Spacer(minLength: 8)
            HStack(spacing: 5) {
              ForEach(Array(item.keys.enumerated()), id: \.offset) { keyIndex, keys in
                if keyIndex > 0, !item.joiner.isEmpty {
                  Text(item.joiner)
                    .font(.mdowUI(11.5))
                    .foregroundStyle(Color.mdowTertiary)
                }
                KeyChip(keys)
              }
            }
          }
          .padding(.horizontal, 14)
          .frame(height: 38)
          .accessibilityElement(children: .ignore)
          .accessibilityLabel(item.title)
          .accessibilityValue(item.keys.joined(separator: item.joiner == "–" ? " to " : ", "))
        }
      }
    }
  }
}

import AppKit
import MdowKit
import SwiftUI

// Flat controls for the SwiftUI panels (Settings, Keyboard Shortcuts), drawn in Mdow's own
// palette rather than the system's bezels.

extension Color {
  static let mdowStrongLine = Color(nsColor: Palette.strongLine)
}

extension Animation {
  /// The one animation the panels use: a short ease-out for hover and selection fades.
  static let mdowFade = Animation.easeOut(duration: 0.12)
}

/// The panels' fade, or none when the user has asked for less motion.
private struct FadeModifier<Value: Equatable>: ViewModifier {
  @Environment(\.accessibilityReduceMotion) private var reduceMotion
  var value: Value

  func body(content: Content) -> some View {
    content.animation(reduceMotion ? nil : .mdowFade, value: value)
  }
}

extension View {
  func mdowFade<Value: Equatable>(_ value: Value) -> some View {
    modifier(FadeModifier(value: value))
  }

  /// A one-pixel border in the hairline colour.
  func hairline(_ radius: CGFloat, color: Color = .mdowLine) -> some View {
    overlay(
      RoundedRectangle(cornerRadius: radius, style: .continuous).strokeBorder(color, lineWidth: 1))
  }
}

// MARK: Surfaces

/// A rounded paper card with a hairline border; rows go inside, split by `CardDivider`.
struct PanelCard<Content: View>: View {
  @ViewBuilder var content: Content

  var body: some View {
    VStack(spacing: 0) { content }
      .background(Color.mdowPaper, in: .rect(cornerRadius: 10, style: .continuous))
      .hairline(10)
  }
}

struct CardDivider: View {
  @Environment(\.displayScale) private var scale

  var body: some View {
    Rectangle().fill(Color.mdowLine).frame(height: 1 / scale).padding(.leading, 16)
  }
}

/// The small muted label above a card.
struct CardHeading: View {
  var title: String

  init(_ title: String) { self.title = title }

  var body: some View {
    Text(title)
      .font(.mdowUI(11.5, .medium))
      .foregroundStyle(Color.mdowTertiary)
      .padding(.leading, 4)
      .padding(.bottom, 6)
      .accessibilityAddTraits(.isHeader)
  }
}

/// A setting: its name and a muted description on the left, the control on the right.
struct SettingRow<Control: View>: View {
  var title: String
  var detail: String?
  @ViewBuilder var control: Control

  var body: some View {
    HStack(alignment: .center, spacing: 16) {
      VStack(alignment: .leading, spacing: 2) {
        Text(title)
          .font(.mdowUI(13, .medium))
          .foregroundStyle(Color.mdowText)
        if let detail {
          Text(detail)
            .font(.mdowUI(12))
            .foregroundStyle(Color.mdowSecondary)
            .fixedSize(horizontal: false, vertical: true)
        }
      }
      .frame(maxWidth: .infinity, alignment: .leading)
      control
    }
    .padding(.horizontal, 16)
    .padding(.vertical, 12)
    .frame(minHeight: 52)
  }
}

// MARK: Buttons

/// A quiet button with a hairline border and a soft fill on hover, for actions like
/// "Restore Defaults".
struct GhostButtonStyle: ButtonStyle {
  func makeBody(configuration: Configuration) -> some View {
    GhostButton(configuration: configuration)
  }

  private struct GhostButton: View {
    var configuration: Configuration
    @Environment(\.isEnabled) private var isEnabled
    @State private var hovering = false

    var body: some View {
      configuration.label
        .font(.mdowUI(12.5, .medium))
        .foregroundStyle(isEnabled ? Color.mdowText : Color.mdowTertiary)
        .padding(.horizontal, 12)
        .frame(height: 28)
        .background(
          configuration.isPressed ? Color.mdowSelected : hovering ? Color.mdowHover : .clear,
          in: .rect(cornerRadius: 7, style: .continuous)
        )
        .hairline(7, color: .mdowStrongLine)
        .contentShape(.rect)
        .onHover { hovering = $0 && isEnabled }
        .mdowFade(hovering)
    }
  }
}

/// One choice in a row of pills: filled when selected, a fainter fill on hover.
private struct PillButtonStyle: ButtonStyle {
  var selected: Bool

  func makeBody(configuration: Configuration) -> some View {
    Pill(configuration: configuration, selected: selected)
  }

  private struct Pill: View {
    var configuration: Configuration
    var selected: Bool
    @State private var hovering = false

    var body: some View {
      configuration.label
        .font(.mdowUI(12.5, .medium))
        .foregroundStyle(selected || hovering ? Color.mdowText : Color.mdowSecondary)
        .padding(.horizontal, 10)
        .frame(height: 26)
        .background(fill, in: .rect(cornerRadius: 7, style: .continuous))
        .contentShape(.rect)
        .onHover { hovering = $0 }
        .mdowFade(hovering)
        .mdowFade(selected)
    }

    private var fill: Color {
      if selected || configuration.isPressed { return .mdowSelected }
      return hovering ? .mdowHover : .clear
    }
  }
}

// MARK: Choices

/// A segmented choice drawn as a row of pills. VoiceOver sees an ordinary picker.
struct PillPicker<Value: Hashable & Identifiable>: View {
  var label: String
  @Binding var selection: Value
  var options: [Value]
  var title: (Value) -> String

  var body: some View {
    HStack(spacing: 2) {
      ForEach(options) { option in
        Button(title(option)) { selection = option }
          .buttonStyle(PillButtonStyle(selected: option == selection))
      }
    }
    .padding(2)
    .background(Color.mdowWell, in: .rect(cornerRadius: 9, style: .continuous))
    .hairline(9)
    .fixedSize()
    .accessibilityRepresentation {
      Picker(label, selection: $selection) {
        ForEach(options) { Text(title($0)).tag($0) }
      }
      .pickerStyle(.segmented)
    }
  }
}

/// One entry in a `MenuPill`'s menu.
struct MenuChoice {
  enum Kind { case item, separator, header }

  var kind: Kind = .item
  var title = ""
  var font: NSFont?
  var isSelected = false
  var action: () -> Void = {}

  static var separator: MenuChoice { MenuChoice(kind: .separator) }
  static func header(_ title: String) -> MenuChoice { MenuChoice(kind: .header, title: title) }
}

/// A pill showing the current value with a chevron; clicking it drops down a menu of choices.
/// The value can be drawn in its own font, so font pickers preview themselves.
struct MenuPill: View {
  var label: String
  var value: String
  var valueFont: Font = .mdowUI(12.5, .medium)
  var minWidth: CGFloat = 150
  var choices: () -> [MenuChoice]

  @State private var anchor = MenuAnchor()
  @State private var hovering = false

  var body: some View {
    Button(action: showMenu) {
      HStack(spacing: 8) {
        Text(value)
          .font(valueFont)
          .foregroundStyle(Color.mdowText)
          .lineLimit(1)
        Spacer(minLength: 10)
        Image(systemName: "chevron.up.chevron.down")
          .font(.system(size: 9, weight: .semibold))
          .foregroundStyle(Color.mdowTertiary)
      }
      .padding(.leading, 11)
      .padding(.trailing, 9)
      .frame(minWidth: minWidth, minHeight: 28, maxHeight: 28)
      .fixedSize()
      .background(
        hovering ? Color.mdowHover : Color.mdowWell, in: .rect(cornerRadius: 8, style: .continuous)
      )
      .hairline(8, color: hovering ? .mdowStrongLine : .mdowLine)
      .contentShape(.rect)
    }
    .buttonStyle(.plain)
    .background(MenuAnchorView(anchor: anchor))
    .onHover { hovering = $0 }
    .mdowFade(hovering)
    .accessibilityLabel(label)
    .accessibilityValue(value)
    .accessibilityHint("Shows a menu")
  }

  private func showMenu() {
    guard let view = anchor.view else { return }
    let menu = NSMenu()
    menu.autoenablesItems = false
    let target = MenuTarget()
    for choice in choices() {
      switch choice.kind {
      case .separator:
        menu.addItem(.separator())
      case .header:
        menu.addItem(.sectionHeader(title: choice.title))
      case .item:
        let item = NSMenuItem(
          title: choice.title, action: #selector(MenuTarget.fire(_:)), keyEquivalent: "")
        item.target = target
        item.representedObject = choice.action
        item.state = choice.isSelected ? .on : .off
        if let font = choice.font {
          item.attributedTitle = NSAttributedString(string: choice.title, attributes: [.font: font])
        }
        menu.addItem(item)
      }
    }
    menu.minimumWidth = view.bounds.width
    // Drops down from the pill's bottom edge. Items only hold their target weakly.
    withExtendedLifetime(target) {
      _ = menu.popUp(positioning: nil, at: NSPoint(x: 0, y: -4), in: view)
    }
  }
}

/// Holds the AppKit view behind a `MenuPill`, so its menu can drop down from the right place.
private final class MenuAnchor {
  weak var view: NSView?
}

private struct MenuAnchorView: NSViewRepresentable {
  var anchor: MenuAnchor

  func makeNSView(context: Context) -> NSView {
    let view = NSView()
    anchor.view = view
    return view
  }

  func updateNSView(_ nsView: NSView, context: Context) { anchor.view = nsView }
}

@MainActor private final class MenuTarget: NSObject {
  @objc func fire(_ item: NSMenuItem) {
    (item.representedObject as? () -> Void)?()
  }
}

// MARK: Switch and slider

/// A small switch tinted with the accent colour.
struct MdowSwitchStyle: ToggleStyle {
  func makeBody(configuration: Configuration) -> some View {
    Switch(configuration: configuration)
  }

  private struct Switch: View {
    var configuration: Configuration
    @Environment(\.isEnabled) private var isEnabled

    var body: some View {
      Button {
        configuration.isOn.toggle()
      } label: {
        ZStack(alignment: configuration.isOn ? .trailing : .leading) {
          Capsule()
            .fill(configuration.isOn ? Color.mdowAccent : Color.mdowSelected)
            .overlay(
              Capsule().strokeBorder(configuration.isOn ? .clear : Color.mdowLine, lineWidth: 1))
            .frame(width: 30, height: 18)
          Circle()
            .fill(.white)
            .shadow(color: .black.opacity(0.18), radius: 1, y: 0.5)
            .frame(width: 14, height: 14)
            .padding(2)
        }
        .opacity(isEnabled ? 1 : 0.5)
        .contentShape(.rect)
      }
      .buttonStyle(.plain)
      .mdowFade(configuration.isOn)
      .accessibilityRepresentation {
        Toggle(isOn: configuration.$isOn) { configuration.label }
      }
    }
  }
}

/// A thin slider with an accent fill and a round knob. Like the system's, it takes focus with
/// keyboard navigation turned on, and the arrow keys then step it.
struct MdowSlider: View {
  var label: String
  @Binding var value: Double
  var range: ClosedRange<Double>
  var step: Double

  @FocusState private var focused: Bool
  private let knob: CGFloat = 16

  var body: some View {
    GeometryReader { geometry in
      let width = geometry.size.width - knob
      let fraction = (value - range.lowerBound) / (range.upperBound - range.lowerBound)
      ZStack(alignment: .leading) {
        Capsule().fill(Color.mdowSelected).frame(height: 4).padding(.horizontal, knob / 2)
        Capsule().fill(Color.mdowAccent)
          .frame(width: max(0, width * fraction), height: 4)
          .padding(.leading, knob / 2)
        Circle()
          .fill(Color.mdowElevated)
          .overlay(Circle().strokeBorder(Color.mdowStrongLine, lineWidth: 1))
          .shadow(color: .black.opacity(0.12), radius: 1.5, y: 1)
          .overlay(
            Circle().strokeBorder(Color.mdowAccent.opacity(focused ? 0.6 : 0), lineWidth: 2)
              .padding(-3)
          )
          .frame(width: knob, height: knob)
          .offset(x: width * fraction)
      }
      .frame(maxHeight: .infinity)
      .contentShape(.rect)
      .gesture(
        DragGesture(minimumDistance: 0).onChanged { drag in
          let x = min(max(0, drag.location.x - knob / 2), width)
          set(range.lowerBound + Double(x / max(width, 1)) * (range.upperBound - range.lowerBound))
        }
      )
    }
    .frame(height: 22)
    .focusable(interactions: .activate)
    .focused($focused)
    .focusEffectDisabled()
    .onKeyPress(.leftArrow) { set(value - step); return .handled }
    .onKeyPress(.downArrow) { set(value - step); return .handled }
    .onKeyPress(.rightArrow) { set(value + step); return .handled }
    .onKeyPress(.upArrow) { set(value + step); return .handled }
    .accessibilityRepresentation {
      Slider(value: $value, in: range, step: step) { Text(label) }
    }
  }

  private func set(_ newValue: Double) {
    let stepped = (newValue / step).rounded() * step
    let clamped = min(max(stepped, range.lowerBound), range.upperBound)
    if clamped != value { value = clamped }
  }
}

// MARK: Key chips

/// A key combination drawn as a small mono chip, e.g. ⇧⌘G.
struct KeyChip: View {
  var keys: String

  init(_ keys: String) { self.keys = keys }

  var body: some View {
    Text(keys)
      .font(.mdowMono(11.5, .medium))
      .tracking(1.5)
      .foregroundStyle(Color.mdowSecondary)
      .padding(.leading, 6.5)
      .padding(.trailing, 5)
      .frame(minWidth: 24, minHeight: 22)
      .background(Color.mdowWell, in: .rect(cornerRadius: 6, style: .continuous))
      .hairline(6, color: .mdowStrongLine)
  }
}

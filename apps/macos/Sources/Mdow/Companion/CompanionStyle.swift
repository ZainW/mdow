import AppKit
import MdowKit
import SwiftUI

/// The companion's visual vocabulary: flat surfaces, hairlines, soft hover fills and pill
/// buttons, in the window's palette and fonts.
enum CompanionStyle {
  /// Space between the panel's content and the document card on its left.
  static let leading: CGFloat = 12
  /// Space between the panel's content and the window's right edge, matching the card's gutter.
  static let trailing: CGFloat = Theme.gutter

  /// Charter, for the assistant's prose and the empty state's greeting.
  static func serif(_ size: CGFloat, _ weight: NSFont.Weight = .regular) -> Font {
    Font(BodyFont.charter.font(size: size, weight: weight) as CTFont)
  }

  /// Animation for small state changes, or none when the user asks for reduced motion.
  static func animation(reduceMotion: Bool) -> Animation? {
    reduceMotion ? nil : .easeOut(duration: 0.15)
  }
}

extension Color {
  /// Text and icons on an accent fill: white on the light theme's clay, ink on the dark one's.
  static let companionOnAccent = Color(
    nsColor: ReaderColors.dynamic(light: .white, dark: Palette.rgb(0x1B1B1B)))
  /// Additions in diffs and the "Applied" state.
  static let companionAdded = Color(
    nsColor: ReaderColors.dynamic(light: Palette.rgb(0x2F7D4F), dark: Palette.rgb(0x74C48F)))
  /// Removals in diffs, errors and the "Failed" state.
  static let companionRemoved = Color(
    nsColor: ReaderColors.dynamic(light: Palette.rgb(0xB4413A), dark: Palette.rgb(0xEC7A70)))
}

// MARK: Buttons

/// A square icon button with a soft rounded fill on hover, like the window's chrome buttons.
struct CompanionIconButtonStyle: ButtonStyle {
  var side: CGFloat = 26

  func makeBody(configuration: Configuration) -> some View {
    Styled(configuration: configuration, side: side)
  }

  private struct Styled: View {
    var configuration: Configuration
    var side: CGFloat
    @Environment(\.isEnabled) private var isEnabled
    @State private var hovering = false

    var body: some View {
      let active = isEnabled && (hovering || configuration.isPressed)
      configuration.label
        .font(.system(size: 12.5, weight: .medium))
        .foregroundStyle(
          !isEnabled ? Color.mdowTertiary : active ? Color.mdowText : Color.mdowSecondary)
        .frame(width: side, height: side)
        .background(
          isEnabled && configuration.isPressed
            ? Color.mdowSelected : active ? Color.mdowHover : .clear,
          in: .rect(cornerRadius: 7))
        .contentShape(.rect(cornerRadius: 7))
        .onHover { hovering = $0 }
    }
  }
}

/// A small pill button. `primary` is filled with the accent, `secondary` is a hairline outline
/// and `quiet` is text that gains a soft fill on hover.
struct CompanionPillButtonStyle: ButtonStyle {
  enum Kind { case primary, secondary, quiet }
  var kind: Kind = .secondary

  func makeBody(configuration: Configuration) -> some View {
    Styled(configuration: configuration, kind: kind)
  }

  private struct Styled: View {
    var configuration: Configuration
    var kind: Kind
    @Environment(\.isEnabled) private var isEnabled
    @State private var hovering = false

    var body: some View {
      let pressed = configuration.isPressed
      configuration.label
        .font(.mdowUI(12, .medium))
        .lineLimit(1)
        .foregroundStyle(foreground)
        .padding(.horizontal, 10)
        .frame(height: 24)
        .background(background(pressed: pressed), in: .capsule)
        .overlay {
          if kind == .secondary {
            Capsule().strokeBorder(Color.mdowStrongLine, lineWidth: 1)
          }
        }
        .brightness(kind == .primary && isEnabled ? (pressed ? -0.06 : hovering ? 0.04 : 0) : 0)
        .opacity(isEnabled ? 1 : 0.5)
        .contentShape(.capsule)
        .onHover { hovering = $0 }
    }

    private var foreground: Color {
      switch kind {
      case .primary: .companionOnAccent
      case .secondary: .mdowText
      case .quiet: hovering && isEnabled ? .mdowText : .mdowSecondary
      }
    }

    private func background(pressed: Bool) -> Color {
      switch kind {
      case .primary: return .mdowAccent
      case .secondary, .quiet:
        guard isEnabled else { return .clear }
        return pressed ? .mdowSelected : hovering ? .mdowHover : .clear
      }
    }
  }
}

// MARK: Disclosure

/// A compact row that expands to show more: a chevron, a label, and the content below it.
/// Used for reasoning, tool calls and sources.
struct CompanionDisclosure<Label: View, Content: View>: View {
  @Binding var expanded: Bool
  @ViewBuilder var content: () -> Content
  @ViewBuilder var label: () -> Label
  @Environment(\.accessibilityReduceMotion) private var reduceMotion
  @State private var hovering = false

  var body: some View {
    VStack(alignment: .leading, spacing: 4) {
      Button {
        withAnimation(CompanionStyle.animation(reduceMotion: reduceMotion)) { expanded.toggle() }
      } label: {
        HStack(spacing: 6) {
          label()
          Image(systemName: "chevron.right")
            .font(.system(size: 8.5, weight: .bold))
            .foregroundStyle(Color.mdowTertiary)
            .rotationEffect(.degrees(expanded ? 90 : 0))
            .opacity(hovering || expanded ? 1 : 0.6)
        }
        .padding(.horizontal, 6)
        .frame(height: 24)
        .background(hovering ? Color.mdowHover : .clear, in: .rect(cornerRadius: 6))
        .contentShape(.rect(cornerRadius: 6))
      }
      .buttonStyle(.plain)
      .padding(.horizontal, -6)
      .onHover { hovering = $0 }
      .accessibilityValue(expanded ? "Expanded" : "Collapsed")
      if expanded {
        content()
          .transition(.opacity)
      }
    }
  }
}

// MARK: Surfaces

extension View {
  /// A flat card: paper fill and a hairline border.
  func companionCard(radius: CGFloat = 10, fill: Color = .mdowPaper, border: Color = .mdowLine)
    -> some View
  {
    background(fill, in: .rect(cornerRadius: radius, style: .continuous))
      .overlay(
        RoundedRectangle(cornerRadius: radius, style: .continuous)
          .strokeBorder(border, lineWidth: 1))
  }
}

/// A small spinner-free activity mark: three dots that fade in turn, or still dots when the
/// user asks for reduced motion.
struct CompanionActivityDots: View {
  @Environment(\.accessibilityReduceMotion) private var reduceMotion

  var body: some View {
    if reduceMotion {
      dots(phase: nil)
    } else {
      TimelineView(.animation(minimumInterval: 1 / 15)) { context in
        dots(phase: context.date.timeIntervalSinceReferenceDate)
      }
    }
  }

  private func dots(phase: Double?) -> some View {
    HStack(spacing: 3) {
      ForEach(0..<3) { index in
        Circle()
          .fill(Color.mdowAccent)
          .frame(width: 4, height: 4)
          .opacity(opacity(index, phase))
      }
    }
    .accessibilityHidden(true)
  }

  private func opacity(_ index: Int, _ phase: Double?) -> Double {
    guard let phase else { return 0.6 }
    let t = (phase * 1.4 - Double(index) * 0.18).truncatingRemainder(dividingBy: 1)
    return 0.25 + 0.75 * max(0, sin(t * .pi))
  }
}

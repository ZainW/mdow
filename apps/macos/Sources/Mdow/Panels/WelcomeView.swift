import AppKit
import MdowKit
import SwiftUI

/// What an empty tab shows: a greeting, open actions and recent documents.
struct WelcomeView: View {
  var open: (URL) -> Void
  @State private var preferences = Preferences.shared

  var body: some View {
    ScrollView {
      VStack(alignment: .leading, spacing: 0) {
        Image(nsImage: NSApp.applicationIconImage)
          .resizable()
          .frame(width: 44, height: 44)
          .accessibilityHidden(true)
        Text(greeting)
          .font(.custom("Charter", size: 34))
          .foregroundStyle(Color.mdowText)
          .padding(.top, 22)
        Text("Open a Markdown file or a folder to start reading.")
          .font(.mdowUI(14))
          .foregroundStyle(Color.mdowSecondary)
          .padding(.top, 6)

        HStack(spacing: 10) {
          ActionCard(symbol: "doc.text", title: "Open file", keys: "⌘O") {
            NSApp.sendAction(#selector(DocumentWindowController.openDocument(_:)), to: nil, from: nil)
          }
          ActionCard(symbol: "folder", title: "Open folder", keys: "⇧⌘O") {
            NSApp.sendAction(#selector(DocumentWindowController.openFolder(_:)), to: nil, from: nil)
          }
        }
        .padding(.top, 28)

        if !recents.isEmpty {
          Text("Recent")
            .font(.mdowUI(12, .medium))
            .foregroundStyle(Color.mdowTertiary)
            .padding(.top, 36)
            .padding(.bottom, 8)
            .padding(.leading, 2)
          VStack(spacing: 0) {
            ForEach(recents, id: \.self) { url in
              RecentRow(url: url) { open(url) }
            }
          }
          .padding(4)
          .overlay(RoundedRectangle(cornerRadius: 12, style: .continuous).strokeBorder(Color.mdowLine))
        }

        Text("Drop files or a folder anywhere in the window to open them.")
          .font(.mdowUI(12))
          .foregroundStyle(Color.mdowTertiary)
          .padding(.top, 24)
      }
      .frame(maxWidth: 520, alignment: .leading)
      .padding(.horizontal, 40)
      .padding(.vertical, 64)
      .frame(maxWidth: .infinity)
    }
    .scrollBounceBehavior(.basedOnSize)
    .background(Color.mdowPaper)
  }

  private var greeting: String {
    switch Calendar.current.component(.hour, from: Date()) {
    case 5..<12: "Good morning."
    case 12..<18: "Good afternoon."
    default: "Good evening."
    }
  }

  private var recents: [URL] {
    Array(preferences.recents.filter { FileManager.default.fileExists(atPath: $0.path) }.prefix(6))
  }
}

/// A large, quiet button: an icon, a title and its shortcut.
private struct ActionCard: View {
  var symbol: String
  var title: String
  var keys: String
  var action: () -> Void
  @State private var hovering = false

  var body: some View {
    Button(action: action) {
      VStack(alignment: .leading, spacing: 18) {
        // A fixed box, so cards with taller or shorter symbols keep the same height.
        Image(systemName: symbol)
          .font(.system(size: 15, weight: .medium))
          .foregroundStyle(Color.mdowAccent)
          .frame(width: 18, height: 18, alignment: .leading)
        HStack {
          Text(title)
            .font(.mdowUI(13.5, .medium))
            .foregroundStyle(Color.mdowText)
          Spacer()
          Text(keys)
            .font(.mdowMono(11))
            .foregroundStyle(Color.mdowTertiary)
        }
      }
      .padding(14)
      .frame(maxWidth: .infinity, alignment: .leading)
      .background(
        hovering ? Color.mdowHover : Color.mdowElevated,
        in: RoundedRectangle(cornerRadius: 12, style: .continuous)
      )
      .overlay(RoundedRectangle(cornerRadius: 12, style: .continuous).strokeBorder(Color.mdowLine))
      .contentShape(.rect)
    }
    .buttonStyle(.plain)
    .onHover { hovering = $0 }
    .animation(.easeOut(duration: 0.12), value: hovering)
  }
}

private struct RecentRow: View {
  var url: URL
  var action: () -> Void
  @State private var hovering = false

  var body: some View {
    Button(action: action) {
      HStack(spacing: 10) {
        Image(systemName: FileKind(url: url) == .html ? "globe" : "doc.text")
          .font(.system(size: 12))
          .foregroundStyle(Color.mdowTertiary)
          .frame(width: 16)
        Text(url.lastPathComponent)
          .font(.mdowUI(13, .medium))
          .foregroundStyle(Color.mdowText)
          .lineLimit(1)
        Spacer(minLength: 16)
        Text(url.deletingLastPathComponent().abbreviatedPath)
          .font(.mdowMono(11))
          .foregroundStyle(Color.mdowTertiary)
          .lineLimit(1)
          .truncationMode(.head)
      }
      .padding(.horizontal, 10)
      .frame(height: 34)
      .contentShape(.rect)
      .background(
        hovering ? Color.mdowHover : .clear, in: RoundedRectangle(cornerRadius: 8, style: .continuous))
    }
    .buttonStyle(.plain)
    .onHover { hovering = $0 }
    .help(url.path)
  }
}

/// Why a document can't be shown.
struct MessageView: View {
  var title: String
  var message: String
  var symbol: String

  var body: some View {
    VStack(spacing: 0) {
      Image(systemName: symbol)
        .font(.system(size: 18, weight: .medium))
        .foregroundStyle(Color.mdowSecondary)
        .frame(width: 44, height: 44)
        .background(Color.mdowWell, in: RoundedRectangle(cornerRadius: 12, style: .continuous))
        .overlay(RoundedRectangle(cornerRadius: 12, style: .continuous).strokeBorder(Color.mdowLine))
      Text(title)
        .font(.custom("Charter", size: 22))
        .foregroundStyle(Color.mdowText)
        .multilineTextAlignment(.center)
        .padding(.top, 18)
      Text(message)
        .font(.mdowUI(13))
        .foregroundStyle(Color.mdowSecondary)
        .multilineTextAlignment(.center)
        .padding(.top, 6)
    }
    .frame(maxWidth: 420)
    .padding(40)
    .frame(maxWidth: .infinity, maxHeight: .infinity)
    .background(Color.mdowPaper)
  }
}

extension URL {
  /// The path with the home folder shown as `~`.
  var abbreviatedPath: String { (path as NSString).abbreviatingWithTildeInPath }
}

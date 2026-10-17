import AppKit
import MdowKit
import SwiftUI

/// What an empty window shows: open buttons and recent documents.
struct WelcomeView: View {
  var open: (URL) -> Void
  @State private var preferences = Preferences.shared

  var body: some View {
    VStack(spacing: 0) {
      Spacer(minLength: 40)
      Image(nsImage: NSApp.applicationIconImage)
        .resizable()
        .frame(width: 112, height: 112)
        .accessibilityHidden(true)
      Text("Mdow")
        .font(.system(size: 32, weight: .bold))
        .padding(.top, 8)
      Text("Open a Markdown file or a folder to start reading.")
        .font(.title3)
        .foregroundStyle(.secondary)
        .padding(.top, 4)

      HStack(spacing: 12) {
        Button {
          NSApp.sendAction(#selector(DocumentWindowController.openDocument(_:)), to: nil, from: nil)
        } label: {
          Label("Open File…", systemImage: "doc")
            .frame(minWidth: 110)
        }
        .buttonStyle(.borderedProminent)
        Button {
          NSApp.sendAction(#selector(DocumentWindowController.openFolder(_:)), to: nil, from: nil)
        } label: {
          Label("Open Folder…", systemImage: "folder")
            .frame(minWidth: 110)
        }
      }
      .controlSize(.large)
      .padding(.top, 24)

      if !recents.isEmpty {
        VStack(alignment: .leading, spacing: 6) {
          Text("Recent")
            .font(.headline)
            .foregroundStyle(.secondary)
            .padding(.leading, 10)
          VStack(spacing: 2) {
            ForEach(recents, id: \.self) { url in
              RecentRow(url: url) { open(url) }
            }
          }
        }
        .frame(width: 440)
        .padding(.top, 36)
      }

      Spacer(minLength: 24)
      Text("You can also drop files or a folder onto this window.")
        .font(.callout)
        .foregroundStyle(.tertiary)
        .padding(.bottom, 24)
    }
    .frame(maxWidth: .infinity, maxHeight: .infinity)
    .background(Color(nsColor: .textBackgroundColor))
  }

  private var recents: [URL] {
    Array(preferences.recents.filter { FileManager.default.fileExists(atPath: $0.path) }.prefix(6))
  }
}

private struct RecentRow: View {
  var url: URL
  var action: () -> Void
  @State private var hovering = false

  var body: some View {
    Button(action: action) {
      HStack(spacing: 10) {
        Image(nsImage: NSWorkspace.shared.icon(forFile: url.path))
          .resizable()
          .frame(width: 28, height: 28)
        VStack(alignment: .leading, spacing: 1) {
          Text(url.lastPathComponent)
            .lineLimit(1)
          Text(url.deletingLastPathComponent().abbreviatedPath)
            .font(.caption)
            .foregroundStyle(.secondary)
            .lineLimit(1)
            .truncationMode(.middle)
        }
        Spacer()
      }
      .padding(.horizontal, 10)
      .padding(.vertical, 5)
      .contentShape(.rect)
      .background(
        hovering ? AnyShapeStyle(.quaternary) : AnyShapeStyle(.clear),
        in: .rect(cornerRadius: 8))
    }
    .buttonStyle(.plain)
    .onHover { hovering = $0 }
    .help(url.path)
  }
}

extension URL {
  /// The path with the home folder shown as `~`.
  var abbreviatedPath: String { (path as NSString).abbreviatingWithTildeInPath }
}

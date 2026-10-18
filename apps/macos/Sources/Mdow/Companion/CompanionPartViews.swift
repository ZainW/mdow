import AppKit
import MdowKit
import SwiftUI

// MARK: Reply text

/// Assistant markdown, drawn with SwiftUI text: paragraphs and headings with inline styling,
/// and code blocks in a monospaced box.
struct ReplyText: View {
  var markdown: String

  var body: some View {
    VStack(alignment: .leading, spacing: 8) {
      ForEach(Array(ReplyBlocks.split(markdown).enumerated()), id: \.offset) { _, block in
        switch block {
        case .paragraph(let text):
          Text(Self.inline(text))
            .fixedSize(horizontal: false, vertical: true)
        case .heading(let level, let text):
          Text(Self.inline(text))
            .font(level <= 2 ? .headline : .subheadline.weight(.semibold))
            .padding(.top, 2)
        case .code(_, let text):
          ScrollView(.horizontal, showsIndicators: false) {
            Text(text)
              .font(.system(.callout, design: .monospaced))
              .fixedSize(horizontal: true, vertical: true)
              .padding(8)
          }
          .frame(maxWidth: .infinity, alignment: .leading)
          .background(.quaternary.opacity(0.5), in: .rect(cornerRadius: 6))
        }
      }
    }
    .textSelection(.enabled)
  }

  static func inline(_ text: String) -> AttributedString {
    let options = AttributedString.MarkdownParsingOptions(
      interpretedSyntax: .inlineOnlyPreservingWhitespace, failurePolicy: .returnPartiallyParsedIfPossible)
    return (try? AttributedString(markdown: listBullets(text), options: options)) ?? AttributedString(text)
  }

  /// `- item` and `* item` read better as bullets when only inline markdown is interpreted.
  private static func listBullets(_ text: String) -> String {
    text.split(separator: "\n", omittingEmptySubsequences: false).map { line in
      let indent = line.prefix { $0 == " " }
      let rest = line.dropFirst(indent.count)
      if rest.hasPrefix("- ") || rest.hasPrefix("* ") || rest.hasPrefix("+ ") {
        return indent + "•\u{2002}" + rest.dropFirst(2)
      }
      return String(line)
    }.joined(separator: "\n")
  }
}

// MARK: Reasoning

struct ReasoningView: View {
  var text: String
  var active: Bool
  @State private var expanded = false

  var body: some View {
    DisclosureGroup(isExpanded: $expanded) {
      Text(text.trimmingCharacters(in: .whitespacesAndNewlines))
        .font(.callout)
        .foregroundStyle(.secondary)
        .textSelection(.enabled)
        .fixedSize(horizontal: false, vertical: true)
        .padding(.top, 4)
        .frame(maxWidth: .infinity, alignment: .leading)
    } label: {
      HStack(spacing: 6) {
        Image(systemName: "brain")
        Text(active ? "Thinking…" : "Thought")
        if active { ProgressView().controlSize(.mini) }
      }
      .foregroundStyle(.secondary)
    }
    .font(.callout)
  }
}

// MARK: Tool calls

struct ToolCallView: View {
  var tool: CompanionToolPart
  @State private var expanded = false

  var body: some View {
    DisclosureGroup(isExpanded: $expanded) {
      VStack(alignment: .leading, spacing: 6) {
        if let input = tool.input, !input.isEmpty { DetailBox(title: "Input", text: input) }
        if let error = tool.error {
          DetailBox(title: "Error", text: error, isError: true)
        } else if let output = tool.output, !output.isEmpty {
          DetailBox(title: "Output", text: output)
        }
      }
      .padding(.top, 4)
    } label: {
      HStack(spacing: 6) {
        stateIcon
          .frame(width: 14)
        Text(tool.name)
          .lineLimit(1)
          .truncationMode(.middle)
      }
      .help(tool.name)
    }
    .font(.callout)
  }

  @ViewBuilder private var stateIcon: some View {
    switch tool.state {
    case .pending, .running:
      ProgressView().controlSize(.mini)
    case .completed:
      Image(systemName: "checkmark.circle").foregroundStyle(.secondary)
    case .error:
      Image(systemName: "exclamationmark.circle").foregroundStyle(.red)
    case .cancelled:
      Image(systemName: "minus.circle").foregroundStyle(.tertiary)
    }
  }
}

private struct DetailBox: View {
  var title: String
  var text: String
  var isError = false

  var body: some View {
    VStack(alignment: .leading, spacing: 3) {
      Text(title.uppercased())
        .font(.caption2.weight(.medium))
        .foregroundStyle(.secondary)
      ScrollView([.vertical, .horizontal]) {
        Text(text)
          .font(.system(.caption, design: .monospaced))
          .foregroundStyle(isError ? AnyShapeStyle(.red) : AnyShapeStyle(.primary))
          .textSelection(.enabled)
          .fixedSize()
          .padding(6)
          .frame(maxWidth: .infinity, alignment: .leading)
      }
      .frame(maxHeight: 160)
      .fixedSize(horizontal: false, vertical: true)
      .background(.quaternary.opacity(0.5), in: .rect(cornerRadius: 5))
    }
  }
}

// MARK: Change cards

struct ChangeCardView: View {
  var change: CompanionChangePart
  var openDocument: (URL) -> Void
  @Environment(CompanionSession.self) private var session

  var body: some View {
    let settled = change.status == .rejected || change.status == .failed
    let additions = change.files.reduce(0) { $0 + $1.additions }
    let deletions = change.files.reduce(0) { $0 + $1.deletions }
    let title: String =
      switch change.files.count {
      case 0: "Proposed change"
      case 1: change.files[0].displayPath
      default: "\(change.files.count) documents"
      }
    let added = !change.files.isEmpty && change.files.allSatisfy { $0.status == .added }

    VStack(alignment: .leading, spacing: 0) {
      HStack(spacing: 6) {
        Image(systemName: added ? "doc.badge.plus" : "pencil.line")
          .foregroundStyle(.secondary)
        Text(title)
          .fontWeight(.medium)
          .lineLimit(1)
          .truncationMode(.middle)
          .help(title)
        Spacer(minLength: 4)
        if additions > 0 || deletions > 0 {
          HStack(spacing: 4) {
            Text("+\(additions)").foregroundStyle(.green)
            Text("−\(deletions)").foregroundStyle(.red)
          }
          .font(.system(.caption, design: .monospaced))
        }
        StatusPill(status: change.status, applying: change.applying)
      }
      .font(.callout)
      .padding(.horizontal, 10)
      .padding(.vertical, 7)
      .background(.quaternary.opacity(0.35))

      ForEach(change.files, id: \.path) { file in
        Divider()
        if change.files.count > 1 {
          Text(file.displayPath)
            .font(.system(.caption, design: .monospaced))
            .foregroundStyle(.secondary)
            .lineLimit(1)
            .truncationMode(.middle)
            .padding(.horizontal, 10)
            .padding(.vertical, 3)
        }
        DiffView(file: file, collapsedByDefault: settled)
      }

      if let error = change.error, change.status == .failed {
        Divider()
        Text(error)
          .font(.caption)
          .foregroundStyle(.red)
          .padding(10)
      }

      if change.awaitingReview || (change.status == .applied && !change.files.isEmpty) {
        Divider()
        HStack(spacing: 6) {
          if change.files.count == 1, change.files[0].status != .deleted {
            Button("Open Document") {
              openDocument(URL(fileURLWithPath: change.files[0].path))
            }
            .buttonStyle(.link)
            .lineLimit(1)
            .fixedSize()
          }
          Spacer(minLength: 0)
          if let permissionId = change.permissionId, change.awaitingReview {
            Button("Decline") { session.review(permissionId, .reject) }
            Button {
              session.review(permissionId, .approve)
            } label: {
              Label("Accept", systemImage: "checkmark")
            }
            .buttonStyle(.borderedProminent)
          }
        }
        .controlSize(.small)
        .padding(.horizontal, 10)
        .padding(.vertical, 6)
      }
    }
    .background(.background, in: .rect(cornerRadius: 8))
    .clipShape(.rect(cornerRadius: 8))
    .overlay(
      RoundedRectangle(cornerRadius: 8)
        .strokeBorder(change.awaitingReview ? AnyShapeStyle(.orange.opacity(0.55)) : AnyShapeStyle(.separator))
    )
    .opacity(settled ? 0.75 : 1)
    .accessibilityElement(children: .contain)
    .accessibilityLabel("Change to \(title)")
  }
}

private struct StatusPill: View {
  var status: CompanionChangeStatus
  var applying: Bool

  var body: some View {
    Group {
      if applying {
        HStack(spacing: 4) {
          ProgressView().controlSize(.mini)
          Text("Applying")
        }
        .foregroundStyle(.secondary)
      } else {
        switch status {
        case .pending:
          Label("Review", systemImage: "circle.fill")
            .labelStyle(PillLabelStyle(iconScale: 0.5))
            .foregroundStyle(.orange)
        case .applied:
          Label("Applied", systemImage: "checkmark").labelStyle(PillLabelStyle()).foregroundStyle(.green)
        case .rejected:
          Label("Declined", systemImage: "arrow.uturn.backward").labelStyle(PillLabelStyle())
            .foregroundStyle(.secondary)
        case .failed:
          Label("Failed", systemImage: "xmark").labelStyle(PillLabelStyle()).foregroundStyle(.red)
        }
      }
    }
    .font(.caption.weight(.medium))
  }
}

private struct PillLabelStyle: LabelStyle {
  var iconScale: CGFloat = 0.85

  func makeBody(configuration: Configuration) -> some View {
    HStack(spacing: 3) {
      configuration.icon.scaleEffect(iconScale)
      configuration.title
    }
  }
}

// MARK: Diffs

/// A unified diff, long ones collapsed past `previewLines` so the conversation stays readable.
struct DiffView: View {
  var file: CompanionFileChange
  var collapsedByDefault = false
  var previewLines = 14
  @State private var expanded: Bool?

  var body: some View {
    let hunks = UnifiedDiff.parse(file.patch)
    let total = hunks.reduce(0) { $0 + $1.lines.count }
    let showAll = (expanded ?? !collapsedByDefault) || total <= previewLines
    if hunks.isEmpty {
      Text(file.status == .deleted ? "Deletes this document." : "No preview available.")
        .font(.caption)
        .foregroundStyle(.secondary)
        .padding(10)
    } else {
      VStack(alignment: .leading, spacing: 0) {
        ForEach(Array(visible(hunks, showAll: showAll).enumerated()), id: \.offset) { index, hunk in
          if index > 0 {
            Rectangle().fill(.separator).frame(height: 1).padding(.vertical, 2)
          }
          ForEach(Array(hunk.lines.enumerated()), id: \.offset) { _, line in
            DiffRow(line: line)
          }
        }
        if total > previewLines {
          Button {
            expanded = !showAll
          } label: {
            HStack(spacing: 4) {
              Image(systemName: showAll ? "chevron.up" : "chevron.down")
              Text(showAll ? "Show Less" : "Show All \(total) Lines")
            }
            .font(.caption)
            .foregroundStyle(.secondary)
            .frame(maxWidth: .infinity)
            .padding(.vertical, 4)
            .contentShape(.rect)
          }
          .buttonStyle(.plain)
        }
      }
      .font(.system(size: 11, design: .monospaced))
      .textSelection(.enabled)
    }
  }

  private func visible(_ hunks: [DiffHunk], showAll: Bool) -> [DiffHunk] {
    guard !showAll else { return hunks }
    var remaining = previewLines
    var result: [DiffHunk] = []
    for hunk in hunks where remaining > 0 {
      result.append(DiffHunk(header: hunk.header, lines: Array(hunk.lines.prefix(remaining))))
      remaining -= hunk.lines.count
    }
    return result
  }
}

private struct DiffRow: View {
  var line: DiffLine

  var body: some View {
    let number = line.kind == .remove ? line.oldLine : line.newLine
    HStack(alignment: .firstTextBaseline, spacing: 0) {
      Text(number.map(String.init) ?? "")
        .foregroundStyle(.tertiary)
        .frame(width: 30, alignment: .trailing)
        .padding(.trailing, 4)
      Text(marker)
        .foregroundStyle(markerStyle)
        .frame(width: 12)
      Text(line.text.isEmpty ? " " : line.text)
        .fixedSize(horizontal: false, vertical: true)
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(.trailing, 6)
    }
    .padding(.vertical, 1)
    .background(background)
    .accessibilityLabel(accessibility)
  }

  private var marker: String {
    switch line.kind {
    case .add: "+"
    case .remove: "−"
    // An empty Text has no baseline, which doubles the row's height.
    case .context: " "
    }
  }

  private var markerStyle: AnyShapeStyle {
    switch line.kind {
    case .add: AnyShapeStyle(.green)
    case .remove: AnyShapeStyle(.red)
    case .context: AnyShapeStyle(.secondary)
    }
  }

  private var background: Color {
    switch line.kind {
    case .add: .green.opacity(0.12)
    case .remove: .red.opacity(0.12)
    case .context: .clear
    }
  }

  private var accessibility: String {
    switch line.kind {
    case .add: "Added: \(line.text)"
    case .remove: "Removed: \(line.text)"
    case .context: line.text
    }
  }
}

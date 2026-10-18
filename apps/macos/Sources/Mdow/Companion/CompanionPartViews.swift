import AppKit
import MdowKit
import SwiftUI

// MARK: Reply text

/// Assistant markdown, drawn with SwiftUI text: paragraphs and headings in Charter with inline
/// styling, and code blocks in Geist Mono on a card.
struct ReplyText: View {
  var markdown: String

  var body: some View {
    VStack(alignment: .leading, spacing: 9) {
      ForEach(Array(ReplyBlocks.split(markdown).enumerated()), id: \.offset) { _, block in
        switch block {
        case .paragraph(let text):
          VStack(alignment: .leading, spacing: 5) {
            ForEach(Array(ReplyLines.split(text).enumerated()), id: \.offset) { _, line in
              switch line {
              case .text(let text):
                Text(Self.inline(text)).fixedSize(horizontal: false, vertical: true)
              case .item(let marker, let depth, let text):
                HStack(alignment: .firstTextBaseline, spacing: 0) {
                  Text(marker)
                    .foregroundStyle(Color.mdowSecondary)
                    .frame(width: 18, alignment: .leading)
                  Text(Self.inline(text)).fixedSize(horizontal: false, vertical: true)
                }
                .padding(.leading, CGFloat(depth) * 16)
              }
            }
          }
          .font(CompanionStyle.serif(14.5))
          .lineSpacing(3.5)
        case .heading(let level, let text):
          Text(Self.inline(text))
            .font(level <= 2 ? CompanionStyle.serif(16, .semibold) : .mdowUI(13, .semibold))
            .fixedSize(horizontal: false, vertical: true)
            .padding(.top, 4)
        case .code(_, let text):
          ScrollView(.horizontal, showsIndicators: false) {
            Text(text)
              .font(.mdowMono(11.5))
              .lineSpacing(2)
              .fixedSize(horizontal: true, vertical: true)
              .padding(.horizontal, 11)
              .padding(.vertical, 9)
          }
          .frame(maxWidth: .infinity, alignment: .leading)
          .companionCard(radius: 8)
        }
      }
    }
    .foregroundStyle(Color.mdowText)
    .textSelection(.enabled)
  }

  static func inline(_ text: String) -> AttributedString {
    let options = AttributedString.MarkdownParsingOptions(
      interpretedSyntax: .inlineOnlyPreservingWhitespace, failurePolicy: .returnPartiallyParsedIfPossible)
    var result = (try? AttributedString(markdown: text, options: options)) ?? AttributedString(text)
    // Inline code in Geist Mono, a touch smaller so it sits level with the serif.
    for run in result.runs where run.inlinePresentationIntent?.contains(.code) == true {
      result[run.range].font = .mdowMono(12.5)
      result[run.range].foregroundColor = .mdowAccent
    }
    return result
  }
}

/// A paragraph's lines, with list items pulled out so they can hang their wrapped lines.
enum ReplyLines: Equatable {
  case text(String)
  case item(marker: String, depth: Int, text: String)

  static func split(_ paragraph: String) -> [ReplyLines] {
    var result: [ReplyLines] = []
    for line in paragraph.split(separator: "\n", omittingEmptySubsequences: false) {
      let indent = line.prefix { $0 == " " }.count
      let rest = line.dropFirst(indent)
      if let match = rest.firstMatch(of: /^([-*+]|\d+[.)])\s+/) {
        let marker = String(match.output.1)
        let bullet = marker.first.map { "-*+".contains($0) } ?? false
        result.append(
          .item(
            marker: bullet ? "•" : marker, depth: indent / 2,
            text: String(rest[match.range.upperBound...])))
      } else if case .item(let marker, let depth, let text)? = result.last, indent > 0 {
        // An indented line under an item continues it.
        result[result.count - 1] = .item(
          marker: marker, depth: depth,
          text: text + " " + rest.trimmingCharacters(in: .whitespaces))
      } else if case .text(let text)? = result.last {
        result[result.count - 1] = .text(text + "\n" + line)
      } else {
        result.append(.text(String(line)))
      }
    }
    return result
  }
}

// MARK: Reasoning

struct ReasoningView: View {
  var text: String
  var active: Bool
  @State private var expanded = false

  var body: some View {
    CompanionDisclosure(expanded: $expanded) {
      Text(text.trimmingCharacters(in: .whitespacesAndNewlines))
        .font(.mdowUI(12.5))
        .lineSpacing(2)
        .foregroundStyle(Color.mdowSecondary)
        .textSelection(.enabled)
        .fixedSize(horizontal: false, vertical: true)
        .padding(.leading, 10)
        .frame(maxWidth: .infinity, alignment: .leading)
        .overlay(alignment: .leading) {
          Rectangle().fill(Color.mdowStrongLine).frame(width: 1.5)
        }
        .padding(.leading, 6)
    } label: {
      HStack(spacing: 7) {
        Image(systemName: "brain")
          .font(.system(size: 11, weight: .medium))
          .foregroundStyle(Color.mdowTertiary)
          .frame(width: 14)
        Text(active ? "Thinking…" : "Thought")
          .font(.mdowUI(12))
          .foregroundStyle(Color.mdowSecondary)
        if active { CompanionActivityDots() }
      }
    }
  }
}

// MARK: Tool calls

struct ToolCallView: View {
  var tool: CompanionToolPart
  @State private var expanded = false

  var body: some View {
    CompanionDisclosure(expanded: $expanded) {
      VStack(alignment: .leading, spacing: 6) {
        if let input = tool.input, !input.isEmpty { DetailBox(title: "Input", text: input) }
        if let error = tool.error {
          DetailBox(title: "Error", text: error, isError: true)
        } else if let output = tool.output, !output.isEmpty {
          DetailBox(title: "Output", text: output)
        }
      }
    } label: {
      HStack(spacing: 7) {
        stateIcon
          .frame(width: 14)
        Text(tool.name)
          .font(.mdowMono(11.5))
          .foregroundStyle(tool.state == .error ? Color.companionRemoved : Color.mdowSecondary)
          .lineLimit(1)
          .truncationMode(.middle)
      }
      .help(tool.name)
    }
  }

  @ViewBuilder private var stateIcon: some View {
    switch tool.state {
    case .pending, .running:
      ProgressView().controlSize(.mini)
    case .completed:
      Image(systemName: "checkmark")
        .font(.system(size: 9.5, weight: .bold))
        .foregroundStyle(Color.mdowTertiary)
    case .error:
      Image(systemName: "exclamationmark")
        .font(.system(size: 10, weight: .bold))
        .foregroundStyle(Color.companionRemoved)
    case .cancelled:
      Image(systemName: "minus")
        .font(.system(size: 9.5, weight: .bold))
        .foregroundStyle(Color.mdowTertiary)
    }
  }
}

private struct DetailBox: View {
  var title: String
  var text: String
  var isError = false

  var body: some View {
    VStack(alignment: .leading, spacing: 0) {
      Text(title.lowercased())
        .font(.mdowMono(10, .medium))
        .foregroundStyle(Color.mdowTertiary)
        .padding(.horizontal, 9)
        .padding(.top, 6)
      ScrollView([.vertical, .horizontal]) {
        Text(text)
          .font(.mdowMono(11))
          .lineSpacing(1.5)
          .foregroundStyle(isError ? Color.companionRemoved : Color.mdowText)
          .textSelection(.enabled)
          .fixedSize()
          .padding(.horizontal, 9)
          .padding(.top, 3)
          .padding(.bottom, 7)
          .frame(maxWidth: .infinity, alignment: .leading)
      }
      .frame(maxHeight: 160)
      .fixedSize(horizontal: false, vertical: true)
    }
    .companionCard(radius: 8, fill: .mdowWell)
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
      HStack(spacing: 7) {
        Image(systemName: added ? "doc.badge.plus" : "pencil.line")
          .font(.system(size: 11, weight: .medium))
          .foregroundStyle(Color.mdowTertiary)
        Text(title)
          .font(.mdowMono(11.5, .medium))
          .lineLimit(1)
          .truncationMode(.middle)
          .help(title)
        Spacer(minLength: 4)
        if additions > 0 || deletions > 0 {
          HStack(spacing: 5) {
            Text("+\(additions)").foregroundStyle(Color.companionAdded)
            Text("−\(deletions)").foregroundStyle(Color.companionRemoved)
          }
          .font(.mdowMono(11))
        }
        StatusPill(status: change.status, applying: change.applying)
      }
      .padding(.horizontal, 10)
      .frame(height: 34)

      ForEach(change.files, id: \.path) { file in
        Hairline()
        if change.files.count > 1 {
          Text(file.displayPath)
            .font(.mdowMono(10.5))
            .foregroundStyle(Color.mdowTertiary)
            .lineLimit(1)
            .truncationMode(.middle)
            .padding(.horizontal, 10)
            .padding(.vertical, 4)
        }
        DiffView(file: file, collapsedByDefault: settled)
          .background(Color.mdowWell)
      }

      if let error = change.error, change.status == .failed {
        Hairline()
        Text(error)
          .font(.mdowUI(11.5))
          .foregroundStyle(Color.companionRemoved)
          .fixedSize(horizontal: false, vertical: true)
          .padding(10)
      }

      if change.awaitingReview || (change.status == .applied && !change.files.isEmpty) {
        Hairline()
        HStack(spacing: 6) {
          if change.files.count == 1, change.files[0].status != .deleted {
            Button("Open Document") {
              openDocument(URL(fileURLWithPath: change.files[0].path))
            }
            .buttonStyle(CompanionPillButtonStyle(kind: .quiet))
            .fixedSize()
            .padding(.leading, -4)
          }
          Spacer(minLength: 0)
          if let permissionId = change.permissionId, change.awaitingReview {
            Button("Decline") { session.review(permissionId, .reject) }
              .buttonStyle(CompanionPillButtonStyle(kind: .secondary))
            Button {
              session.review(permissionId, .approve)
            } label: {
              HStack(spacing: 4) {
                Image(systemName: "checkmark").font(.system(size: 9.5, weight: .bold))
                Text("Accept")
              }
            }
            .buttonStyle(CompanionPillButtonStyle(kind: .primary))
            .accessibilityLabel("Accept")
          }
        }
        .padding(.horizontal, 8)
        .padding(.vertical, 7)
      }
    }
    .background(Color.mdowPaper)
    .clipShape(.rect(cornerRadius: 10, style: .continuous))
    .overlay(
      RoundedRectangle(cornerRadius: 10, style: .continuous)
        .strokeBorder(
          change.awaitingReview ? Color.mdowAccent.opacity(0.55) : Color.mdowLine, lineWidth: 1)
    )
    .opacity(settled ? 0.7 : 1)
    .accessibilityElement(children: .contain)
    .accessibilityLabel("Change to \(title)")
  }
}

private struct Hairline: View {
  var body: some View {
    Rectangle().fill(Color.mdowLine).frame(height: 1)
  }
}

private struct StatusPill: View {
  var status: CompanionChangeStatus
  var applying: Bool

  var body: some View {
    if applying {
      HStack(spacing: 5) {
        ProgressView().controlSize(.mini)
        Text("Applying")
      }
      .font(.mdowUI(11, .medium))
      .foregroundStyle(Color.mdowSecondary)
    } else {
      switch status {
      case .pending: pill("Review", dot: true, color: .mdowAccent)
      case .applied: pill("Applied", symbol: "checkmark", color: .companionAdded)
      case .rejected: pill("Declined", symbol: "arrow.uturn.backward", color: .mdowSecondary)
      case .failed: pill("Failed", symbol: "xmark", color: .companionRemoved)
      }
    }
  }

  private func pill(_ title: String, symbol: String? = nil, dot: Bool = false, color: Color)
    -> some View
  {
    HStack(spacing: 4) {
      if dot {
        Circle().fill(color).frame(width: 5, height: 5)
      } else if let symbol {
        Image(systemName: symbol).font(.system(size: 8.5, weight: .bold))
      }
      Text(title)
    }
    .font(.mdowUI(11, .medium))
    .foregroundStyle(color)
    .padding(.horizontal, 7)
    .frame(height: 20)
    .background(color.opacity(0.12), in: .capsule)
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
        .font(.mdowUI(11.5))
        .foregroundStyle(Color.mdowSecondary)
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(10)
    } else {
      VStack(alignment: .leading, spacing: 0) {
        ForEach(Array(visible(hunks, showAll: showAll).enumerated()), id: \.offset) { index, hunk in
          if index > 0 {
            Text("⋯")
              .foregroundStyle(Color.mdowTertiary)
              .frame(maxWidth: .infinity, alignment: .leading)
              .padding(.leading, 34)
              .padding(.vertical, 1)
          }
          ForEach(Array(hunk.lines.enumerated()), id: \.offset) { _, line in
            DiffRow(line: line)
          }
        }
        if total > previewLines {
          ShowMoreButton(showAll: showAll, total: total) { expanded = !showAll }
        }
      }
      .padding(.vertical, 4)
      .font(.mdowMono(11))
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

private struct ShowMoreButton: View {
  var showAll: Bool
  var total: Int
  var toggle: () -> Void
  @State private var hovering = false

  var body: some View {
    Button(action: toggle) {
      HStack(spacing: 4) {
        Image(systemName: showAll ? "chevron.up" : "chevron.down")
          .font(.system(size: 8.5, weight: .bold))
        Text(showAll ? "Show Less" : "Show All \(total) Lines")
      }
      .font(.mdowUI(11, .medium))
      .foregroundStyle(hovering ? Color.mdowText : Color.mdowSecondary)
      .frame(maxWidth: .infinity)
      .frame(height: 24)
      .contentShape(.rect)
    }
    .buttonStyle(.plain)
    .onHover { hovering = $0 }
  }
}

private struct DiffRow: View {
  var line: DiffLine

  var body: some View {
    let number = line.kind == .remove ? line.oldLine : line.newLine
    HStack(alignment: .firstTextBaseline, spacing: 0) {
      Text(number.map(String.init) ?? "")
        .foregroundStyle(Color.mdowTertiary)
        .frame(width: 28, alignment: .trailing)
        .padding(.trailing, 6)
      Text(marker)
        .foregroundStyle(markerStyle)
        .frame(width: 12, alignment: .leading)
      Text(line.text.isEmpty ? " " : line.text)
        .foregroundStyle(line.kind == .context ? Color.mdowSecondary : Color.mdowText)
        .fixedSize(horizontal: false, vertical: true)
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(.trailing, 8)
    }
    .padding(.vertical, 1.5)
    .background(background)
    .overlay(alignment: .leading) {
      if line.kind != .context {
        Rectangle().fill(markerStyle).frame(width: 2)
      }
    }
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

  private var markerStyle: Color {
    switch line.kind {
    case .add: .companionAdded
    case .remove: .companionRemoved
    case .context: .mdowTertiary
    }
  }

  private var background: Color {
    switch line.kind {
    case .add: .companionAdded.opacity(0.11)
    case .remove: .companionRemoved.opacity(0.10)
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

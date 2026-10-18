import AppKit
import MdowKit
import SwiftUI

struct CompanionMessagesView: View {
  var session: CompanionSession
  var document: CompanionDocumentState
  var openDocument: (URL) -> Void

  var body: some View {
    let messages = session.conversation.messages
    if messages.isEmpty {
      CompanionEmptyState(session: session, document: document)
    } else {
      ScrollView {
        LazyVStack(alignment: .leading, spacing: 18) {
          ForEach(messages) { message in
            if message.role == .user {
              UserMessageView(message: message)
            } else {
              AssistantMessageView(message: message, openDocument: openDocument)
            }
          }
          if messages.last?.status == .error,
            let lastUser = messages.last(where: { $0.role == .user })
          {
            TurnErrorView(message: session.conversation.error) { session.send(lastUser.content) }
          }
          ForEach(Array(session.conversation.warnings.enumerated()), id: \.offset) { _, warning in
            HStack(alignment: .firstTextBaseline, spacing: 6) {
              Image(systemName: "exclamationmark.triangle")
              Text(warning).fixedSize(horizontal: false, vertical: true)
            }
            .font(.mdowUI(11.5))
            .foregroundStyle(Color.mdowTertiary)
          }
        }
        .padding(.leading, CompanionStyle.leading + 2)
        .padding(.trailing, CompanionStyle.trailing + 2)
        .padding(.top, 6)
        .padding(.bottom, 16)
        .frame(maxWidth: .infinity, alignment: .leading)
      }
      .scrollIndicators(.automatic)
      // Short conversations start at the top; long ones open at, and follow, the newest reply.
      .defaultScrollAnchor(.top, for: .alignment)
      .defaultScrollAnchor(.bottom, for: .initialOffset)
      .defaultScrollAnchor(.bottom, for: .sizeChanges)
    }
  }
}

// MARK: Empty state

private struct Suggestion: Identifiable {
  var symbol: String
  var label: String
  var prompt: String
  var id: String { label }
}

private func suggestions(for documentName: String?) -> [Suggestion] {
  if let name = documentName {
    return [
      Suggestion(
        symbol: "list.bullet", label: "Summarize this document",
        prompt: "Summarize \(name) in a few bullet points."),
      Suggestion(
        symbol: "lightbulb", label: "Explain the key ideas",
        prompt: "Explain the key ideas in \(name) as if I'm new to the topic."),
      Suggestion(
        symbol: "text.badge.checkmark", label: "Fix spelling and grammar",
        prompt: "Fix spelling and grammar mistakes in \(name). Don't change the meaning or style."),
      Suggestion(
        symbol: "scissors", label: "Make it more concise",
        prompt: "Tighten the writing in \(name): cut repetition and filler, keep every point."),
    ]
  }
  return [
    Suggestion(
      symbol: "folder", label: "What’s in this folder?",
      prompt: "Give me a quick tour of the documents in this folder."),
    Suggestion(
      symbol: "questionmark.text.page", label: "Find open questions",
      prompt: "Find open questions and TODOs across these documents."),
  ]
}

private struct CompanionEmptyState: View {
  var session: CompanionSession
  var document: CompanionDocumentState

  var body: some View {
    let documentName = document.documentURL?.lastPathComponent
    let scope = documentName ?? document.folderURL?.lastPathComponent
    let items = suggestions(for: documentName)
    ScrollView {
      VStack(alignment: .leading, spacing: 0) {
        Image(systemName: "sparkle")
          .font(.system(size: 17, weight: .regular))
          .foregroundStyle(Color.mdowAccent)
          .padding(.bottom, 12)
          .accessibilityHidden(true)
        Text(scope.map { "Ask about \($0)" } ?? "Ask about your documents")
          .font(CompanionStyle.serif(22))
          .fixedSize(horizontal: false, vertical: true)
        Text("Get answers, summaries and edits. Every change comes back as a diff for you to approve.")
          .font(.mdowUI(12.5))
          .foregroundStyle(Color.mdowSecondary)
          .lineSpacing(2)
          .fixedSize(horizontal: false, vertical: true)
          .padding(.top, 6)
        Grid(horizontalSpacing: 8, verticalSpacing: 8) {
          ForEach(Array(stride(from: 0, to: items.count, by: 2)), id: \.self) { start in
            GridRow {
              ForEach(items[start..<min(start + 2, items.count)]) { suggestion in
                SuggestionButton(suggestion: suggestion) { session.send(suggestion.prompt) }
              }
            }
          }
        }
        .fixedSize(horizontal: false, vertical: true)
        .padding(.top, 20)
      }
      .padding(.leading, CompanionStyle.leading + 2)
      .padding(.trailing, CompanionStyle.trailing + 2)
      .padding(.vertical, 20)
      .frame(maxWidth: .infinity, alignment: .leading)
      .containerRelativeFrame(.vertical, alignment: .center) { length, _ in length }
    }
    .scrollIndicators(.never)
  }
}

private struct SuggestionButton: View {
  var suggestion: Suggestion
  var action: () -> Void
  @State private var hovering = false

  var body: some View {
    Button(action: action) {
      VStack(alignment: .leading, spacing: 8) {
        Image(systemName: suggestion.symbol)
          .font(.system(size: 11.5, weight: .medium))
          .foregroundStyle(hovering ? Color.mdowAccent : Color.mdowTertiary)
          .frame(height: 14)
        Text(suggestion.label)
          .font(.mdowUI(12.5, .medium))
          .foregroundStyle(Color.mdowText)
          .multilineTextAlignment(.leading)
          .fixedSize(horizontal: false, vertical: true)
        Spacer(minLength: 0)
      }
      .padding(.horizontal, 11)
      .padding(.vertical, 10)
      .frame(maxWidth: .infinity, minHeight: 68, maxHeight: .infinity, alignment: .topLeading)
      .background(
        hovering ? Color.mdowHover : .clear, in: .rect(cornerRadius: 10, style: .continuous))
      .companionCard(radius: 10, fill: .mdowPaper, border: hovering ? .mdowStrongLine : .mdowLine)
      .contentShape(.rect(cornerRadius: 10))
    }
    .buttonStyle(.plain)
    .onHover { hovering = $0 }
    .accessibilityLabel(suggestion.label)
  }
}

// MARK: Messages

private struct UserMessageView: View {
  var message: CompanionMessage

  var body: some View {
    HStack {
      Spacer(minLength: 36)
      Text(message.content)
        .font(.mdowUI(13))
        .lineSpacing(2)
        .textSelection(.enabled)
        .fixedSize(horizontal: false, vertical: true)
        .padding(.horizontal, 12)
        .padding(.vertical, 8)
        .companionCard(radius: 14, fill: .mdowElevated)
    }
  }
}

private struct AssistantMessageView: View {
  var message: CompanionMessage
  var openDocument: (URL) -> Void
  @State private var hovering = false

  var body: some View {
    let streaming = message.status == .streaming
    VStack(alignment: .leading, spacing: 10) {
      ForEach(Array(message.parts.enumerated()), id: \.offset) { _, part in
        switch part {
        case .text(let text):
          ReplyText(markdown: text)
        case .thinking(let text, let done):
          ReasoningView(text: text, active: !done && streaming)
        case .tool(let tool):
          ToolCallView(tool: tool)
        case .change(let change):
          ChangeCardView(change: change, openDocument: openDocument)
        case .status(let text):
          Text(text).font(.mdowUI(11.5)).foregroundStyle(Color.mdowTertiary)
        }
      }
      if streaming, isWaiting {
        HStack(spacing: 8) {
          CompanionActivityDots()
          Text(message.parts.isEmpty ? "Thinking…" : "Working…")
        }
        .font(.mdowUI(12))
        .foregroundStyle(Color.mdowSecondary)
        .frame(height: 20)
      }
      if message.status == .cancelled {
        Text("Stopped").font(.mdowUI(11.5)).foregroundStyle(Color.mdowTertiary)
      }
      if !message.citations.isEmpty {
        SourcesView(citations: message.citations, openDocument: openDocument)
      }
      if message.status == .complete, !message.content.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
        CopyButton(text: message.content)
          .padding(.leading, -5)
          .padding(.top, -4)
          .opacity(hovering ? 1 : 0)
      }
    }
    .frame(maxWidth: .infinity, alignment: .leading)
    .onHover { hovering = $0 }
  }

  /// Nothing is streaming in right now, so show that OpenCode is still working.
  private var isWaiting: Bool {
    switch message.parts.last {
    case nil: true
    case .tool(let tool): tool.state == .completed
    case .thinking(_, let done): done
    case .change(let change): change.status == .applied
    default: false
    }
  }
}

private struct CopyButton: View {
  var text: String
  @State private var copied = false

  var body: some View {
    Button {
      NSPasteboard.general.clearContents()
      NSPasteboard.general.setString(text, forType: .string)
      copied = true
      Task {
        try? await Task.sleep(for: .seconds(1.5))
        copied = false
      }
    } label: {
      Image(systemName: copied ? "checkmark" : "doc.on.doc")
        .font(.system(size: 11, weight: .medium))
    }
    .buttonStyle(CompanionIconButtonStyle(side: 24))
    .help(copied ? "Copied" : "Copy Response")
    .accessibilityLabel(copied ? "Copied" : "Copy Response")
  }
}

private struct TurnErrorView: View {
  var message: String?
  var retry: () -> Void

  var body: some View {
    HStack(alignment: .firstTextBaseline, spacing: 8) {
      Image(systemName: "exclamationmark.circle")
        .font(.system(size: 12, weight: .medium))
        .foregroundStyle(Color.companionRemoved)
      Text(message ?? "Something went wrong.")
        .font(.mdowUI(12.5))
        .lineSpacing(2)
        .textSelection(.enabled)
        .fixedSize(horizontal: false, vertical: true)
        .frame(maxWidth: .infinity, alignment: .leading)
      Button("Retry", action: retry)
        .buttonStyle(CompanionPillButtonStyle(kind: .secondary))
        .alignmentGuide(.firstTextBaseline) { $0[VerticalAlignment.center] + 4 }
    }
    .padding(.horizontal, 12)
    .padding(.vertical, 10)
    .companionCard(
      radius: 10, fill: .companionRemoved.opacity(0.07), border: .companionRemoved.opacity(0.25))
  }
}

private struct SourcesView: View {
  var citations: [CompanionCitation]
  var openDocument: (URL) -> Void
  @State private var expanded = false

  var body: some View {
    CompanionDisclosure(expanded: $expanded) {
      VStack(alignment: .leading, spacing: 2) {
        ForEach(citations, id: \.sourceId) { citation in
          SourceLink(citation: citation) { openDocument(URL(fileURLWithPath: citation.path)) }
        }
      }
    } label: {
      Text(citations.count == 1 ? "Used 1 source" : "Used \(citations.count) sources")
        .font(.mdowUI(11.5))
        .foregroundStyle(Color.mdowTertiary)
    }
  }
}

private struct SourceLink: View {
  var citation: CompanionCitation
  var open: () -> Void
  @State private var hovering = false

  var body: some View {
    Button(action: open) {
      HStack(spacing: 6) {
        Image(systemName: "doc.text")
          .font(.system(size: 10.5))
          .foregroundStyle(Color.mdowTertiary)
        Text(citation.label)
          .font(.mdowMono(11))
          .foregroundStyle(hovering ? Color.mdowAccent : Color.mdowSecondary)
          .lineLimit(1)
          .truncationMode(.middle)
      }
      .contentShape(.rect)
    }
    .buttonStyle(.plain)
    .onHover { hovering = $0 }
    .help(citation.path)
  }
}

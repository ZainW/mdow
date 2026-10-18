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
        LazyVStack(alignment: .leading, spacing: 16) {
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
            Label(warning, systemImage: "exclamationmark.triangle")
              .font(.caption)
              .foregroundStyle(.secondary)
          }
        }
        .padding(14)
        .frame(maxWidth: .infinity, alignment: .leading)
      }
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
        symbol: "list.bullet.rectangle", label: "Summarize this document",
        prompt: "Summarize \(name) in a few bullet points."),
      Suggestion(
        symbol: "lightbulb", label: "Explain the key ideas",
        prompt: "Explain the key ideas in \(name) as if I'm new to the topic."),
      Suggestion(
        symbol: "textformat.abc.dottedunderline", label: "Fix spelling and grammar",
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
    ScrollView {
      VStack(spacing: 6) {
        Image(systemName: "sparkles")
          .font(.system(size: 22, weight: .regular))
          .foregroundStyle(.secondary)
          .frame(width: 44, height: 44)
          .background(.quaternary.opacity(0.6), in: .rect(cornerRadius: 10))
          .padding(.bottom, 6)
        Text(scope.map { "Ask about \($0)" } ?? "Ask about your documents")
          .font(.headline)
          .multilineTextAlignment(.center)
        Text("Get answers, summaries and edits. Every change comes back as a diff for you to approve.")
          .font(.callout)
          .foregroundStyle(.secondary)
          .multilineTextAlignment(.center)
          .fixedSize(horizontal: false, vertical: true)
          .frame(maxWidth: 260)
        VStack(alignment: .leading, spacing: 2) {
          ForEach(suggestions(for: documentName)) { suggestion in
            SuggestionButton(suggestion: suggestion) { session.send(suggestion.prompt) }
          }
        }
        .frame(maxWidth: 280)
        .padding(.top, 14)
      }
      .padding(20)
      .frame(maxWidth: .infinity)
      .containerRelativeFrame(.vertical, alignment: .center) { length, _ in length }
    }
  }
}

private struct SuggestionButton: View {
  var suggestion: Suggestion
  var action: () -> Void
  @State private var hovering = false

  var body: some View {
    Button(action: action) {
      Label {
        Text(suggestion.label).foregroundStyle(.primary)
      } icon: {
        Image(systemName: suggestion.symbol)
          .foregroundStyle(.secondary)
          .frame(width: 18)
      }
      .frame(maxWidth: .infinity, alignment: .leading)
      .padding(.horizontal, 10)
      .padding(.vertical, 6)
      .contentShape(.rect)
      .background(hovering ? AnyShapeStyle(.quaternary) : AnyShapeStyle(.clear), in: .rect(cornerRadius: 6))
    }
    .buttonStyle(.plain)
    .onHover { hovering = $0 }
  }
}

// MARK: Messages

private struct UserMessageView: View {
  var message: CompanionMessage

  var body: some View {
    HStack {
      Spacer(minLength: 32)
      Text(message.content)
        .textSelection(.enabled)
        .padding(.horizontal, 11)
        .padding(.vertical, 7)
        .background(.quaternary.opacity(0.8), in: .rect(cornerRadius: 12))
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
          Text(text).font(.caption).foregroundStyle(.secondary)
        }
      }
      if streaming, isWaiting {
        HStack(spacing: 6) {
          ProgressView().controlSize(.mini)
          Text(message.parts.isEmpty ? "Thinking…" : "Working…")
        }
        .font(.caption)
        .foregroundStyle(.secondary)
      }
      if message.status == .cancelled {
        Text("Stopped").font(.caption).foregroundStyle(.secondary)
      }
      if !message.citations.isEmpty {
        SourcesView(citations: message.citations, openDocument: openDocument)
      }
      if message.status == .complete, !message.content.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
        CopyButton(text: message.content)
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
        .foregroundStyle(copied ? AnyShapeStyle(.green) : AnyShapeStyle(.secondary))
    }
    .buttonStyle(.borderless)
    .controlSize(.small)
    .help(copied ? "Copied" : "Copy Response")
    .accessibilityLabel(copied ? "Copied" : "Copy Response")
  }
}

private struct TurnErrorView: View {
  var message: String?
  var retry: () -> Void

  var body: some View {
    HStack(alignment: .firstTextBaseline, spacing: 8) {
      Image(systemName: "exclamationmark.triangle.fill")
        .foregroundStyle(.red)
      Text(message ?? "Something went wrong.")
        .font(.callout)
        .textSelection(.enabled)
        .fixedSize(horizontal: false, vertical: true)
        .frame(maxWidth: .infinity, alignment: .leading)
      Button("Retry", action: retry)
        .controlSize(.small)
    }
    .padding(10)
    .background(.red.opacity(0.08), in: .rect(cornerRadius: 8))
    .overlay(RoundedRectangle(cornerRadius: 8).strokeBorder(.red.opacity(0.25)))
  }
}

private struct SourcesView: View {
  var citations: [CompanionCitation]
  var openDocument: (URL) -> Void
  @State private var expanded = false

  var body: some View {
    DisclosureGroup(isExpanded: $expanded) {
      VStack(alignment: .leading, spacing: 4) {
        ForEach(citations, id: \.sourceId) { citation in
          Button {
            openDocument(URL(fileURLWithPath: citation.path))
          } label: {
            Label(citation.label, systemImage: "doc.text")
          }
          .buttonStyle(.link)
          .help(citation.path)
        }
      }
      .padding(.top, 4)
    } label: {
      Text(citations.count == 1 ? "Used 1 source" : "Used \(citations.count) sources")
        .foregroundStyle(.secondary)
    }
    .font(.caption)
  }
}

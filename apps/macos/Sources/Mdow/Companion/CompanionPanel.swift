import AppKit
import MdowKit
import SwiftUI

/// The companion inspector's content: setup states, the conversation and the composer.
struct CompanionPanel: View {
  var session: CompanionSession
  var runtime: CompanionRuntime
  var document: CompanionDocumentState
  var openDocument: (URL) -> Void

  var body: some View {
    let ready = runtime.isAvailable
    let models = runtime.modelState
    let noModels = ready && !models.stale && models.options.isEmpty
    VStack(spacing: 0) {
      CompanionHeader(session: session, runtime: runtime)
      if !ready {
        CompanionSetupView(status: runtime.status) { reconnect() }
      } else if noModels && session.conversation.messages.isEmpty {
        SetupMessage(
          title: "Connect a Model Provider",
          message: models.unavailableReason ?? "OpenCode has no models available yet.",
          commands: ["opencode auth login"]
        ) { reconnect() }
      } else {
        CompanionMessagesView(session: session, document: document, openDocument: openDocument)
        if let change = session.conversation.pendingChange {
          PendingReviewBar(change: change, document: document, session: session, openDocument: openDocument)
        }
        CompanionComposer(session: session, runtime: runtime, document: document)
      }
    }
    .frame(maxWidth: .infinity, maxHeight: .infinity)
    .background(Color.mdowChrome)
    .foregroundStyle(Color.mdowText)
    .tint(.mdowAccent)
    .environment(session)
  }

  private func reconnect() {
    Task { await runtime.connect(directory: session.directory) }
  }
}

// MARK: Header

private struct CompanionHeader: View {
  var session: CompanionSession
  var runtime: CompanionRuntime

  var body: some View {
    HStack(spacing: 8) {
      Text("Companion")
        .font(.mdowUI(13, .semibold))
      if let version = runtime.status?.version, runtime.isAvailable {
        Text("OpenCode \(version)")
          .font(.mdowMono(11))
          .foregroundStyle(Color.mdowTertiary)
          .lineLimit(1)
          .offset(y: 0.5)
      }
      Spacer(minLength: 0)
      Button {
        session.newChat()
      } label: {
        Image(systemName: "square.and.pencil")
      }
      .buttonStyle(CompanionIconButtonStyle(side: 28))
      .help("New Chat")
      .accessibilityLabel("New Chat")
      .disabled(session.conversation.messages.isEmpty)
    }
    .padding(.leading, CompanionStyle.leading + 2)
    .padding(.trailing, CompanionStyle.trailing)
    .frame(height: 40)
  }
}

// MARK: Setup

private let installCommand = "curl -fsSL https://opencode.ai/install | bash"

private struct CompanionSetupView: View {
  var status: CompanionRuntimeStatus?
  var retry: () -> Void

  var body: some View {
    switch status?.availability {
    case nil:
      HStack(spacing: 8) {
        CompanionActivityDots()
        Text("Starting OpenCode…")
          .font(.mdowUI(12.5))
          .foregroundStyle(Color.mdowSecondary)
      }
      .frame(maxWidth: .infinity, maxHeight: .infinity)
    case .missing:
      SetupMessage(
        title: "Connect OpenCode",
        message:
          "The companion runs on OpenCode, using the models and subscriptions you already use there. Install OpenCode 2, then sign in to a provider:",
        commands: [installCommand, "opencode auth login"], showDocs: true, retry: retry)
    case .outdated:
      SetupMessage(
        title: "Update OpenCode",
        message:
          "OpenCode \(status?.version ?? "") is installed, but the companion needs OpenCode 2 or newer.",
        commands: ["opencode upgrade"], showDocs: true, retry: retry)
    case .failed, .available:
      SetupMessage(
        title: "OpenCode Didn’t Start",
        message: status?.detail ?? "Something went wrong while starting OpenCode.", commands: [],
        showDocs: true, retry: retry)
    }
  }
}

private struct SetupMessage: View {
  var title: String
  var message: String
  var commands: [String]
  var showDocs = false
  var retry: () -> Void

  var body: some View {
    ScrollView {
      VStack(alignment: .leading, spacing: 10) {
        Text(title)
          .font(CompanionStyle.serif(19))
          .padding(.bottom, 2)
        Text(message)
          .font(.mdowUI(12.5))
          .foregroundStyle(Color.mdowSecondary)
          .lineSpacing(2)
          .fixedSize(horizontal: false, vertical: true)
        ForEach(commands, id: \.self) { CommandLineView(command: $0) }
        HStack(spacing: 6) {
          Button("Check Again", action: retry)
            .buttonStyle(CompanionPillButtonStyle(kind: .secondary))
          if showDocs {
            Button("OpenCode Docs") {
              NSWorkspace.shared.open(URL(string: "https://opencode.ai/docs")!)
            }
            .buttonStyle(CompanionPillButtonStyle(kind: .quiet))
          }
        }
        .padding(.top, 4)
      }
      .padding(.leading, CompanionStyle.leading + 2)
      .padding(.trailing, CompanionStyle.trailing + 2)
      .padding(.vertical, 16)
      .frame(maxWidth: .infinity, alignment: .leading)
    }
    .scrollIndicators(.never)
    .frame(maxHeight: .infinity)
  }
}

private struct CommandLineView: View {
  var command: String

  var body: some View {
    HStack(spacing: 6) {
      Text("$")
        .foregroundStyle(Color.mdowTertiary)
      Text(command)
        .foregroundStyle(Color.mdowText)
        .textSelection(.enabled)
        .frame(maxWidth: .infinity, alignment: .leading)
      Button {
        NSPasteboard.general.clearContents()
        NSPasteboard.general.setString(command, forType: .string)
      } label: {
        Image(systemName: "doc.on.doc")
      }
      .buttonStyle(CompanionIconButtonStyle(side: 22))
      .help("Copy")
      .accessibilityLabel("Copy")
    }
    .font(.mdowMono(11.5))
    .padding(.leading, 10)
    .padding(.trailing, 4)
    .padding(.vertical, 4)
    .companionCard(radius: 8)
  }
}

// MARK: Pending review

/// Stays above the composer while OpenCode waits for a decision on an edit, wherever the change
/// card has scrolled to. ⌘↩ accepts and ⌘⌫ declines, as in the desktop app's review bar.
private struct PendingReviewBar: View {
  var change: CompanionChangePart
  var document: CompanionDocumentState
  var session: CompanionSession
  var openDocument: (URL) -> Void

  var body: some View {
    let first = change.files.first
    let label: String =
      switch change.files.count {
      case 0: "Suggested edit"
      case 1: first!.displayPath
      default: "\(first!.displayPath) and \(change.files.count - 1) more"
      }
    let elsewhere = first.map { $0.path != document.documentURL?.path && $0.status != .deleted } ?? false
    HStack(spacing: 8) {
      Circle()
        .fill(Color.mdowAccent)
        .frame(width: 6, height: 6)
        .padding(.leading, 2)
      VStack(alignment: .leading, spacing: 1) {
        Text("Suggested edit")
          .font(.mdowUI(12.5, .medium))
        Text(label)
          .font(.mdowMono(11))
          .foregroundStyle(Color.mdowTertiary)
          .lineLimit(1)
          .truncationMode(.middle)
      }
      .help(label)
      Spacer(minLength: 4)
      if elsewhere, let first {
        Button("Show") { openDocument(URL(fileURLWithPath: first.path)) }
          .buttonStyle(CompanionPillButtonStyle(kind: .quiet))
          .help("Open \(first.displayPath)")
      }
      if let permissionId = change.permissionId {
        Button("Decline") { session.review(permissionId, .reject) }
          .buttonStyle(CompanionPillButtonStyle(kind: .secondary))
          .keyboardShortcut(session.composerFocused ? nil : KeyboardShortcut(.delete, modifiers: .command))
          .help("Decline (⌘⌫)")
        Button("Accept") { session.review(permissionId, .approve) }
          .buttonStyle(CompanionPillButtonStyle(kind: .primary))
          .keyboardShortcut(.return, modifiers: .command)
          .help("Accept (⌘↩)")
      }
    }
    .padding(.leading, 10)
    .padding(.trailing, 8)
    .padding(.vertical, 8)
    .companionCard(radius: 12, border: .mdowAccent.opacity(0.35))
    .padding(.leading, CompanionStyle.leading)
    .padding(.trailing, CompanionStyle.trailing)
    .padding(.top, 6)
  }
}

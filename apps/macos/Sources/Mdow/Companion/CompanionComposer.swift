import AppKit
import MdowKit
import SwiftUI

struct CompanionComposer: View {
  var session: CompanionSession
  var runtime: CompanionRuntime
  var document: CompanionDocumentState
  @State private var text = ""
  @State private var mentionQuery: String?
  @State private var mentionIndex = 0
  @FocusState private var focused: Bool

  var body: some View {
    let streaming = session.conversation.streaming
    let mentions = mentionResults
    VStack(alignment: .leading, spacing: 0) {
      if mentionQuery != nil, !mentions.isEmpty {
        MentionList(results: mentions, selected: mentionIndex, root: document.folderURL) { pick($0) }
          .padding(.bottom, 6)
      }
      VStack(alignment: .leading, spacing: 8) {
        if document.documentURL != nil || !session.conversation.tags.isEmpty {
          ContextChips(session: session, document: document)
        }
        TextField(
          placeholder, text: $text, prompt: Text(placeholder).foregroundStyle(Color.mdowTertiary),
          axis: .vertical)
          .textFieldStyle(.plain)
          .font(.mdowUI(13))
          .lineSpacing(2)
          .lineLimit(1...8)
          .focused($focused)
          .onChange(of: focused) { _, value in session.composerFocused = value }
          .onChange(of: text) { _, value in updateMention(value) }
          .onKeyPress(keys: [.upArrow, .downArrow, .return, .tab, .escape], phases: .down) { press in
            handleKey(press, mentions: mentions)
          }
          .accessibilityLabel("Message the companion")
          .padding(.horizontal, 2)
        HStack(spacing: 2) {
          Button {
            startMention()
          } label: {
            Image(systemName: "at")
          }
          .buttonStyle(CompanionIconButtonStyle(side: 26))
          .help("Mention a Document")
          .accessibilityLabel("Mention a Document")
          .disabled(document.documents.isEmpty)
          ModelPicker(state: runtime.modelState) { session.selectModel($0) }
          Spacer(minLength: 4)
          if streaming {
            Button {
              session.stop()
            } label: {
              RoundedRectangle(cornerRadius: 2, style: .continuous)
                .fill(Color.mdowPaper)
                .frame(width: 8, height: 8)
                .frame(width: 26, height: 26)
                .background(Color.mdowText, in: .circle)
            }
            .buttonStyle(.plain)
            .help("Stop")
            .accessibilityLabel("Stop")
            .keyboardShortcut(".", modifiers: .command)
          } else {
            let canSend = !text.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
            Button {
              send()
            } label: {
              Image(systemName: "arrow.up")
                .font(.system(size: 12, weight: .bold))
                .foregroundStyle(canSend ? Color.companionOnAccent : Color.mdowTertiary)
                .frame(width: 26, height: 26)
                .background(canSend ? Color.mdowAccent : Color.mdowHover, in: .circle)
            }
            .buttonStyle(.plain)
            .disabled(!canSend)
            .help("Send (↩)")
            .accessibilityLabel("Send")
          }
        }
        .padding(.horizontal, -4)
      }
      .padding(.top, 10)
      .padding(.horizontal, 10)
      .padding(.bottom, 7)
      .background(Color.mdowPaper, in: .rect(cornerRadius: Theme.cardRadius, style: .continuous))
      .overlay(
        RoundedRectangle(cornerRadius: Theme.cardRadius, style: .continuous)
          .strokeBorder(focused ? Color.mdowStrongLine : Color.mdowLine, lineWidth: 1)
      )
      .shadow(color: .black.opacity(0.05), radius: 6, y: 2)
      .contentShape(.rect)
      .onTapGesture { focused = true }
    }
    .padding(.leading, CompanionStyle.leading)
    .padding(.trailing, CompanionStyle.trailing)
    .padding(.bottom, Theme.gutter)
    .padding(.top, 8)
    .onReceive(NotificationCenter.default.publisher(for: .companionFocusComposer)) { note in
      if note.object as AnyObject? === session { focused = true }
    }
  }

  private var placeholder: String {
    if let name = document.documentURL?.lastPathComponent {
      return "Ask about \(name), or ask for an edit…"
    }
    return "Ask about your documents…"
  }

  private func send() {
    let message = text
    guard !message.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty,
      !session.conversation.streaming
    else { return }
    session.send(message)
    text = ""
    mentionQuery = nil
  }

  // MARK: Mentions

  private var mentionResults: [URL] {
    guard let query = mentionQuery else { return [] }
    let current = document.documentURL
    let candidates = document.documents.filter { $0 != current }
    if query.isEmpty { return Array(candidates.prefix(8)) }
    return candidates.compactMap { url -> (URL, Int)? in
      let name = url.lastPathComponent
      let relative = relativePath(url)
      guard let score = max(FuzzyMatch.score(query, in: name) ?? -1, FuzzyMatch.score(query, in: relative) ?? -1)
        as Int?, score >= 0
      else { return nil }
      return (url, score)
    }
    .sorted { $0.1 > $1.1 }
    .prefix(8)
    .map(\.0)
  }

  private func relativePath(_ url: URL) -> String {
    guard let root = document.folderURL?.path, url.path.hasPrefix(root + "/") else {
      return url.lastPathComponent
    }
    return String(url.path.dropFirst(root.count + 1))
  }

  private func updateMention(_ value: String) {
    if let match = value.firstMatch(of: /(?:^|\s)@([^\s@]*)$/) {
      let query = String(match.output.1)
      if mentionQuery != query { mentionIndex = 0 }
      mentionQuery = query
    } else {
      mentionQuery = nil
    }
  }

  private func startMention() {
    if !text.isEmpty, !text.hasSuffix(" ") { text += " " }
    text += "@"
    mentionQuery = ""
    mentionIndex = 0
    focused = true
  }

  private func pick(_ url: URL) {
    session.addTag(url)
    if let range = text.range(of: #"@[^\s@]*$"#, options: .regularExpression) {
      text.removeSubrange(range)
    }
    mentionQuery = nil
    focused = true
  }

  private func handleKey(_ press: KeyPress, mentions: [URL]) -> KeyPress.Result {
    if mentionQuery != nil, !mentions.isEmpty {
      switch press.key {
      case .downArrow:
        mentionIndex = (mentionIndex + 1) % mentions.count
        return .handled
      case .upArrow:
        mentionIndex = mentionIndex <= 0 ? mentions.count - 1 : mentionIndex - 1
        return .handled
      case .return, .tab:
        pick(mentions[min(mentionIndex, mentions.count - 1)])
        return .handled
      case .escape:
        mentionQuery = nil
        return .handled
      default:
        break
      }
    }
    if press.key == .return {
      // Shift- or Option-Return adds a line; Return sends.
      if press.modifiers.contains(.shift) || press.modifiers.contains(.option) {
        text += "\n"
        return .handled
      }
      send()
      return .handled
    }
    return .ignored
  }
}

private struct MentionList: View {
  var results: [URL]
  var selected: Int
  var root: URL?
  var pick: (URL) -> Void

  var body: some View {
    VStack(alignment: .leading, spacing: 1) {
      ForEach(Array(results.enumerated()), id: \.element) { index, url in
        MentionRow(
          name: url.lastPathComponent, folder: folder(of: url), selected: index == selected
        ) { pick(url) }
      }
    }
    .padding(4)
    .background(Color.mdowElevated, in: .rect(cornerRadius: 10, style: .continuous))
    .overlay(
      RoundedRectangle(cornerRadius: 10, style: .continuous)
        .strokeBorder(Color.mdowLine, lineWidth: 1)
    )
    .shadow(color: .black.opacity(0.1), radius: 10, y: 3)
  }

  private func folder(of url: URL) -> String {
    let parent = url.deletingLastPathComponent().path
    guard let root = root?.path else { return "" }
    if parent == root { return "" }
    return parent.hasPrefix(root + "/") ? String(parent.dropFirst(root.count + 1)) : parent
  }
}

private struct MentionRow: View {
  var name: String
  var folder: String
  var selected: Bool
  var pick: () -> Void
  @State private var hovering = false

  var body: some View {
    Button(action: pick) {
      HStack(spacing: 7) {
        Image(systemName: "doc.text")
          .font(.system(size: 11))
          .foregroundStyle(Color.mdowTertiary)
        Text(name)
          .font(.mdowUI(12.5))
          .foregroundStyle(Color.mdowText)
          .lineLimit(1)
        Spacer(minLength: 4)
        Text(folder)
          .font(.mdowMono(10.5))
          .foregroundStyle(Color.mdowTertiary)
          .lineLimit(1)
          .truncationMode(.head)
      }
      .padding(.horizontal, 8)
      .frame(height: 26)
      .contentShape(.rect)
      .background(
        selected ? Color.mdowSelected : hovering ? Color.mdowHover : .clear,
        in: .rect(cornerRadius: 6))
    }
    .buttonStyle(.plain)
    .onHover { hovering = $0 }
  }
}

private struct ContextChips: View {
  var session: CompanionSession
  var document: CompanionDocumentState

  var body: some View {
    ScrollView(.horizontal, showsIndicators: false) {
      HStack(spacing: 4) {
        if let url = document.documentURL {
          Chip(symbol: "doc.text", title: url.lastPathComponent, secondary: true)
            .help("OpenCode sees that you are viewing \(url.path)")
        }
        ForEach(session.conversation.tags) { tag in
          Chip(symbol: "at", title: (tag.path as NSString).lastPathComponent) {
            session.removeTag(tag.sourceId)
          }
          .help(tag.path)
        }
      }
    }
  }
}

/// A document the message refers to, as a small mono pill.
private struct Chip: View {
  var symbol: String
  var title: String
  var secondary = false
  var remove: (() -> Void)?

  var body: some View {
    HStack(spacing: 4) {
      Image(systemName: symbol)
        .font(.system(size: 9.5, weight: .medium))
        .foregroundStyle(Color.mdowTertiary)
      Text(title).lineLimit(1).truncationMode(.middle)
      if let remove {
        Button(action: remove) {
          Image(systemName: "xmark")
            .font(.system(size: 8, weight: .bold))
            .foregroundStyle(Color.mdowTertiary)
            .frame(width: 14, height: 14)
            .contentShape(.rect)
        }
        .buttonStyle(.plain)
        .accessibilityLabel("Remove \(title)")
      }
    }
    .font(.mdowMono(11))
    .foregroundStyle(secondary ? Color.mdowSecondary : Color.mdowText)
    .padding(.leading, 7)
    .padding(.trailing, remove == nil ? 8 : 3)
    .frame(height: 20)
    .frame(maxWidth: 180)
    .background(Color.mdowHover, in: .capsule)
    .overlay(Capsule().strokeBorder(Color.mdowLine, lineWidth: 1))
  }
}

// MARK: Model picker

/// The model menu: a searchable popover grouped by provider, as OpenCode can offer hundreds.
struct ModelPicker: View {
  var state: CompanionModelState
  var select: (String) -> Void
  @State private var presented = false
  @State private var hovering = false

  var body: some View {
    let unavailable = state.stale || state.options.isEmpty
    let label = state.current?.name ?? (state.stale ? "Loading models…" : "Choose a model")
    Button {
      presented.toggle()
    } label: {
      HStack(spacing: 4) {
        Text(label).lineLimit(1).truncationMode(.tail)
        Image(systemName: "chevron.down")
          .font(.system(size: 7.5, weight: .bold))
          .foregroundStyle(Color.mdowTertiary)
      }
      .font(.mdowMono(11))
      .foregroundStyle(hovering || presented ? Color.mdowText : Color.mdowSecondary)
      .padding(.horizontal, 8)
      .frame(height: 24)
      .background(
        presented ? Color.mdowSelected : hovering && !unavailable ? Color.mdowHover : .clear,
        in: .capsule)
      .contentShape(.capsule)
    }
    .buttonStyle(.plain)
    .onHover { hovering = $0 }
    .disabled(unavailable)
    .opacity(unavailable ? 0.6 : 1)
    .help(unavailable ? (state.unavailableReason ?? "") : (state.current?.providerName ?? "Choose a model"))
    .accessibilityLabel("Model: \(label)")
    .popover(isPresented: $presented, arrowEdge: .top) {
      ModelList(state: state) { value in
        presented = false
        if value != state.currentValue { select(value) }
      }
    }
  }
}

struct ModelList: View {
  var state: CompanionModelState
  var select: (String) -> Void
  @State private var query = ""
  @FocusState private var searchFocused: Bool

  private var groups: [(provider: String, options: [CompanionModelOption])] {
    var order: [String] = []
    var byProvider: [String: [CompanionModelOption]] = [:]
    for option in state.options {
      if !query.isEmpty, FuzzyMatch.score(query, in: "\(option.name) \(option.providerName)") == nil {
        continue
      }
      if byProvider[option.providerId] == nil { order.append(option.providerId) }
      byProvider[option.providerId, default: []].append(option)
    }
    return order.map { (byProvider[$0]![0].providerName, byProvider[$0]!) }
  }

  var body: some View {
    VStack(spacing: 0) {
      HStack(spacing: 7) {
        Image(systemName: "magnifyingglass")
          .font(.system(size: 11, weight: .medium))
          .foregroundStyle(Color.mdowTertiary)
        TextField("Search models", text: $query)
          .textFieldStyle(.plain)
          .font(.mdowUI(12.5))
          .focused($searchFocused)
          .onSubmit {
            if let first = groups.first?.options.first { select(first.value) }
          }
      }
      .padding(.horizontal, 9)
      .frame(height: 28)
      .background(Color.mdowHover, in: .rect(cornerRadius: 7))
      .padding(8)
      Rectangle().fill(Color.mdowLine).frame(height: 1)
      let groups = groups
      if groups.isEmpty {
        Text("No matching models.")
          .font(.mdowUI(12.5))
          .foregroundStyle(Color.mdowSecondary)
          .frame(maxWidth: .infinity, maxHeight: .infinity)
      } else {
        ScrollView {
          LazyVStack(alignment: .leading, spacing: 1) {
            ForEach(groups, id: \.provider) { group in
              Text(group.provider)
                .font(.mdowUI(11, .medium))
                .foregroundStyle(Color.mdowTertiary)
                .padding(.horizontal, 8)
                .padding(.top, 10)
                .padding(.bottom, 3)
              ForEach(group.options) { option in
                ModelRow(name: option.name, current: option.value == state.currentValue) {
                  select(option.value)
                }
              }
            }
          }
          .padding(.horizontal, 6)
          .padding(.bottom, 6)
        }
      }
    }
    .frame(width: 300, height: 380)
    .background(Color.mdowElevated)
    .onAppear { searchFocused = true }
  }
}

private struct ModelRow: View {
  var name: String
  var current: Bool
  var pick: () -> Void
  @State private var hovering = false

  var body: some View {
    Button(action: pick) {
      HStack {
        Text(name)
          .font(.mdowUI(12.5))
          .foregroundStyle(Color.mdowText)
          .lineLimit(1)
        Spacer()
        if current {
          Image(systemName: "checkmark")
            .font(.system(size: 10, weight: .bold))
            .foregroundStyle(Color.mdowAccent)
        }
      }
      .padding(.horizontal, 8)
      .frame(height: 26)
      .contentShape(.rect)
      .background(hovering ? Color.mdowHover : .clear, in: .rect(cornerRadius: 6))
    }
    .buttonStyle(.plain)
    .onHover { hovering = $0 }
    .accessibilityAddTraits(current ? .isSelected : [])
  }
}


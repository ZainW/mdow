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
          .padding(.horizontal, 10)
          .padding(.bottom, 6)
      }
      VStack(alignment: .leading, spacing: 6) {
        if document.documentURL != nil || !session.conversation.tags.isEmpty {
          ContextChips(session: session, document: document)
        }
        TextField(placeholder, text: $text, axis: .vertical)
          .textFieldStyle(.plain)
          .lineLimit(1...8)
          .focused($focused)
          .onChange(of: focused) { _, value in session.composerFocused = value }
          .onChange(of: text) { _, value in updateMention(value) }
          .onKeyPress(keys: [.upArrow, .downArrow, .return, .tab, .escape], phases: .down) { press in
            handleKey(press, mentions: mentions)
          }
          .accessibilityLabel("Message the companion")
        HStack(spacing: 4) {
          Button {
            startMention()
          } label: {
            Image(systemName: "at")
          }
          .buttonStyle(.borderless)
          .help("Mention a Document")
          .accessibilityLabel("Mention a Document")
          .disabled(document.documents.isEmpty)
          ModelPicker(state: runtime.modelState) { session.selectModel($0) }
          Spacer(minLength: 4)
          if streaming {
            Button {
              session.stop()
            } label: {
              Image(systemName: "stop.fill")
                .font(.system(size: 9, weight: .bold))
                .frame(width: 22, height: 22)
                .background(.quaternary, in: .circle)
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
                .font(.system(size: 11, weight: .bold))
                .foregroundStyle(canSend ? AnyShapeStyle(.white) : AnyShapeStyle(.secondary))
                .frame(width: 22, height: 22)
                .background(canSend ? AnyShapeStyle(Color.accentColor) : AnyShapeStyle(.quaternary), in: .circle)
            }
            .buttonStyle(.plain)
            .disabled(!canSend)
            .help("Send (↩)")
            .accessibilityLabel("Send")
          }
        }
      }
      .padding(10)
      .background(.background, in: .rect(cornerRadius: 10))
      .overlay(
        RoundedRectangle(cornerRadius: 10)
          .strokeBorder(focused ? AnyShapeStyle(Color.accentColor.opacity(0.6)) : AnyShapeStyle(.separator))
      )
      .contentShape(.rect)
      .onTapGesture { focused = true }
      .padding([.horizontal, .bottom], 10)
      .padding(.top, 8)
    }
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
    VStack(alignment: .leading, spacing: 0) {
      ForEach(Array(results.enumerated()), id: \.element) { index, url in
        Button {
          pick(url)
        } label: {
          HStack(spacing: 6) {
            Image(systemName: "doc.text").foregroundStyle(.secondary)
            Text(url.lastPathComponent).lineLimit(1)
            Spacer(minLength: 4)
            Text(folder(of: url))
              .font(.caption)
              .foregroundStyle(.tertiary)
              .lineLimit(1)
              .truncationMode(.head)
          }
          .padding(.horizontal, 8)
          .padding(.vertical, 4)
          .contentShape(.rect)
          .background(
            index == selected ? AnyShapeStyle(.selection) : AnyShapeStyle(.clear), in: .rect(cornerRadius: 5))
        }
        .buttonStyle(.plain)
      }
    }
    .font(.callout)
    .padding(4)
    .background(.regularMaterial, in: .rect(cornerRadius: 8))
    .overlay(RoundedRectangle(cornerRadius: 8).strokeBorder(.separator))
    .shadow(color: .black.opacity(0.12), radius: 6, y: 2)
  }

  private func folder(of url: URL) -> String {
    let parent = url.deletingLastPathComponent().path
    guard let root = root?.path else { return "" }
    if parent == root { return "" }
    return parent.hasPrefix(root + "/") ? String(parent.dropFirst(root.count + 1)) : parent
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

private struct Chip: View {
  var symbol: String
  var title: String
  var secondary = false
  var remove: (() -> Void)?

  var body: some View {
    HStack(spacing: 3) {
      Image(systemName: symbol).imageScale(.small)
      Text(title).lineLimit(1).truncationMode(.middle)
      if let remove {
        Button(action: remove) {
          Image(systemName: "xmark").imageScale(.small).font(.caption2.weight(.semibold))
        }
        .buttonStyle(.borderless)
        .accessibilityLabel("Remove \(title)")
      }
    }
    .font(.caption)
    .foregroundStyle(secondary ? .secondary : .primary)
    .padding(.horizontal, 6)
    .padding(.vertical, 2)
    .frame(maxWidth: 180)
    .background(.quaternary.opacity(0.6), in: .rect(cornerRadius: 5))
  }
}

// MARK: Model picker

/// The model menu: a searchable popover grouped by provider, as OpenCode can offer hundreds.
struct ModelPicker: View {
  var state: CompanionModelState
  var select: (String) -> Void
  @State private var presented = false

  var body: some View {
    let unavailable = state.stale || state.options.isEmpty
    let label = state.current?.name ?? (state.stale ? "Loading models…" : "Choose a model")
    Button {
      presented.toggle()
    } label: {
      HStack(spacing: 3) {
        Text(label).lineLimit(1).truncationMode(.tail)
        Image(systemName: "chevron.up.chevron.down").imageScale(.small)
      }
      .font(.caption)
      .foregroundStyle(.secondary)
      .contentShape(.rect)
    }
    .buttonStyle(.borderless)
    .disabled(unavailable)
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
      TextField("Search models", text: $query)
        .textFieldStyle(.roundedBorder)
        .focused($searchFocused)
        .padding(8)
        .onSubmit {
          if let first = groups.first?.options.first { select(first.value) }
        }
      Divider()
      let groups = groups
      if groups.isEmpty {
        Text("No matching models.")
          .foregroundStyle(.secondary)
          .frame(maxWidth: .infinity, maxHeight: .infinity)
      } else {
        List {
          ForEach(groups, id: \.provider) { group in
            Section(group.provider) {
              ForEach(group.options) { option in
                Button {
                  select(option.value)
                } label: {
                  HStack {
                    Text(option.name).lineLimit(1)
                    Spacer()
                    if option.value == state.currentValue {
                      Image(systemName: "checkmark").foregroundStyle(Color.accentColor)
                    }
                  }
                  .contentShape(.rect)
                }
                .buttonStyle(.plain)
              }
            }
          }
        }
        .listStyle(.sidebar)
      }
    }
    .frame(width: 300, height: 380)
    .onAppear { searchFocused = true }
  }
}

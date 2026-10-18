import MdowKit
import SwiftUI

/// The Companion section of Settings: what it runs on, whether it's connected, and the model
/// new conversations start with.
struct CompanionSettingsSection: View {
  @Bindable private var preferences = Preferences.shared
  private var runtime = CompanionRuntime.shared

  var body: some View {
    Section {
      LabeledContent("OpenCode") {
        Text(statusText)
          .foregroundStyle(.secondary)
      }
      if runtime.modelState.options.isEmpty {
        LabeledContent("Default model") {
          Text(preferences.companionModel ?? "OpenCode’s default")
            .foregroundStyle(.secondary)
        }
      } else {
        Picker("Default model", selection: $preferences.companionModel) {
          Text("OpenCode’s default").tag(String?.none)
          Divider()
          ForEach(providers, id: \.self) { provider in
            Section(provider) {
              ForEach(runtime.modelState.options.filter { $0.providerName == provider }) { option in
                Text(option.name).tag(Optional(option.value))
              }
            }
          }
        }
      }
    } header: {
      Text("Companion")
    } footer: {
      Text(
        "The companion runs on OpenCode 2 on this Mac and uses the providers you have signed in to there. You can also choose a model from the companion’s message box."
      )
      .font(.caption)
      .foregroundStyle(.secondary)
      .fixedSize(horizontal: false, vertical: true)
    }
  }

  private var providers: [String] {
    var seen = Set<String>()
    return runtime.modelState.options.map(\.providerName).filter { seen.insert($0).inserted }
  }

  private var statusText: String {
    guard let status = runtime.status else { return "Starts when you open the companion" }
    if status.availability == .available {
      return "Connected to OpenCode \(status.version ?? "")".trimmingCharacters(in: .whitespaces)
    }
    return status.detail ?? "Not connected"
  }
}

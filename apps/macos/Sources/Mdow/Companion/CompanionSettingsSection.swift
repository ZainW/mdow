import MdowKit
import SwiftUI

/// The Companion section of Settings: what it runs on, whether it's connected, and the model
/// new conversations start with.
struct CompanionSettingsSection: View {
  @Bindable private var preferences = Preferences.shared
  private var runtime = CompanionRuntime.shared

  var body: some View {
    VStack(alignment: .leading, spacing: 0) {
      CardHeading("OpenCode")
      PanelCard {
        SettingRow(title: "Status") {
          HStack(spacing: 7) {
            Circle()
              .fill(runtime.isAvailable ? Color.mdowAccent : Color.mdowTertiary)
              .frame(width: 6, height: 6)
              .accessibilityHidden(true)
            Text(statusText)
              .font(.mdowUI(12.5))
              .foregroundStyle(Color.mdowSecondary)
              .multilineTextAlignment(.trailing)
          }
          .frame(maxWidth: 280, alignment: .trailing)
          .accessibilityElement(children: .combine)
        }
        CardDivider()
        SettingRow(title: "Default model", detail: "New conversations start with this model.") {
          if runtime.modelState.options.isEmpty {
            Text(preferences.companionModel ?? "OpenCode’s default")
              .font(.mdowMono(11.5))
              .foregroundStyle(Color.mdowSecondary)
          } else {
            MenuPill(label: "Default model", value: modelName, choices: modelChoices)
          }
        }
      }
      Text(
        "The companion runs on OpenCode 2 on this Mac and uses the providers you have signed in to there. You can also choose a model from the companion’s message box."
      )
      .font(.mdowUI(12))
      .foregroundStyle(Color.mdowTertiary)
      .lineSpacing(2)
      .fixedSize(horizontal: false, vertical: true)
      .padding(.horizontal, 4)
      .padding(.top, 10)
    }
  }

  private var modelName: String {
    guard let value = preferences.companionModel else { return "OpenCode’s default" }
    return runtime.modelState.options.first { $0.value == value }?.name ?? value
  }

  private func modelChoices() -> [MenuChoice] {
    var choices = [
      MenuChoice(title: "OpenCode’s default", isSelected: preferences.companionModel == nil) {
        preferences.companionModel = nil
      },
      .separator,
    ]
    for provider in providers {
      choices.append(.header(provider))
      for option in runtime.modelState.options where option.providerName == provider {
        choices.append(
          MenuChoice(title: option.name, isSelected: preferences.companionModel == option.value) {
            preferences.companionModel = option.value
          })
      }
    }
    return choices
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

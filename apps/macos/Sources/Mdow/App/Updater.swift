import AppKit
import Security
import Sparkle

/// Sparkle 2 updates. Only release builds signed with a Developer ID and carrying a feed URL
/// update; ad hoc and debug builds leave Sparkle off and "Check for Updates…" disabled.
@MainActor final class Updater: NSObject, NSMenuItemValidation {
  static let shared = Updater()

  private var controller: SPUStandardUpdaterController?

  /// Whether this build can update itself.
  let isAvailable: Bool = {
    #if DEBUG
      return false
    #else
      let feed = Bundle.main.object(forInfoDictionaryKey: "SUFeedURL") as? String ?? ""
      return !feed.isEmpty && Updater.isDeveloperIDSigned()
    #endif
  }()

  /// Starts Sparkle, which checks for updates on its own schedule. Call once at launch.
  func start() {
    guard isAvailable, controller == nil else { return }
    controller = SPUStandardUpdaterController(
      startingUpdater: true, updaterDelegate: nil, userDriverDelegate: nil)
  }

  @objc func checkForUpdates(_ sender: Any?) {
    controller?.checkForUpdates(sender)
  }

  func validateMenuItem(_ menuItem: NSMenuItem) -> Bool {
    guard menuItem.action == #selector(checkForUpdates(_:)) else { return true }
    return controller?.updater.canCheckForUpdates ?? false
  }

  /// True when the running app is signed with a Developer ID Application certificate.
  private nonisolated static func isDeveloperIDSigned() -> Bool {
    var code: SecCode?
    guard SecCodeCopySelf([], &code) == errSecSuccess, let code else { return false }
    var requirement: SecRequirement?
    let text =
      "anchor apple generic and certificate 1[field.1.2.840.113635.100.6.2.6] exists"
      + " and certificate leaf[field.1.2.840.113635.100.6.1.13] exists"
    guard SecRequirementCreateWithString(text as CFString, [], &requirement) == errSecSuccess
    else { return false }
    return SecCodeCheckValidity(code, [], requirement) == errSecSuccess
  }
}

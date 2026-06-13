import Foundation

public struct KeyboardShortcutGroup: Equatable, Sendable {
    public let heading: String
    public let items: [KeyboardShortcutItem]

    public init(heading: String, items: [KeyboardShortcutItem]) {
        self.heading = heading
        self.items = items
    }
}

public struct KeyboardShortcutItem: Equatable, Sendable {
    public let label: String
    public let keys: String

    public init(label: String, keys: String) {
        self.label = label
        self.keys = keys
    }
}

public enum KeyboardShortcutReference {
    public static let groups: [KeyboardShortcutGroup] = [
        KeyboardShortcutGroup(
            heading: "Files",
            items: [
                KeyboardShortcutItem(label: "Open file", keys: "⌘ O"),
                KeyboardShortcutItem(label: "Open folder", keys: "⌘ ⇧ O"),
            ]
        ),
        KeyboardShortcutGroup(
            heading: "Navigation",
            items: [
                KeyboardShortcutItem(label: "Command palette", keys: "⌘ K"),
                KeyboardShortcutItem(label: "Find in document", keys: "⌘ F"),
                KeyboardShortcutItem(label: "Find next", keys: "⌘ G"),
                KeyboardShortcutItem(label: "Find previous", keys: "⌘ ⇧ G"),
            ]
        ),
        KeyboardShortcutGroup(
            heading: "Tabs",
            items: [
                KeyboardShortcutItem(label: "Close tab", keys: "⌘ W"),
                KeyboardShortcutItem(label: "Next tab", keys: "⌘ ⇧ ]"),
                KeyboardShortcutItem(label: "Previous tab", keys: "⌘ ⇧ ["),
            ]
        ),
        KeyboardShortcutGroup(
            heading: "View",
            items: [
                KeyboardShortcutItem(label: "Toggle sidebar", keys: "⌘ B"),
                KeyboardShortcutItem(label: "Toggle full width", keys: "⌘ ⇧ W"),
                KeyboardShortcutItem(label: "Keyboard shortcuts", keys: "⌘ /"),
                KeyboardShortcutItem(label: "Zoom in", keys: "⌘ +"),
                KeyboardShortcutItem(label: "Zoom out", keys: "⌘ -"),
                KeyboardShortcutItem(label: "Reset zoom", keys: "⌘ 0"),
                KeyboardShortcutItem(label: "Toggle full screen", keys: "⌃ ⌘ F"),
            ]
        ),
        KeyboardShortcutGroup(
            heading: "App",
            items: [
                KeyboardShortcutItem(label: "Settings", keys: "⌘ ,"),
            ]
        ),
    ]
}

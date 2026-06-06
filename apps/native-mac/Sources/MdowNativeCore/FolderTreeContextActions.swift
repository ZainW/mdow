import Foundation

public enum FolderTreeContextAction: String, Sendable {
    case open
    case copyPath
    case revealInFinder

    public var title: String {
        switch self {
        case .open: "Open"
        case .copyPath: "Copy Path"
        case .revealInFinder: "Reveal in Finder"
        }
    }
}

public enum FolderTreeContextActions {
    public static func actions(isFolder: Bool) -> [FolderTreeContextAction] {
        isFolder ? [.copyPath, .revealInFinder] : [.open, .copyPath, .revealInFinder]
    }
}

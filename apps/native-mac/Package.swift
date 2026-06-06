// swift-tools-version: 6.0

import PackageDescription

let package = Package(
    name: "MdowNative",
    platforms: [
        .macOS(.v14),
    ],
    products: [
        .library(
            name: "MdowNativeCore",
            targets: ["MdowNativeCore"]
        ),
        .executable(
            name: "MdowNativeCoreChecks",
            targets: ["MdowNativeCoreChecks"]
        ),
        .executable(
            name: "MdowNative",
            targets: ["MdowNative"]
        ),
    ],
    dependencies: [
        .package(url: "https://github.com/ChimeHQ/SwiftTreeSitter", from: "0.8.0"),
        .package(url: "https://github.com/tree-sitter/tree-sitter-typescript", from: "0.23.2"),
    ],
    targets: [
        .target(
            name: "MdowNativeCore",
            dependencies: [
                .product(name: "SwiftTreeSitter", package: "SwiftTreeSitter"),
                .product(name: "TreeSitterTypeScript", package: "tree-sitter-typescript"),
            ]
        ),
        .executableTarget(
            name: "MdowNative",
            dependencies: ["MdowNativeCore"]
        ),
        .executableTarget(
            name: "MdowNativeCoreChecks",
            dependencies: ["MdowNativeCore"]
        ),
    ]
)

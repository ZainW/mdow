// swift-tools-version: 6.0

import PackageDescription

let package = Package(
  name: "Mdow",
  platforms: [.macOS(.v15)],
  products: [
    .executable(name: "Mdow", targets: ["Mdow"]),
  ],
  dependencies: [
    .package(url: "https://github.com/swiftlang/swift-markdown", from: "0.9.0"),
    .package(url: "https://github.com/sparkle-project/Sparkle", from: "2.10.0"),
    .package(url: "https://github.com/mgriebling/SwiftMath", from: "1.7.3"),
  ],
  targets: [
    // Parsing, rendering to attributed strings, highlighting and files. No windows.
    .target(
      name: "MdowKit",
      dependencies: [
        .product(name: "Markdown", package: "swift-markdown"),
        .product(name: "SwiftMath", package: "SwiftMath"),
      ]
    ),
    // The AppKit app.
    .executableTarget(
      name: "Mdow",
      dependencies: ["MdowKit", .product(name: "Sparkle", package: "Sparkle")],
      // bundle.sh embeds Sparkle.framework in Contents/Frameworks.
      linkerSettings: [
        .unsafeFlags(["-Xlinker", "-rpath", "-Xlinker", "@executable_path/../Frameworks"])
      ]
    ),
    .testTarget(
      name: "MdowKitTests",
      dependencies: ["MdowKit"]
    ),
  ]
)

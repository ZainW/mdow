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
  ],
  targets: [
    // Parsing, rendering to attributed strings, highlighting and files. No windows.
    .target(
      name: "MdowKit",
      dependencies: [.product(name: "Markdown", package: "swift-markdown")]
    ),
    // The AppKit app.
    .executableTarget(
      name: "Mdow",
      dependencies: ["MdowKit"]
    ),
    .testTarget(
      name: "MdowKitTests",
      dependencies: ["MdowKit"]
    ),
  ]
)

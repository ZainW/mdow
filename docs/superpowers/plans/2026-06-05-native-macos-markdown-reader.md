# Native macOS Markdown Reader Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a separate native macOS Markdown reader that opens and displays `.md`, `.markdown`, and `.mdx` files without web technology.

**Architecture:** Add a SwiftPM app under `apps/native-mac/`. Put file validation, file loading, and Markdown block parsing in `MdowNativeCore` so behavior is testable, and keep SwiftUI/AppKit UI in the `MdowNative` executable.

**Tech Stack:** SwiftPM, Swift 6, SwiftUI, AppKit `NSOpenPanel`, UniformTypeIdentifiers, framework-free Swift check executable.

---

## File Structure

- Create: `apps/native-mac/Package.swift` — SwiftPM manifest with library, executable, and test targets.
- Create: `apps/native-mac/Sources/MdowNativeCore/MarkdownFile.swift` — supported extension detection and UTF-8 file loading.
- Create: `apps/native-mac/Sources/MdowNativeCore/MarkdownParser.swift` — lightweight native Markdown block parser.
- Create: `apps/native-mac/Sources/MdowNative/App/MdowNativeApp.swift` — SwiftUI app entrypoint and menu command wiring.
- Create: `apps/native-mac/Sources/MdowNative/Stores/DocumentStore.swift` — app state and open-file operations.
- Create: `apps/native-mac/Sources/MdowNative/Views/ContentView.swift` — root empty/error/document layout.
- Create: `apps/native-mac/Sources/MdowNative/Views/MarkdownDocumentView.swift` — native SwiftUI Markdown block renderer.
- Create: `apps/native-mac/Sources/MdowNativeCoreChecks/main.swift` — file loading and parser check runner.
- Create: `script/build_and_run.sh` — one command to build, bundle, launch, and verify the native app.
- Create or modify: `.codex/environments/environment.toml` — Codex Run action.

## Task 1: File Validation And Loading

**Files:**
- Create: `apps/native-mac/Package.swift`
- Create: `apps/native-mac/Sources/MdowNativeCoreChecks/main.swift`
- Create: `apps/native-mac/Sources/MdowNativeCore/MarkdownFile.swift`

- [ ] **Step 1: Write the failing tests**

```swift
import XCTest
@testable import MdowNativeCore

final class MarkdownFileTests: XCTestCase {
    func testSupportedMarkdownExtensionsAreCaseInsensitive() {
        XCTAssertTrue(MarkdownFile.isSupported(URL(fileURLWithPath: "/tmp/readme.md")))
        XCTAssertTrue(MarkdownFile.isSupported(URL(fileURLWithPath: "/tmp/readme.MARKDOWN")))
        XCTAssertTrue(MarkdownFile.isSupported(URL(fileURLWithPath: "/tmp/readme.mdx")))
    }

    func testRejectsNonMarkdownExtensions() {
        XCTAssertFalse(MarkdownFile.isSupported(URL(fileURLWithPath: "/tmp/readme.txt")))
        XCTAssertFalse(MarkdownFile.isSupported(URL(fileURLWithPath: "/tmp/markdown.json")))
    }

    func testLoadsUtf8MarkdownFile() throws {
        let directory = FileManager.default.temporaryDirectory
        let url = directory.appendingPathComponent(UUID().uuidString).appendingPathExtension("md")
        try "# Hello\n\nNative reader".write(to: url, atomically: true, encoding: .utf8)
        defer { try? FileManager.default.removeItem(at: url) }

        let document = try MarkdownFile.load(url)

        XCTAssertEqual(document.url, url)
        XCTAssertEqual(document.title, url.lastPathComponent)
        XCTAssertEqual(document.content, "# Hello\n\nNative reader")
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd apps/native-mac && swift run MdowNativeCoreChecks`

Expected: FAIL because `Package.swift` and `MarkdownFile` do not exist yet.

- [ ] **Step 3: Implement minimal file support**

Create `Package.swift` with `MdowNativeCore`, `MdowNative`, and test targets. Create `MarkdownFile.swift` with `MarkdownDocument`, `MarkdownFileError`, `isSupported(_:)`, and `load(_:)`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd apps/native-mac && swift run MdowNativeCoreChecks`

Expected: all `MarkdownFileTests` pass.

## Task 2: Native Markdown Block Parser

**Files:**
- Modify: `apps/native-mac/Sources/MdowNativeCoreChecks/main.swift`
- Create: `apps/native-mac/Sources/MdowNativeCore/MarkdownParser.swift`

- [ ] **Step 1: Write the failing parser tests**

```swift
import XCTest
@testable import MdowNativeCore

final class MarkdownParserTests: XCTestCase {
    func testParsesCoreMarkdownBlocks() {
        let source = """
        # Title

        Intro paragraph
        - Item one
        1. Step one
        > Quoted
        ---
        ```swift
        let value = 1
        ```
        """

        let blocks = MarkdownParser.parse(source)

        XCTAssertEqual(blocks, [
            .heading(level: 1, text: "Title"),
            .paragraph("Intro paragraph"),
            .unorderedListItem("Item one"),
            .orderedListItem(number: 1, text: "Step one"),
            .blockquote("Quoted"),
            .horizontalRule,
            .codeBlock(language: "swift", code: "let value = 1"),
        ])
    }

    func testCombinesWrappedParagraphLines() {
        let blocks = MarkdownParser.parse("This wraps\nonto another line\n\nNext")

        XCTAssertEqual(blocks, [
            .paragraph("This wraps onto another line"),
            .paragraph("Next"),
        ])
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd apps/native-mac && swift run MdowNativeCoreChecks`

Expected: FAIL because `MarkdownParser` does not exist yet.

- [ ] **Step 3: Implement minimal parser**

Create `MarkdownBlock: Equatable` and `MarkdownParser.parse(_:)`. Handle headings, paragraphs, list items, blockquotes, horizontal rules, fenced code blocks, and blank line flushing.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd apps/native-mac && swift run MdowNativeCoreChecks`

Expected: all `MarkdownParserTests` pass.

## Task 3: SwiftUI Native App

**Files:**
- Create: `apps/native-mac/Sources/MdowNative/App/MdowNativeApp.swift`
- Create: `apps/native-mac/Sources/MdowNative/Stores/DocumentStore.swift`
- Create: `apps/native-mac/Sources/MdowNative/Views/ContentView.swift`
- Create: `apps/native-mac/Sources/MdowNative/Views/MarkdownDocumentView.swift`

- [ ] **Step 1: Implement document store**

Create an observable `DocumentStore` that opens files through `MarkdownFile.load(_:)`, rejects unsupported file extensions, and exposes `document`, `blocks`, and `errorMessage`.

- [ ] **Step 2: Implement app entry and commands**

Create a SwiftUI `@main` app with `WindowGroup`, `.commands` for Open (`Command-O`), and AppKit activation through `NSApplicationDelegateAdaptor`.

- [ ] **Step 3: Implement root content**

Create `ContentView` with an empty state, open button, drag/drop for file URLs, error text, and a scrollable document state.

- [ ] **Step 4: Implement Markdown block renderer**

Render parser blocks with SwiftUI `Text`, `Divider`, and monospaced code blocks. Keep all content native; do not use `WKWebView`, HTML views, or web renderers.

- [ ] **Step 5: Build the app**

Run: `cd apps/native-mac && swift build`

Expected: build exits `0`.

## Task 4: Build/Run Integration

**Files:**
- Create: `script/build_and_run.sh`
- Create or modify: `.codex/environments/environment.toml`

- [ ] **Step 1: Add run script**

Create `script/build_and_run.sh` to build `apps/native-mac`, stage `dist/MdowNative.app`, and launch it with `/usr/bin/open -n`. Include `run`, `--debug`, `--logs`, `--telemetry`, and `--verify` modes.

- [ ] **Step 2: Make script executable**

Run: `chmod +x script/build_and_run.sh`

Expected: script has executable bit set.

- [ ] **Step 3: Add Codex Run action**

Create `.codex/environments/environment.toml` with a `Run` action pointing to `./script/build_and_run.sh`.

- [ ] **Step 4: Verify launch**

Run: `./script/build_and_run.sh --verify`

Expected: build exits `0`, app bundle launches, and `pgrep -x MdowNative` succeeds.

## Final Verification

- [ ] Run: `cd apps/native-mac && swift run MdowNativeCoreChecks`
- [ ] Run: `cd apps/native-mac && swift build`
- [ ] Run: `./script/build_and_run.sh --verify`
- [ ] Search to confirm no web tech in native app: `rg -n "WKWebView|WebView|HTML|JavaScript|Electron|React" apps/native-mac`
- [ ] Confirm native app is separate from current Electron app: `test -d apps/native-mac && test -d apps/desktop`

# Mdow GPUI Markdown Reader — Design Spec

Build a separate, locally runnable Rust + GPUI version of Mdow for macOS. The prototype
preserves the established reading experience and visual system while intentionally omitting AI
chat, editing, distribution, and other secondary features.

## Purpose

The prototype must prove that Mdow's core product can feel complete in GPUI: open Markdown,
browse a folder, switch among documents, and read polished rendered content in a quiet native
window. “Minimal” refers to product scope, not presentation quality. Typography, spacing,
color, hierarchy, control states, and window composition are first-class requirements.

The shipping Electron renderer is the visual source of truth. The existing Swift native app is
an interaction and native-platform reference where the Electron behavior depends on browser or
Electron APIs. Neither existing app is modified by this work.

## Scope

The first GPUI build includes:

- A single macOS application window with custom Mdow chrome.
- Opening `.md`, `.markdown`, and `.mdx` files through a native file dialog, drag and drop, or an
  optional command-line path.
- Opening a folder through a native folder dialog or drag and drop.
- A recursively scanned, alphabetically sorted, Markdown-only folder tree.
- Multiple open document tabs with selection and close behavior.
- A document breadcrumb and constrained/full-width reading toggle.
- Native light and dark presentation following the current system appearance.
- Live reload when an open file changes on disk.
- Rendered headings, paragraphs, emphasis, strong text, links, inline code, ordered lists,
  unordered lists, task lists, blockquotes, thematic breaks, fenced code blocks, tables, and
  local images.
- Language labels and copy feedback on fenced code blocks.
- Core shortcuts: Command-O opens a file, Command-Shift-O opens a folder, Command-B toggles the
  sidebar, Command-W closes the active tab, and Command-Shift-W toggles full-width reading.

The first build does not include AI chat, a companion button or panel, editing, split view,
search, command palette, settings, recents persistence, Mermaid, HTML rendering, math rendering,
syntax highlighting, automatic updates, telemetry, signing, notarization, distribution
packaging, or CI configuration.

## Architecture

Create a standalone Cargo package under `apps/gpui/`. It is independent of the pnpm/Turborepo
task graph and is built directly with Cargo for this local experiment.

The package has four focused areas:

1. `app` owns tabs, the selected tab, folder state, sidebar visibility, reading width, theme
   selection, file watchers, and GPUI actions.
2. `document` validates paths, reads UTF-8 files, parses Markdown with `pulldown-cmark`, and
   produces a UI-independent document model.
3. `workspace` scans folders, filters ignored paths, sorts tree entries, and coordinates file
   watching through `notify`.
4. `ui` renders the Mdow window, welcome state, sidebar, tabs, breadcrumb, errors, and document
   blocks with custom GPUI elements and shared design tokens.

GPUI is consumed as the published crate compatible with the installed stable Rust toolchain and
the exact dependency graph is committed in `Cargo.lock`. The prototype does not depend on
`gpui-component`; the small visible control set is implemented directly so Mdow's metrics and
states remain authoritative. `rfd` provides native macOS file and folder dialogs.

Bundled Inter and Geist Mono font assets are copied from the Electron renderer into the GPUI
package and registered at application startup. The required Lucide icons are bundled as SVG
assets inside the GPUI package; system emoji or text glyphs are not substitutes for UI icons.

## Application State and Data Flow

`MdowApp` owns a vector of `DocumentTab` values and an optional active tab identifier. Each tab
contains its canonical path, display title, parsed document, last successful source text, and an
optional non-fatal reload error. Opening an already-open path focuses its existing tab instead of
creating a duplicate.

Opening a file follows this sequence:

1. Canonicalize and validate the extension case-insensitively.
2. Read the file as UTF-8.
3. Parse the source into `DocumentBlock` values and collect headings.
4. Create or replace the matching tab and select it.
5. Ensure the parent path is watched for changes.
6. Notify GPUI so the affected surfaces rerender.

Opening a folder canonicalizes its root, recursively scans it, skips hidden entries and common
generated directories (`.git`, `node_modules`, `target`, `dist`, and `build`), and emits
directories before files with case-insensitive alphabetical sorting. Selecting a tree file uses
the same open-file flow.

File watcher events are debounced before re-reading a document. A successful reload replaces the
parsed document while retaining the tab and active selection. A failed reload retains the last
successfully rendered content and attaches a dismissible inline error message.

Local relative links and images resolve against the active document's parent directory. Markdown
links to supported local files open inside Mdow; HTTP and HTTPS links use the operating system's
default browser. Image loading is bounded by the available reading width and preserves aspect
ratio.

## Markdown Model and Rendering

The parser converts `pulldown-cmark` events into owned, testable blocks and inline spans rather
than exposing parser events to views. Blocks cover headings one through six, paragraphs, list
items and nesting depth, blockquotes, thematic breaks, fenced code, tables, and images. Inline
spans cover plain text, emphasis, strong text, code, links, soft breaks, and hard breaks.

Unsupported Markdown remains readable as text. Raw HTML is displayed as source text and is never
executed. MDX JavaScript and JSX remain readable source text; the prototype does not execute or
interpret them.

Documents render as discrete blocks inside one GPUI vertical scroll container. The model keeps
block boundaries stable so a later virtualized renderer can replace the initial scroll container
without changing the public parser-to-view interface; virtualization itself is not required for
this local prototype.

## Visual Contract

The compact interface scale is fixed for this prototype:

- App chrome uses Inter at 13 px; primary controls and sidebar rows use 12 px.
- Document copy uses Inter at 15.5 px with a 1.65 line-height.
- Inline and fenced code use Geist Mono at 0.875 times the surrounding document size.
- The sidebar is 244 px wide.
- The tab bar is 36 px high; each tab is 28 px high and no more than 200 px wide.
- The breadcrumb is 28 px high.
- The constrained reading column is 768 px wide with 48 px horizontal padding, 22 px top
  padding, and 40 px bottom padding.
- Standard radii are 8 px; code blocks use 10 px, inline code uses 4 px, and images use 8 px.
- General scrollbars are 6 px wide and sidebar scrollbars are 4 px wide.

The light palette preserves the Electron app's warm stone/paper values. The dark palette remains
neutral gray without warm tint. Theme tokens are defined once as GPUI colors for background,
foreground, elevated surface, muted surface, muted foreground, primary, accent, border,
subtle border, destructive, sidebar, and sidebar selection.

Document typography mirrors `markdown.css`:

- H1 is 1.875 em, weight 700, line-height 1.2, and letter-spacing -0.025 em.
- H2 is 1.5 em, weight 650, line-height 1.25, and letter-spacing -0.02 em.
- H3 is 1.15 em, weight 600, line-height 1.3, and letter-spacing -0.01 em.
- H4–H6 use the existing muted hierarchy, with H6 uppercase and 0.03 em tracking.
- Paragraphs have 1 em bottom spacing.
- Blockquotes use a 3 px border and 1 em horizontal inset.
- Code blocks have a 1 px border, 10 px radius, 14 px vertical and 18 px horizontal padding,
  1.6 line-height, and a quiet top-edge/elevation treatment.
- Tables use full 1 px grid borders, 8 px outer radius, 10 px by 14 px cells, and compact
  uppercase headers.

Window composition is titlebar inset, then sidebar beside the main column. The main column is tab
bar, breadcrumb, and content. The empty state uses the Mdow mark, “Mdow” title, “A quiet markdown
viewer. Drop a file anywhere, or open one below.” copy, Open File and Open Folder buttons, and the
established drop-zone treatment.

The active sidebar row uses the subtle sidebar selection plus a 2 px accent bar without changing
font weight. The active tab uses a rounded elevated surface, subtle one-pixel ring, and light
shadow. Inactive tab, close, breadcrumb, tree, and icon controls remain muted until hover. Pressed
controls scale to 0.98 over 120 ms. Focus indication remains visible for keyboard users. Motion is
short and restrained and is disabled when the platform requests reduced motion.

There is no chat control, companion drawer, split-view control, settings footer, AI copy, or empty
space reserved for any AI surface.

## Interaction Details

- Clicking a folder disclosure expands or collapses that branch without opening a document.
- Clicking a file opens or focuses its tab and marks the row active.
- Clicking a tab selects it; its close button closes it without selecting another tab first.
- Closing the active tab selects the nearest remaining tab, preferring the item to its right and
  then the item to its left.
- The sidebar toggle animates its width over roughly 180 ms when reduced motion is not requested.
- Full-width mode removes the 768 px cap and left-aligns content while retaining 48 px padding.
- Dragging supported files or folders anywhere over the content shows a restrained primary-tint
  drop state; dropping uses the normal open flow.
- Clicking a code copy control writes the exact source to the clipboard and shows a check/Copied
  state for two seconds.
- Hover and pressed states never cause text reflow.

## Error Handling

Unsupported extensions, missing paths, invalid UTF-8, and initial read failures produce the
established centered error state with a concise title, explanatory body, muted path, and an Open
File recovery action. Folder scan errors leave the existing tabs usable and show a concise
sidebar error. Reload errors keep the last successful document visible and add a restrained
banner above it. Errors are user-readable and do not expose Rust debug formatting.

An empty or fully closed tab set returns to the welcome state. A folder may remain open while the
main surface is empty.

## Testing and Local Verification

Rust unit tests cover:

- Case-insensitive supported-extension validation and rejection.
- Markdown event conversion for every in-scope block and inline type.
- Raw HTML and MDX remaining inert and readable.
- Folder filtering, directory-first sorting, and ignored directories.
- Tab deduplication, activation, and close-selection behavior.
- Relative file-link and image path resolution.

`cargo test --manifest-path apps/gpui/Cargo.toml` and
`cargo build --manifest-path apps/gpui/Cargo.toml` are the required automated checks.

Add `script/build_and_run_gpui.sh` to build and launch the executable locally, optionally passing
a Markdown file or folder. The script is not a packaging or distribution pipeline.

Runtime verification covers the welcome state, folder tree, rendered showcase document,
multiple tabs, live reload, light appearance, and dark appearance. Final visual verification
captures the GPUI window at the same dimensions as the established Mdow reference and checks the
font faces, chrome heights, sidebar width, content width, palette, and major block spacing.

## Platform and Delivery Constraints

- Target macOS 14 or newer on Apple Silicon for this local prototype.
- Use stable Rust 1.93.0 or newer.
- Keep all work under `apps/gpui/`, the local run script, and this documentation.
- Do not modify the Electron or Swift application behavior.
- Do not add CI, release packaging, signing, notarization, telemetry, or updater work.

## Success Criteria

The prototype is successful when a developer can run one local command, see the polished Mdow
welcome state in a GPUI window, open a Markdown file or folder, navigate the folder tree and tabs,
read the supported Markdown with the established typography and chrome, observe live file reload,
and use the core shortcuts in both system light and dark appearance—with no AI chat UI or AI
runtime code present.

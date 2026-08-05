# Mdow GPUI Reader Fidelity Pass — Design Spec

## Status and Relationship to the Prototype Spec

This is a focused amendment to
`docs/superpowers/specs/2026-07-31-gpui-markdown-reader-design.md`. The existing GPUI prototype,
its deliberately small product scope, and its local-only delivery constraints remain in force.
Where the two documents conflict, this specification supersedes the earlier one in three places:

- Syntax highlighting is now in scope.
- The constrained reader uses 32 px top padding instead of 22 px.
- Active document tabs use the shipping Electron tab geometry, including a 6 px radius instead
  of the GPUI prototype's general 8 px radius.

The shipping Electron app remains the visual and interaction source of truth. The goal is not a
redesign. It is to make the Rust + GPUI reader feel like the established Mdow product while
retaining the previously approved exclusion of AI chat and secondary utilities.

## Problem

The GPUI prototype proves the basic shell and Markdown model, but the open-document experience is
not yet usable or visually faithful enough:

- The document appears longer than the viewport but wheel input does not move it reliably because
  the reader's flex hierarchy does not establish a bounded vertical scroll viewport with a
  content-sized child.
- Fenced code is rendered as one unstyled string, so language-aware syntax color is absent.
- The active tab is visually heavy and its relationship to the tab rail, sidebar toggle, and
  breadcrumb does not match the compact Electron composition.
- Reader top padding, heading rhythm, breadcrumb labeling, and several Markdown surfaces visibly
  drift from the established implementation.
- Existing tests can set a `ScrollHandle`, but do not prove that a long document creates scroll
  extent or that real input changes the visible position.

“Minimal” continues to describe feature scope, not finish quality. A local build that cannot
scroll through a document or present code correctly does not meet the prototype's purpose.

## Goals

1. Make long documents reliably scrollable with trackpad, mouse wheel, scrollbar, and keyboard.
2. Preserve scroll position independently for each open document.
3. Add native syntax highlighting with the language behavior and GitHub-inspired light/dark
   appearance established by the Electron app.
4. Match the compact Electron chrome geometry and reader spacing at the 1120 × 760 reference
   viewport.
5. Bring the already-supported Markdown elements to visual parity without adding new product
   areas.
6. Verify behavior and appearance locally in both system appearances; do not add CI or packaging.

## Non-goals

- AI chat, companion controls, or any AI runtime code.
- Search, recents, outline, settings, command palette, editing, or split view.
- Mermaid, math, raw HTML execution, MDX execution, or arbitrary browser content.
- New theme controls or density settings.
- Virtualization, background indexing, or a generalized editor text engine.
- CI, distribution packaging, signing, notarization, updates, or telemetry.
- Changes to the Electron or Swift applications.

## Measured Visual Contract

The compact Electron density is authoritative. Values are taken from `TabBar.tsx`,
`DocumentBreadcrumb.tsx`, `MarkdownView.tsx`, `index.css`, and `markdown.css`, and were checked
against matched 1120 × 760 dark-mode captures.

### Window and chrome

- Sidebar width: 244 px.
- GPUI titlebar inset: 28 px.
- Tab rail height: 36 px.
- Tab height: 28 px, vertically centered to leave exactly 4 px above and below.
- Tab-list horizontal inset: 6 px.
- Gap between adjacent tab wrappers: 1 px.
- Active tab radius: 6 px.
- Active tab maximum width: 200 px.
- Active tab treatment: elevated card background, one-pixel subtle ring, a one-pixel bottom edge,
  and a restrained 0 1 px 2 px shadow at roughly four-percent black.
- Tab content horizontal inset: 10 px.
- File icon: 14 px, followed by a 6 px icon-to-label gap.
- Close target: 24 px with 4 px end margin; the visible close icon remains 12 px.
- Inactive tabs remain borderless and muted until hover. Separators appear only between inactive
  neighbors and do not touch an active tab.
- The sidebar toggle remains available, but occupies its own centered control slot outside the tab
  list. The list keeps its full 6 px leading inset instead of visually colliding with the toggle.
- Breadcrumb height: 28 px with 12 px horizontal inset and 8 px separation between the path and
  the trailing width control. Path internals use 2 px gaps.
- The breadcrumb displays the filename by default. A frontmatter title replaces the primary label
  only when it is present, with the filename retained as muted secondary text.

The tab bar, breadcrumb, and sidebar boundary must share aligned one-pixel separators. No hover,
focus, copied, or pressed state may change component size or move neighboring content.

### Reader geometry and typography

- Constrained content width: 768 px maximum.
- Horizontal reader padding: 48 px.
- Top padding: 32 px.
- Bottom padding: 40 px minimum, plus enough scroll extent to reveal the final block cleanly.
- Body: Inter, 15.5 px, regular weight, 1.65 line height.
- Code: Geist Mono at 0.875 em with 1.6 line height for fenced blocks.
- H1: 1.875 em, weight 700, line height 1.2, tracking -0.025 em.
- H2: 1.5 em, weight 650, line height 1.25, tracking -0.02 em.
- H3: 1.15 em, weight 600, line height 1.3, tracking -0.01 em.
- H4–H6 retain the muted Electron hierarchy; H6 is uppercase with 0.03 em tracking.
- Paragraphs use 1 em bottom spacing. Heading margins, adjacent block spacing, nested list rhythm,
  and first/last block exceptions follow `markdown.css` rather than a separate GPUI scale.
- Inline code uses a 4 px radius and 0.1 em by 0.35 em padding.
- Code blocks use a one-pixel border, 10 px radius, 14 px vertical and 18 px horizontal padding,
  and a quiet top-edge lift in dark appearance.
- Blockquotes use a 3 px leading border and 0.4 em by 1 em inset.
- Tables use an 8 px outer radius, full one-pixel grid, 10 px by 14 px cells, compact uppercase
  headers, and horizontal scrolling inside the table wrapper.
- Images preserve aspect ratio, never exceed the reading column, and use an 8 px radius.

Wide mode continues to remove the 768 px cap while retaining 48 px side padding. Narrow windows
may reduce effective side padding only as needed to prevent the reading surface from disappearing.

### Sidebar empty state

The minimal build does not gain recents, outline, settings, or other utilities. Its existing empty
folder state is refined using the established hierarchy: a real bundled folder icon, “No folder
open” as the primary label, and short muted instructional copy at 12 px / 18 px. It remains quiet
and centered within the available sidebar body.

## Scrolling Design

The main document surface is a vertical flex column. After error banners, the active reader
viewport owns the remaining bounded height. The scroll viewport and document column obey these
layout invariants:

1. The main surface and reader viewport both use `min-height: 0` semantics so their parent may
   constrain them.
2. The scroll viewport grows to the remaining height and is explicitly a vertical layout surface.
3. The document column is content-sized and non-shrinking; its full block height contributes to
   vertical scroll extent.
4. Code and table wrappers own horizontal overflow without intercepting vertical wheel movement.
5. The reader scrollbar is 6 px and uses the existing theme tokens.

`MdowApp` continues to own one stable `ScrollHandle` per canonical document path. Switching tabs
selects the matching handle, so returning to a document restores its prior position. Live reload
retains the existing handle and clamps an out-of-range position only if the replacement document
is shorter. Closing a tab removes its handle when no remaining tab references that path.

The focused reader responds to Page Up, Page Down, Home, and End. Wheel and trackpad input use
native GPUI scrolling. Dragging the scrollbar thumb remains available; no custom inertial physics
or scroll animation is introduced.

## Syntax Highlighting Design

Add an isolated `syntax` module backed by Syntect `=5.3.0`. Pin the dependency in
`apps/gpui/Cargo.toml` and the GPUI lockfile. Syntax definitions and theme data are loaded once
rather than for every render.

The parser remains responsible only for preserving the fenced language label and exact code
source. A preparation step converts code blocks into UI-independent highlighted lines and token
runs for light and dark appearance. The GPUI renderer selects the prepared palette and emits
styled Geist Mono text without reparsing Markdown or re-highlighting on every frame.

Language normalization:

- Trim whitespace, lowercase the first info-string token, and remove a leading `language-`.
- Recognize the Electron aliases where a Syntect grammar exists, including `js`/`javascript`,
  `ts`/`typescript`, `py`/`python`, `rs`/`rust`, `sh`/`shell`/`bash`/`zsh`, `yml`/`yaml`,
  `md`/`markdown`, `rb`/`ruby`, `cs`/`csharp`, `cpp`/`c++`, `jsx`, and `tsx`.
- Preserve the normalized language badge even when no grammar is available.
- Missing, unknown, unsupported, or malformed language labels produce one plain-text run rather
  than an error.

Token colors follow the GitHub Light and GitHub Dark family already used by Shiki in Electron.
The palette must clearly distinguish comments, keywords, strings, numbers/constants, types,
functions, and punctuation while remaining subordinate to prose. Representative anchors are:

| Token | Light | Dark |
| --- | --- | --- |
| Default | `#24292f` | `#e6edf3` |
| Comment | `#6e7781` | `#8b949e` |
| Keyword | `#cf222e` | `#ff7b72` |
| String | `#0a3069` | `#a5d6ff` |
| Function/type | `#8250df` | `#d2a8ff` |
| Constant/number | `#0550ae` | `#79c0ff` |

The language badge remains 11 px Geist Mono, lowercase, and sits to the left of the existing copy
target. Copy always writes the original unmodified code, not reconstructed highlighted text.
Successful copy feedback remains visible for two seconds.

## Data Flow and Component Boundaries

Document loading becomes:

1. Validate and read the file exactly as today.
2. Parse Markdown into the existing owned `ParsedDocument` model.
3. Prepare syntax runs for each fenced block through the shared highlighter.
4. Store the parsed content and prepared code presentation together behind the active tab's
   existing shared document ownership.
5. Render blocks using only prepared presentation data and current theme selection.

The parser tests remain independent of GPUI and Syntect. The syntax layer exposes small owned
types such as highlighted lines and token runs, allowing alias and fallback behavior to be tested
without opening a window. The renderer does not know Syntect scope names, and Syntect does not
know GPUI colors or elements.

Live reload repeats parsing and preparation before replacing the tab's last successful content.
If either file parsing or preparation fails, the existing last-good document remains visible.
Theme switching chooses the precomputed light or dark runs and triggers a normal rerender.

## Markdown Fidelity and Interaction Details

The pass audits every already-supported block against `markdown.css`: headings, paragraphs,
emphasis, strong text, links, inline code, ordered and unordered lists, nested lists, task lists,
blockquotes, thematic breaks, fenced code, tables, and local images.

- Link hover and keyboard focus remain visible without changing line wrapping.
- Task checkboxes remain non-editable viewer affordances and use the established checked/unchecked
  colors.
- Code and tables scroll horizontally only when their contents exceed the available width.
- The document itself owns vertical scrolling even when the pointer is over code or a table.
- The final block can be scrolled above the bottom edge by the full 40 px bottom inset.
- Switching constrained/full-width mode preserves the document's vertical position.
- Resizing the window does not reset position, clip chrome, or allow the sidebar to consume the
  minimum readable main width.

## Error and Fallback Behavior

- Unknown syntax languages render as plain code with the badge intact.
- A Syntect lookup or highlighting failure falls back to plain code and never prevents the rest of
  the document from rendering.
- Invalid UTF-8, missing paths, unsupported extensions, and folder errors keep their established
  user-facing recovery states.
- A live-reload read or parse failure keeps the last successful document and its scroll position,
  with the existing restrained reload banner.
- Raw HTML and MDX remain inert readable source; this pass does not execute either format.
- No failure path displays Rust debug formatting or panics the application for document content.

## Testing Strategy

Implementation follows red-green-refactor for each behavior change.

### Automated tests

- A long reader fixture produces content height greater than viewport height in GPUI test support.
- The scroll structure exposes a bounded viewport and a non-shrinking content column.
- Setting and changing a document's scroll handle changes the visible reader offset.
- Switching between two paths retains independent scroll offsets.
- Reloading a path retains its handle; closing it removes stale scroll state.
- Language normalization and aliases resolve to the expected syntax.
- Known Rust, JavaScript, JSON, and shell samples yield more than one styled token run.
- Unknown and missing languages yield exact plain-text fallback content.
- Light and dark preparation produce the same text and different token colors.
- Copy uses the exact original source after highlighting.
- Compact shell tests assert the 244 / 36 / 28 / 28 geometry, 4 px tab breathing room, 6 px tab
  radius, and 32 px reader top inset.
- Breadcrumb tests assert filename default and frontmatter-title fallback behavior.
- Existing parser, folder, tab, error, link, image, and interaction tests continue to pass.

### Runtime verification

Use a single deterministic showcase document containing enough content to require scrolling:
H1–H6, paragraphs, emphasis, strong text, links, nested ordered/unordered/task lists, a
blockquote, thematic rule, wide table, local image, long code line, highlighted Rust/JavaScript/
JSON/shell blocks, and an unknown-language block.

Check locally:

- Trackpad and mouse-wheel movement.
- Scrollbar thumb dragging.
- Page Up, Page Down, Home, and End.
- Horizontal code/table scrolling without losing vertical wheel behavior.
- Per-tab restoration, live reload, constrained/full-width mode, and window resizing.
- Tab activation, close, hover, pressed, and keyboard-focus states.
- Code badge, token colors, copy feedback, and unknown-language fallback.
- Light and dark system appearance.

Capture fresh Electron and GPUI windows in the same one-tab state, document position, appearance,
and 1120 × 760 viewport. Place the captures together and compare chrome alignment, tab spacing,
reader inset, content measure, typography, code surface, and scrollbar behavior. A screenshot alone
is not acceptance; the input checks above must also pass.

Required commands are:

```sh
cargo fmt --manifest-path apps/gpui/Cargo.toml -- --check
cargo test --manifest-path apps/gpui/Cargo.toml
cargo clippy --manifest-path apps/gpui/Cargo.toml --all-targets -- -D warnings
cargo build --manifest-path apps/gpui/Cargo.toml
./script/build_and_run_gpui.sh apps/gpui/tests/fixtures/showcase.md
```

No pnpm, Electron, CI, packaging, or distribution changes are required for this pass.

## Success Criteria

The pass is complete when:

- A long Markdown document can be traversed from first line to final block with trackpad, wheel,
  scrollbar, and keyboard.
- Each tab restores its own vertical position and keeps it through theme, width, resize, and live
  reload changes.
- Supported fenced code visibly uses language-aware GitHub-inspired light/dark token colors;
  unknown languages remain fully readable.
- Tabs, breadcrumb, sidebar boundary, reader inset, typography, and Markdown surfaces match the
  measured Electron reference at 1120 × 760 in both appearances.
- The deterministic visual comparison shows no unexplained spacing, radius, clipping, or hierarchy
  drift in the reader path.
- All required local checks pass and the launcher opens the verified build.
- The GPUI app still contains no AI chat surface or AI runtime code.

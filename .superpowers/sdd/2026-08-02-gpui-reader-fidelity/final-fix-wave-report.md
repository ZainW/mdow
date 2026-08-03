# GPUI Reader Fidelity — Final Fix Wave Report

Base commit: `7e6b7ca fix(gpui): scope list groups by depth`

Intended final commit: `fix(gpui): close final reader gaps`

## Outcome

The final reader findings are closed without changing the established Mdow visual direction,
adding AI/companion UI, or touching Electron, CI, packaging, publishing, or distribution code.

- Long documents now expose a visible 6 px GPUI scrollbar with a proportional thumb, track clicks,
  and literal thumb dragging backed by the active document's existing `ScrollHandle`.
- A Markdown list item now remains one structured item containing ordered child blocks, so a loose
  paragraph → fenced code → paragraph item renders one marker and preserves source order.
- Workspace traversal cannot escape the canonical workspace root through file or directory
  symlinks.
- Local Markdown links and images resolve percent-encoded URL paths, queries, and fragments safely.
- The showcase's authored local guide and image now resolve to real fixture files.
- Existing tabs, per-document scroll retention, live reload, syntax highlighting, link focus,
  native menus, and reader geometry remain covered and green.

## Finding 1 — Explicit reader scrollbar

### Root cause

The reader used GPUI's vertical overflow and `ScrollHandle`, but GPUI 0.2.2 did not paint a visible
vertical scrollbar for this surface. The specification's 6 px rail and literal drag contract were
therefore not met even though wheel and keyboard scrolling worked.

### TDD evidence

RED was observed in three independent slices:

1. Geometry tests failed to compile because the scrollbar geometry/pointer helpers did not exist.
2. A GPUI render regression could not find `reader-scrollbar-track` for a long document.
3. With track-click production behavior temporarily absent, the integration test left the active
   offset at zero.

GREEN coverage now proves proportional/clamped geometry, pointer-to-offset mapping, a visible thumb
only when content overflows, a literal drag changing the active offset, and a track click changing
the active offset.

### Implementation

- The renderer overlays a theme-aware 6 px track without changing the 768 px reader measure.
- Thumb height is proportional to viewport/content extent with a 28 px minimum.
- Pointer down/move/up events update the same per-document `ScrollHandle` already used by wheel,
  keyboard, tab switching, and live reload.
- Track clicks center the thumb around the pointer and clamp to the scroll extent.
- Drag state is cleared with the app's other transient reader state and on document transitions.

## Finding 2 — Structured multi-block list items

### Root cause

`ItemContext::into_blocks` flattened one Markdown item into multiple top-level blocks. A loose list
item containing a paragraph, fenced code block, and trailing paragraph could therefore emit more
than one marker and lose the structural relationship between its children.

### TDD evidence

The parser regression first failed to compile because the old `content` model could not express
ordered child blocks. Renderer compilation then exposed every remaining old-pattern match. A link
focus regression also returned no targets when recursion was intentionally absent. The final GPUI
regression proves one marker, equally indented children, Paragraph → CodeBlock → Paragraph vertical
order, and no continuation marker.

GREEN affected suites before the final full gate were:

- `document::tests`: 13/13
- `syntax::tests`: 6/6
- `ui::reader::tests`: 26/26 at that checkpoint
- native GPUI multi-block list regression: 1/1

### Implementation

- `ListItem` and `TaskItem` now own `children: Vec<DocumentBlock>`.
- Parser item contexts collect inline paragraphs and block children, then emit exactly one item.
- Nested list items remain children of their parent instead of becoming unrelated top-level blocks.
- Syntax preparation keys code by a full block path (`Vec<usize>`) and recursively prepares nested
  fenced blocks.
- Rendering recursively preserves spacing, marker/check ownership, stable debug identity, code
  copy identity, and link-focus source order.
- The old reader-bounds test now finds the deliberately long code block semantically instead of
  relying on a top-level index that list structure can legitimately change.

## Finding 3 — Workspace symlink confinement

### Root cause

Workspace scanning canonicalized entries and prevented cycles, but did not verify that a canonical
symlink target remained beneath the canonical workspace root. Both a linked directory and a linked
Markdown file outside the root could appear in the tree.

### TDD evidence

The RED regression expected only `visible.md` but received
`external-directory`, `external-file.md`, and `visible.md`. The final workspace suite passes 6/6,
including broken symlinks, in-root cycles, nested read failures, sorting, and escaped file/directory
targets.

### Implementation

`scan_directory` now carries the canonical workspace root through recursion. Each entry is
canonicalized before metadata or traversal and is skipped when `canonical_path.starts_with(root)`
is false. In-root symlinks retain existing cycle detection and behavior.

## Finding 4 — Percent-decoded local targets

### Root cause

Local target resolution stripped only a fragment. URL-path escapes and queries remained literal,
so `guide%20one.md#section` and `images/hero%20shot.png?raw=1` did not resolve to filesystem paths.

### TDD evidence

The RED resolver returned `/vault/images/hero%20shot.png?raw=1` instead of
`/vault/images/hero shot.png`. GREEN tests cover Markdown classification, a real existing image,
UTF-8 escapes, query/fragment removal, malformed escapes, and encoded separators.

### Implementation and safety

- The URL path ends at the first raw `?` or `#`.
- Valid percent bytes are decoded exactly once; `+` remains a literal plus because this is a URL
  path, not form data.
- Malformed or non-UTF-8 encodings return an inert target.
- Encoded NUL, `/`, and `\` are rejected, so percent decoding cannot introduce a new path separator
  or traversal interpretation.
- Existing URI-scheme rejection and lexical path normalization remain in force.

## Minor fidelity items

The fixture is now self-contained:

- `tests/fixtures/guide.md` is a real local Markdown destination with a return link.
- `tests/fixtures/images/preview.png` is a byte-for-byte reuse of the repository's existing desktop
  raster icon, not a generated placeholder. Both files have the authored showcase paths.
- A regression asserts that the local link classifies as Markdown and exists, and that the local
  image both exists and passes GPUI's supported-image resolution.

Heading letter-spacing remains represented and tested in `BlockStyle`, but GPUI 0.2.2 exposes no
letter-spacing property in either its exact `TextStyle` or `TextRun` API. Applying the value would
require replacing GPUI's wrapped, link-aware text layout with a custom glyph engine. This wave does
not introduce a per-character approximation that would regress wrapping, selection, accessibility,
or link interaction. This is a bounded upstream API limitation, not an unimplemented available
style call.

## Files changed

- `apps/gpui/src/app.rs` — scrollbar drag state, native scrollbar integration tests, structured-list
  GPUI regression, and semantic showcase code lookup.
- `apps/gpui/src/document.rs` — structured item children, recursive plain text, safe URL-path
  decoding, and parser/resolver regressions.
- `apps/gpui/src/syntax.rs` — path-keyed recursive nested-code preparation and tests.
- `apps/gpui/src/ui/reader.rs` — scrollbar geometry/rendering/input, recursive list rendering,
  nested link identity, nested code lookup, fixture/link/image tests.
- `apps/gpui/src/workspace.rs` — canonical-root confinement and escaped-symlink regression.
- `apps/gpui/tests/fixtures/guide.md` — real local Markdown destination.
- `apps/gpui/tests/fixtures/images/preview.png` — reused repository raster asset.
- This report and the final-fix native QA artifacts.

## Automated verification

Every Cargo invocation used:

```text
DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer
```

Fresh pre-report results:

```text
cargo fmt --manifest-path apps/gpui/Cargo.toml -- --check
# exit 0

cargo test --manifest-path apps/gpui/Cargo.toml
# 122 library + 2 binary = 124 passed, 0 failed; doc tests 0

cargo clippy --manifest-path apps/gpui/Cargo.toml --all-targets -- -D warnings
# exit 0, no warnings

cargo build --manifest-path apps/gpui/Cargo.toml
# exit 0
```

The required native launch command also built and launched the showcase successfully:

```text
DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer \
  ./script/build_and_run_gpui.sh apps/gpui/tests/fixtures/showcase.md
```

## Native 1120 × 760 verification

Computer Use cannot attach to the raw unbundled debug process. A temporary uniquely named
`Mdow GPUI QA.app` wrapper therefore symlinked its executable to the exact gated binary, whose
SHA-256 was `4380a9eaae00d9b2051596dd0a49356defd690ab2b24f7eb054d559d9340457b`.
The wrapper changed no repository source. It was quit after QA and moved from `/private/tmp` to the
recoverable Trash location `/Users/zain/.Trash/mdow-gpui-qa.TqY8K1`.

Verified in the native dark window:

- Literal thumb drag from `(1116, 120)` to `(1116, 410)` moved the thumb from approximately
  y=96–199 to y=386–489 and moved the visible document from the heading/list region to the
  JavaScript/JSON/shell/unknown-code/wide-table region.
- Home returned to the first heading; End reached paragraph 24 with the thumb at the bottom; Page
  Down and native wheel input both moved the document.
- The local raster visibly decoded, and the local-guide link opened the real `guide.md` in a second
  native tab.
- Switching back to a showcase tab left at End restored its independent bottom position.
- Nested list/task structure, checked states, syntax colors, code cards, the 768 px constrained
  measure, bounded wide code/table surfaces, tab chrome, and the visible scrollbar remained intact.
- No AI, chat, companion, split-view, or reserved companion surface appeared.

All final-fix captures are exactly 1120 × 760:

| Artifact | Purpose | SHA-256 |
| --- | --- | --- |
| `final-fix-artifacts/native-scrollbar-drag-before.jpg` | Thumb and document before literal drag | `2937ee22bcb85e870aea6adf44937dcc663ff6e7e4416eee73e072bb3d4d0640` |
| `final-fix-artifacts/native-scrollbar-drag-after.jpg` | Thumb and document after literal drag | `5d0474796f12426a8370363dcd726596bae1fae082ef3e514860362dc34b08f1` |
| `final-fix-artifacts/native-tab-scroll-restored.jpg` | Showcase bottom restored after switching tabs | `52d8268737bc804dce902c7d2880370e3a362549b587ee248d3dea8d2aaa9da2` |

The earlier Task 6 evidence in this same SDD directory continues to cover the full light/dark
state-matched Electron/GPUI visual comparison. This final wave added interaction-specific evidence
for the new explicit thumb and final fixes rather than changing the approved visual direction.

## Scope confirmation

- No Electron source was modified.
- No CI, packaging, publishing, or distribution configuration was modified.
- No dependency was added.
- No AI/companion capability or UI was added.

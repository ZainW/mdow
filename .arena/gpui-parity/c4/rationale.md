# Rationale — C4, the reader engine

## Problem

Mdow's GPUI build has to match the Electron reader loop, and the gap is not a list of
missing dialogs. It is that Electron gets four of the reader's most-used behaviours free
from the DOM — cross-block text selection, copy, find-in-document with highlight and
reveal, and scroll-spy over headings — and GPUI 0.2.2 gives none of them. There is no
cross-element selection, no built-in input widget, and `TextLayout` geometry is only
readable during paint. Bolting each behaviour onto the existing renderer means four
independent traversals of `Vec<DocumentBlock>`, four coordinate systems, and four caches to
invalidate on every reload.

The existing shape makes that worse in three concrete ways, and the design has to answer
each. `render_document` re-runs `inline_layout()` for every block on every frame, so the
flat string a selection would address does not outlive the frame. Nested blocks have no
identity: `block_path_render_index` FNV-hashes the path into a `usize` and sets the top bit
to avoid colliding with top-level indices, which is a collision-prone stand-in for the
`BlockId` the system actually needs. And `MdowApp` has already absorbed six reader fields
and fifteen reader methods — link focus handles keyed by `(PathBuf, LinkFocusKey)`, scroll
handles keyed by path, hover and focus keys, scrollbar drag state — because the renderer is
free functions with nowhere to keep anything.

Two constraints bound the answer. The 132 passing tests include a measured visual contract
(sidebar 244, reader 768, 15.5/1.65, the heading table, the margin-collapse table), and that
contract must survive verbatim. And live reload is not the streaming-append case the Waku
design is tuned for: an external editor rewrites the whole file, and the change can be
anywhere in it.

## Usage (caller's view)

Written first, in [`usage.md`](usage.md). The three call sites it commits to:

```rust
// 1. Mount. One child; the app sees no block, atom, run, or glyph.
let style = ReaderStyle::resolve(&self.prefs, theme);
tab.reader.update(cx, |reader, cx| reader.set_style(style, cx));
shell.child(tab.reader.clone())

// 2. Reload under the user's cursor. Scroll, selection, and the current match survive.
tab.reader.update(cx, |reader, cx| reader.reload(loaded.source, cx));

// 3. Find and copy, in the same coordinate space.
reader.set_query(Some(query), cx);
let label = match reader.match_status() {
    None => "".into(),
    Some(s) if s.total == 0 => "No results".into(),
    Some(s) => format!("{} of {}", s.index + 1, s.total).into(),
};
cx.write_to_clipboard(ClipboardItem::new_string(reader.selection_text()?));
```

The app keeps what it should: which document is active, what a link activation _does_,
preferences, and the find bar's chrome. The reader keeps what only it can know: what a link
_is_, where the glyphs landed, and every byte range derived from the text.

## Shape

### The core structure

A document compiles to a flat, document-ordered `Vec<Atom>`. An atom is one shaped text
element: a `SharedString`, the semantic runs that tile it, its link ranges, and its
inline-code ranges. Selection, find, copy, outline reveal, and link hit-testing are all
queries in one coordinate space, `(atom, byte range)`.

Alongside it sits `Vec<Block>` in depth-first order, where each block is either a leaf that
owns a contiguous atom range or a container that owns a contiguous descendant span — never
both. That `BlockBody` enum is doing real work: it is what guarantees a subtree's atoms stay
contiguous, which is what makes an incremental reparse one `Vec::splice` rather than a
scatter, and what lets `reveal` map an atom to a top-level scroll target with arithmetic.

Tracing the dominant access patterns through it, per the data-structures-first discipline:
render walks atoms in order (`Vec` iteration); selection resolve needs the atoms between two
positions (index range); find produces hits already in document order, so "next match" is
`index + 1` with no sort; hit-testing scans only the atoms painted this frame; the reparse
splice needs a block's atoms contiguous. None of those wants a map. The one identity lookup
that does — `AtomId` to `AtomIndex` after a reload — happens twice per reload, for the two
selection endpoints, and is a linear scan over blocks on purpose.

### Identity, and why it is two types

`AtomId` is stable across reparses; `AtomIndex` is a position in the current compilation.
They are separate types because the two mistakes they prevent are opposite: comparing ids for
document order is meaningless, and persisting an index across a reload is silently wrong.
Waku uses a bare paint ordinal, which is correct for an append-only transcript and wrong for
a file that can gain a paragraph at the top. Per encode-lessons-in-structure, the difference
is in the type system rather than in a comment.

`BlockId` also retires `block_path_render_index`. Element ids, debug selectors, and focus
keys all derive from a real identity instead of a hash with a hand-set top bit.

### Incremental reload is the load-bearing behaviour

`Doc::reparse` diffs old against new source, snaps the common prefix and suffix down to
block boundaries, reparses only the middle, and splices. Blocks whose bytes did not move keep
their `BlockId`, so their atoms keep their `AtomId`, so scroll offset, selection anchors, and
find matches outside the edited span survive a save. Electron cannot do this — it re-renders
and drops find state — so this is parity plus one.

It is also the thing that makes reload cheap enough that reload is the _only_ mutation. There
is no separate "invalidate highlights," "recollect link targets," "rebuild outline," "rerun
search." Those are derived from `Doc`; splicing `Doc` updates them by construction. Single
source of truth per invariant, derive instead of sync.

Reloading byte-identical source returns `unchanged: true` and mutates nothing, so a watcher
that fires twice costs one string compare.

### Colour is not compiled

Runs stored in `Doc` are semantic — emphasis, strong, code, strike, script, token class — and
carry no colour or font handle. Colour resolves against the palette at paint, memoised in a
`RunCache` keyed by a `StyleEpoch` integer. A theme switch therefore invalidates only the
cheap colour mapping; parsing, tokenising, and flattening survive it untouched. Waku bakes
colours into cached `TextRun`s and has to clear the whole cache on a theme change. Removing
that invalidation axis removes a class of bug, not just some work.

### The one piece of shared mutable state, named and fenced

Geometry has two writers in the naive design: paint fills it, input reads it. Per
separate-before-serializing-shared-state, I asked what happens if both write, and the answer
was "nothing good," so the two roles are split rather than locked. `FrameLog` is written only
during paint (`begin`, `record`) and read only during input (`hit`, `bounds_of`). It exposes
no iterator and no borrow, so a caller cannot hold a reference across the phase boundary. The
merge happens at exactly one point: `Selection::resolve`, which takes logical anchors from
the entity — where GPUI already serialises writes — and projects them onto the frame's order.

The reader's _logical_ state has one writer, the entity, and needs no interior mutability at
all.

### Interface depth

The public surface is `Reader` with ten methods, a `Render` impl, and three events. Behind
it: two markdown/HTML front ends, an incremental block differ, a syntax tokeniser, a text
flattener with a tiling invariant, per-frame glyph geometry, nearest-neighbour hit testing,
wrapped-row rect walking, selection algebra with word and line granularity, clipboard
formatting with block breaks, match ordering and repair, and scroll reveal with an offscreen
fallback. Learning the interface genuinely saves the caller from learning the implementation,
which is the test that matters.

The surface is also smaller than what it replaces. `MdowApp` sheds six fields and fifteen
methods; `ReaderLinkState`, `LinkSurfaceKey`, `LinkFocusKey`, `LinkFocusTarget`,
`InlineLayout`, `InlineStyleRange`, `PreparedDocument`, and `HighlightedCode` all leave the
public surface. Per boundary-discipline nothing from `pulldown_cmark` or `syntect` crosses
it; the current build re-exports `SyntaxColor` and `HighlightedRun`, which is leakage that
would force a coordinated edit if either dependency changed.

### What the design deliberately does not do

No virtualisation. No editing. No `gpui-component` dependency: the refusal in the existing
spec holds because the visual contract is Mdow's, and the pieces we would have wanted from it
— a text selection layer — are exactly what this design owns. No mermaid execution; a mermaid
block is a native card whose diagram source renders as a selectable, searchable code atom.

### Chrome, typed but thin

`Chrome` holds `modal: Option<Modal>` and `find: Option<FindBar>`. Two fields, not one enum,
because find is genuinely a different layer: non-modal, coexisting with a readable document,
and keeping the reader's selection alive. Folding it into the modal enum would encode a lie
about how it behaves. Escape resolves in one place, `Chrome::dismiss`, which closes the modal
first and the find bar second and reports whether anything closed.

`ReadingWidth` replaces `wide_mode: bool` with the three Electron widths (768 / 896 / 1088)
plus wide mode as a separate override. `ZoomLevel` clamps 60..=200 in steps of ten by
construction. `Session` is one value written to one file with temp-plus-rename and a
compare-before-write, so saving twice writes once and a crash mid-save leaves the previous
session intact.

## Synthesis decision

_Filled in by arena._

## Tradeoffs accepted

- **We accept a full rewrite of `document.rs`, `syntax.rs`, and `ui/reader.rs` in exchange
  for one coordinate space.** Roughly 3,500 lines change. The alternative is four
  behaviours each carrying their own traversal and cache, which is more total code and more
  ways to be inconsistent.
- **We accept `Rc<RefCell<_>>` for painted geometry in exchange for cross-element
  selection.** GPUI 0.2.2 fills `TextLayout` during prepaint and offers no other read point.
  The mitigation is the narrow four-method surface, not pretending the sharing is absent.
- **We accept re-resolving `TextRun`s per frame for visible atoms in exchange for a theme
  switch that invalidates nothing expensive.** This is the same per-frame allocation the
  current build already pays for _everything_, including the string and the styles; we are
  keeping the cheap half of it and deleting the expensive half.
- **We accept that a prefix/suffix block diff is coarser than a real tree diff.** A one-word
  edit in the middle of a document reparses everything between the last unchanged block
  before it and the first unchanged block after it — usually one block. A change on line 1
  and another on the last line reparses the whole file. That is the honest common case, and
  the fallback is the current behaviour, not worse than it.
- **We accept `AtomId`/`AtomIndex` as two types the reader must keep straight** in exchange
  for making persist-an-index and order-by-id unrepresentable. This looks like ceremony until
  the first reload lands under a live selection.
- **We accept that mermaid source is selectable and searchable, where Electron excludes
  `.mermaid-container` from find.** Registered content is uniformly live; a per-atom
  searchability flag would be a filter maintained at three call sites to reproduce an
  Electron limitation rather than an Electron feature.
- **We accept one `Reader` entity per open tab, holding its whole compiled document in
  memory.** A parsed document runs several times its source size. Ten open READMEs is
  nothing; a hundred 5 MB documents would need an eviction policy, and this design does not
  have one.

## Alternatives considered

**Keep `DocumentBlock` and layer a parallel selection index over it.** Add a
`SelectionRegistry` beside the existing renderer, keyed by the `LinkSurfaceKey` scheme
already in place, and leave parse and render untouched. Hides less: the caller — really the
renderer — now has to keep two orderings consistent, because the registry's paint order and
the block tree's `block_path` order agree only by convention. It exposes the surface-key
scheme to find, copy, and selection alike, so all three depend on one internal decision;
that is textbook information leakage, and it is what makes adding the fourth behaviour cost
the same as the third. It also cannot preserve anything across a reload, because
`block_path_render_index` is positional. Rejected: smaller diff, worse interface.

**Adopt `longbridge/gpui-component`'s `TextSelectionLayer` and native `TextView`.** This is
the honest contender, and it is a lot of working code. Its public protocol is seven types the
participant must operate in sequence — register a handle, submit a registration each frame,
submit runs during paint, read back a projection, paint the ranges yourself. That is a
shallow module by the "callers coordinate several methods to complete one operation" test,
and it would sit _under_ Mdow's renderer rather than replacing it, so the block tree, the
identity problem, and the reload problem all remain. It also brings its own metrics and
component styling into a build whose whole test suite pins Mdow's. Rejected, but the wrapped-
row geometry problem it solves is real, and `frame::range_rects` solves it the same way.

**Make selection a document-wide `EntityInputHandler`, treating the reader as one read-only
text field.** GPUI already routes selection, IME, and copy through that path for inputs, so
the platform does the coordinate work. It requires one flat string for the entire document
with a byte-offset-to-screen mapping — which means either one giant `StyledText` (losing every
block layout: code cards, tables, blockquote rules, images) or a hand-written mapping between
document offsets and per-block layouts, which is the atom registry again with a harder API
bolted on. Hides less and costs more. Rejected.

**Keep colours in compiled runs, Waku-style, and clear the cache on theme change.** Simpler
implementation, one less indirection at paint. It puts an invalidation edge between the theme
and the parse cache that has no business existing, and the failure mode — a stale palette
painting after a system appearance change — is exactly the kind of bug that survives review.
Rejected on invariant count, not on performance.

## Open questions and risks

- Should `reveal` for an offscreen match go through `ScrollHandle::scroll_to_item` on the
  owning top-level block and then refine to the exact atom on the following frame, or should
  the reader keep a running estimate of block heights so it can land exactly in one frame?
  The two-frame version is simple and may visibly settle on long documents. Which do you
  want to see first?
- Is preserving find state and selection across a live reload the behaviour you want, or is
  a save meant to feel like a fresh document? The incremental splice makes preservation
  possible; it does not make it obviously right, and Electron does the opposite today.
- The interface-scale preference (compact / comfortable / large) changes chrome metrics that
  the visual-contract tests currently assert as constants — 28 px tabs, 36 px tab bar, 244 px
  sidebar. Should those tests be reframed as "compact equals the current numbers," or should
  scale only affect the reader column and leave chrome pinned?
- HTML documents reach the same atom pipeline through `html::compile`, which means they get
  selection, find, and outline for free. How much HTML is in scope — a sanitised subset
  matching what the Electron iframe renders, or best-effort over arbitrary pages? The sketch
  assumes the former.
- Footnotes become first-class blocks collected at the document foot, and GFM alerts become a
  blockquote variant. Both change visible output relative to today's `[^label]` plain text.
  Is that in scope for the parity run, or does it want its own pass?
- Risk worth flagging before implementation: `TextLayout::bounds()` panics before prepaint has
  run, and a block spliced by a reload can reach paint with a fresh layout in the same frame.
  Every geometry read in `FrameLog` and `range_rects` has to probe `line_layouts()` first.
  This is a real crash source, and it is the first thing a test should pin.

## Next implementation step

Write `reader/doc.rs` and `reader/markdown.rs` against the existing `document.rs` test suite —
`Doc::compile` producing atoms and blocks, with `AtomBuilder` enforcing the tiling invariant —
then add one test that reparses a document with a mid-file edit and asserts the surrounding
blocks kept their `BlockId`s.

// A very large document takes seconds to parse in full. Its opening screens parse in a few
// milliseconds, so they are shown first while the full parse runs in the worker.

/** Documents at least this long get a preview of their opening while the full parse runs. */
export const PREVIEW_MIN_CHARS = 1024 * 1024
/** The preview covers at least this much of the document, ending on a block boundary. */
const PREVIEW_TARGET_CHARS = 64 * 1024
/** Give up looking for a heading to cut at past this point; fall back to a blank line. */
const PREVIEW_MAX_CHARS = 256 * 1024

const FENCE = /^ {0,3}(`{3,}|~{3,})/
const ATX_HEADING = /^ {0,3}#{1,6}(\s|$)/

function frontmatterEnd(text: string): number {
  if (!text.startsWith('---\n') && !text.startsWith('---\r\n')) return 0
  const close = text.indexOf('\n---', 3)
  return close === -1 ? text.length : close + 4
}

/**
 * Returns the opening of `text`, cut just before a heading (or, failing that, a blank line)
 * outside any code fence, or null when the document is too short to need a preview. Cutting on
 * those boundaries keeps every block in the preview identical to its place in the full render.
 */
export function sliceDocumentHead(text: string): string | null {
  if (text.length < PREVIEW_MIN_CHARS) return null

  let fence: string | null = null
  let blankCut = -1
  let position = frontmatterEnd(text)

  while (position < text.length && position < PREVIEW_MAX_CHARS) {
    const lineEnd = text.indexOf('\n', position)
    const end = lineEnd === -1 ? text.length : lineEnd
    const line = text.slice(position, end)

    const fenceMatch = FENCE.exec(line)
    if (fence) {
      // A closing fence is the same character, at least as long, with nothing after it but
      // spaces; ```bash inside a ``` block is content, not the end of it.
      if (
        fenceMatch &&
        fenceMatch[1][0] === fence[0] &&
        fenceMatch[1].length >= fence.length &&
        line.slice(fenceMatch[0].length).trim() === ''
      ) {
        fence = null
      }
    } else if (fenceMatch) {
      fence = fenceMatch[1]
    } else if (position >= PREVIEW_TARGET_CHARS) {
      if (ATX_HEADING.test(line)) return text.slice(0, position)
      if (blankCut === -1 && line.trim() === '') blankCut = position
    }

    position = end + 1
  }

  return blankCut === -1 ? null : text.slice(0, blankCut)
}

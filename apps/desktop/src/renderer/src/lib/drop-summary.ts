import { isHtmlPath, isMarkdownPath } from './path-utils'

export interface DropSummary {
  markdown: number
  html: number
  folders: number
  /** Entries whose kind can't be known yet (no name or MIME type during drag-over). */
  unknown: number
  /** Entries Mdow can't open (images, PDFs, …). */
  unsupported: number
}

const EMPTY: DropSummary = { markdown: 0, html: 0, folders: 0, unknown: 0, unsupported: 0 }

const MARKDOWN_MIME = new Set(['text/markdown', 'text/x-markdown'])
const HTML_MIME = new Set(['text/html'])

/**
 * Classify drag-over items. Chromium hides file names and paths until the drop, so this works
 * from MIME types: known Markdown/HTML types count as documents, anything else with a type is
 * unsupported, and untyped entries (folders, or extensions the OS has no type for, like `.mdx`)
 * stay unknown rather than being guessed at.
 */
export function summarizeDragItems(
  items: ReadonlyArray<{ kind: string; type: string }>,
): DropSummary {
  const summary = { ...EMPTY }
  for (const item of items) {
    if (item.kind !== 'file') continue
    const type = item.type.toLowerCase()
    if (MARKDOWN_MIME.has(type)) summary.markdown += 1
    else if (HTML_MIME.has(type)) summary.html += 1
    else if (type === '') summary.unknown += 1
    else summary.unsupported += 1
  }
  return summary
}

/** Classify dropped entries once their names (and whether they are folders) are known. */
export function summarizeDroppedEntries(
  entries: ReadonlyArray<{ name: string; isDirectory: boolean }>,
): DropSummary {
  const summary = { ...EMPTY }
  for (const entry of entries) {
    if (entry.isDirectory) summary.folders += 1
    else if (isMarkdownPath(entry.name)) summary.markdown += 1
    else if (isHtmlPath(entry.name)) summary.html += 1
    else summary.unsupported += 1
  }
  return summary
}

function countLabel(count: number, singular: string, plural = `${singular}s`): string {
  return `${count} ${count === 1 ? singular : plural}`
}

export function canOpenDrop(summary: DropSummary): boolean {
  return summary.markdown + summary.html + summary.folders + summary.unknown > 0
}

/** One line such as "3 Markdown files · 1 folder", or "Nothing Mdow can open". */
export function describeDrop(summary: DropSummary): string {
  if (!canOpenDrop(summary)) return 'Nothing Mdow can open'
  const parts: string[] = []
  if (summary.markdown) parts.push(countLabel(summary.markdown, 'Markdown file'))
  if (summary.html) parts.push(countLabel(summary.html, 'HTML file'))
  if (summary.folders) parts.push(countLabel(summary.folders, 'folder'))
  if (summary.unknown) parts.push(countLabel(summary.unknown, 'item'))
  return parts.join(' · ')
}

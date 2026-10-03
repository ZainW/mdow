import { useMemo, type MouseEvent } from 'react'
import { cn } from '@renderer/lib/utils'

/** Maps a path the model mentioned to a document Mdow can open, or null. */
export type DocumentResolver = (reference: string) => string | null

const DOC_REFERENCE = /^[^\s<>"'`]+\.(?:md|markdown|mdx|html?)(?:#[\w-]*)?$/i

function escapeHtml(text: string): string {
  return text
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;')
}

function docAttr(reference: string, resolve?: DocumentResolver): string {
  if (!resolve || !DOC_REFERENCE.test(reference)) return ''
  const path = resolve(reference.replace(/#.*$/, ''))
  return path ? ` data-doc-path="${escapeHtml(path)}" role="link" tabindex="0"` : ''
}

function renderInline(text: string, resolve?: DocumentResolver): string {
  const codeSpans: string[] = []
  // Pull code spans out first so their contents are never treated as emphasis or links.
  const withoutCode = text.replace(/`([^`]+)`/g, (_, code: string) => {
    const doc = docAttr(code, resolve)
    codeSpans.push(
      `<code class="rounded bg-muted px-1 py-px text-[0.85em]${doc ? ' companion-doc-ref' : ''}"${doc}>${escapeHtml(code)}</code>`,
    )
    return `\uE000${codeSpans.length - 1}\uE001`
  })

  return escapeHtml(withoutCode)
    .replace(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>')
    .replace(/(?<![*\w])\*([^*\s][^*]*)\*(?![*\w])/g, '<em>$1</em>')
    .replace(/(?<![\w])_([^_\s][^_]*)_(?![\w])/g, '<em>$1</em>')
    .replace(/~~([^~]+)~~/g, '<del>$1</del>')
    .replace(/\[([^\]]+)\]\(([^)\s]+)\)/g, (_match, label: string, target: string) => {
      if (/^https?:\/\//.test(target)) {
        return `<a href="${target}" class="text-primary underline underline-offset-2" rel="noreferrer" target="_blank">${label}</a>`
      }
      const doc = docAttr(target.replaceAll('&amp;', '&'), resolve)
      return doc
        ? `<a class="companion-doc-ref text-primary underline underline-offset-2"${doc}>${label}</a>`
        : label
    })
    .replace(/\uE000(\d+)\uE001/g, (_, index: string) => codeSpans[Number(index)])
}

function splitTableRow(line: string): string[] {
  return line
    .trim()
    .replace(/^\|/, '')
    .replace(/\|$/, '')
    .split('|')
    .map((cell) => cell.trim())
}

const TABLE_DIVIDER = /^\s*\|?\s*:?-{2,}:?\s*(\|\s*:?-{2,}:?\s*)*\|?\s*$/

export function markdownToHtml(source: string, resolve?: DocumentResolver): string {
  const lines = source.replace(/\r\n/g, '\n').split('\n')
  const html: string[] = []
  let inCode = false
  let codeLang = ''
  let codeLines: string[] = []
  let inList: 'ul' | 'ol' | null = null
  let quote: string[] = []

  const closeList = () => {
    if (inList) {
      html.push(`</${inList}>`)
      inList = null
    }
  }
  const closeQuote = () => {
    if (quote.length === 0) return
    html.push(
      `<blockquote class="border-l-2 border-border pl-3 text-muted-foreground">${markdownToHtml(quote.join('\n'), resolve)}</blockquote>`,
    )
    quote = []
  }
  const flushCode = () => {
    const label = codeLang
      ? `<div class="mb-1 font-sans text-[10px] tracking-wide text-muted-foreground uppercase">${escapeHtml(codeLang)}</div>`
      : ''
    html.push(
      `<pre class="overflow-x-auto rounded-md bg-muted p-2 text-[0.8em] leading-5">${label}<code>${escapeHtml(codeLines.join('\n'))}</code></pre>`,
    )
    codeLines = []
    codeLang = ''
  }

  for (let index = 0; index < lines.length; index += 1) {
    const line = lines[index]
    if (line.trimStart().startsWith('```')) {
      if (inCode) {
        flushCode()
        inCode = false
      } else {
        closeList()
        closeQuote()
        inCode = true
        codeLang = line.trim().slice(3).trim()
      }
      continue
    }
    if (inCode) {
      codeLines.push(line)
      continue
    }

    const quoted = /^\s*>\s?(.*)$/.exec(line)
    if (quoted) {
      closeList()
      quote.push(quoted[1])
      continue
    }
    closeQuote()

    if (line.includes('|') && TABLE_DIVIDER.test(lines[index + 1] ?? '')) {
      closeList()
      const head = splitTableRow(line)
      const rows: string[][] = []
      index += 2
      while (index < lines.length && lines[index].includes('|') && lines[index].trim()) {
        rows.push(splitTableRow(lines[index]))
        index += 1
      }
      index -= 1
      const th = head
        .map(
          (cell) =>
            `<th class="px-2 py-1 text-left font-medium">${renderInline(cell, resolve)}</th>`,
        )
        .join('')
      const body = rows
        .map(
          (row) =>
            `<tr class="border-t border-border-subtle">${row
              .map((cell) => `<td class="px-2 py-1 align-top">${renderInline(cell, resolve)}</td>`)
              .join('')}</tr>`,
        )
        .join('')
      html.push(
        `<div class="overflow-x-auto rounded-md border border-border-subtle"><table class="w-full text-[0.9em]"><thead class="bg-muted/50"><tr>${th}</tr></thead><tbody>${body}</tbody></table></div>`,
      )
      continue
    }

    const heading = /^(#{1,6})\s+(.*)$/.exec(line)
    if (heading) {
      closeList()
      const level = Math.min(heading[1].length + 2, 6)
      html.push(
        `<h${level} class="mt-1 font-semibold tracking-tight">${renderInline(heading[2], resolve)}</h${level}>`,
      )
      continue
    }

    if (/^\s*([-*_])(\s*\1){2,}\s*$/.test(line)) {
      closeList()
      html.push('<hr class="border-border-subtle" />')
      continue
    }

    const ol = /^\s*(\d+)[.)]\s+(.*)$/.exec(line)
    if (ol) {
      if (inList !== 'ol') {
        closeList()
        html.push('<ol class="list-decimal space-y-1 pl-5">')
        inList = 'ol'
      }
      html.push(`<li>${renderInline(ol[2], resolve)}</li>`)
      continue
    }

    const ul = /^\s*[-*+]\s+(.*)$/.exec(line)
    if (ul) {
      if (inList !== 'ul') {
        closeList()
        html.push('<ul class="list-disc space-y-1 pl-5">')
        inList = 'ul'
      }
      const task = /^\[([ xX])\]\s+(.*)$/.exec(ul[1])
      html.push(
        task
          ? `<li class="list-none -ml-5 flex gap-2"><span aria-hidden="true">${task[1] === ' ' ? '☐' : '☑'}</span><span>${renderInline(task[2], resolve)}</span></li>`
          : `<li>${renderInline(ul[1], resolve)}</li>`,
      )
      continue
    }

    if (!line.trim()) {
      closeList()
      continue
    }

    closeList()
    html.push(`<p class="leading-6">${renderInline(line, resolve)}</p>`)
  }

  if (inCode) flushCode()
  closeQuote()
  closeList()
  return html.join('')
}

function openDocumentFrom(event: MouseEvent<HTMLElement>) {
  const target = (event.target as HTMLElement).closest<HTMLElement>('[data-doc-path]')
  const path = target?.dataset.docPath
  if (!path) return
  event.preventDefault()
  window.dispatchEvent(new CustomEvent('mdow:open-document-link', { detail: { path } }))
}

export function CompanionMarkdown({
  text,
  streaming = false,
  resolveDocument,
  className,
}: {
  text: string
  streaming?: boolean
  resolveDocument?: DocumentResolver
  className?: string
}) {
  const html = useMemo(() => markdownToHtml(text, resolveDocument), [text, resolveDocument])

  if (!text) return null

  return (
    // oxlint-disable-next-line jsx-a11y/click-events-have-key-events, jsx-a11y/no-static-element-interactions -- delegates clicks on document links rendered into the HTML; each link is focusable and handled by onKeyDown below
    <div
      className={cn(
        'companion-md flex flex-col gap-2 break-words [&_code]:font-mono [&_pre]:font-mono',
        '[&_.companion-doc-ref]:cursor-pointer [&_code.companion-doc-ref]:underline [&_code.companion-doc-ref]:decoration-dotted [&_code.companion-doc-ref]:underline-offset-2',
        streaming &&
          'after:ml-0.5 after:inline-block motion-safe:after:animate-pulse after:content-["▍"]',
        className,
      )}
      onClick={openDocumentFrom}
      onKeyDown={(event) => {
        if (event.key === 'Enter') openDocumentFrom(event as unknown as MouseEvent<HTMLElement>)
      }}
      dangerouslySetInnerHTML={{ __html: html }}
    />
  )
}

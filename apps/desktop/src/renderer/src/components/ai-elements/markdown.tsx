import { useMemo } from 'react'
import { cn } from '@renderer/lib/utils'

function escapeHtml(text: string): string {
  return text
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;')
}

function renderInline(text: string): string {
  return escapeHtml(text)
    .replace(/`([^`]+)`/g, '<code class="rounded bg-muted px-1 py-px text-[0.85em]">$1</code>')
    .replace(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>')
    .replace(/(?<!\*)\*([^*]+)\*(?!\*)/g, '<em>$1</em>')
    .replace(
      /\[([^\]]+)\]\((https?:\/\/[^)\s]+)\)/g,
      '<a href="$2" class="text-primary underline underline-offset-2" rel="noreferrer" target="_blank">$1</a>',
    )
}

function markdownToHtml(source: string): string {
  const lines = source.replace(/\r\n/g, '\n').split('\n')
  const html: string[] = []
  let inCode = false
  let codeLang = ''
  let codeLines: string[] = []
  let inList: 'ul' | 'ol' | null = null

  const closeList = () => {
    if (inList) {
      html.push(`</${inList}>`)
      inList = null
    }
  }

  for (const line of lines) {
    if (line.startsWith('```')) {
      if (inCode) {
        html.push(
          `<pre class="overflow-x-auto rounded-md bg-muted p-2 text-[0.8em] leading-5"><code>${escapeHtml(codeLines.join('\n'))}</code></pre>`,
        )
        inCode = false
        codeLines = []
        codeLang = ''
      } else {
        closeList()
        inCode = true
        codeLang = line.slice(3).trim()
        void codeLang
      }
      continue
    }
    if (inCode) {
      codeLines.push(line)
      continue
    }

    const heading = /^(#{1,3})\s+(.*)$/.exec(line)
    if (heading) {
      closeList()
      const level = heading[1].length
      html.push(
        `<h${level} class="font-semibold tracking-tight">${renderInline(heading[2])}</h${level}>`,
      )
      continue
    }

    const ol = /^(\d+)\.\s+(.*)$/.exec(line)
    if (ol) {
      if (inList !== 'ol') {
        closeList()
        html.push('<ol class="list-decimal space-y-1 pl-5">')
        inList = 'ol'
      }
      html.push(`<li>${renderInline(ol[2])}</li>`)
      continue
    }

    const ul = /^[-*]\s+(.*)$/.exec(line)
    if (ul) {
      if (inList !== 'ul') {
        closeList()
        html.push('<ul class="list-disc space-y-1 pl-5">')
        inList = 'ul'
      }
      html.push(`<li>${renderInline(ul[1])}</li>`)
      continue
    }

    if (!line.trim()) {
      closeList()
      continue
    }

    closeList()
    html.push(`<p class="leading-6">${renderInline(line)}</p>`)
  }

  if (inCode) {
    html.push(
      `<pre class="overflow-x-auto rounded-md bg-muted p-2 text-[0.8em] leading-5"><code>${escapeHtml(codeLines.join('\n'))}</code></pre>`,
    )
  }
  closeList()
  return html.join('')
}

export function CompanionMarkdown({
  text,
  streaming = false,
  className,
}: {
  text: string
  streaming?: boolean
  className?: string
}) {
  const html = useMemo(() => markdownToHtml(text), [text])

  if (!text) return null

  return (
    <div
      className={cn(
        'companion-md flex flex-col gap-2 break-words [&_code]:font-mono [&_pre]:font-mono',
        streaming && 'after:ml-0.5 after:inline-block after:animate-pulse after:content-["▍"]',
        className,
      )}
      dangerouslySetInnerHTML={{ __html: html }}
    />
  )
}

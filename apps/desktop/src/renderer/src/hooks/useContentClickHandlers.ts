import { useEffect, type RefObject } from 'react'
import type { RenderResult } from '../lib/markdown'
import { isMarkdownPath, resolveRelativePath } from '../lib/path-utils'

export function useContentClickHandlers({
  contentRef,
  tabPath,
  renderResult,
  onOpenMarkdownLink,
}: {
  contentRef: RefObject<HTMLDivElement | null>
  tabPath: string
  renderResult: RenderResult | null
  onOpenMarkdownLink?: (path: string) => void
}): void {
  useEffect(() => {
    const container = contentRef.current
    if (!container) return undefined

    const handleCopyClick = (event: MouseEvent) => {
      if (!(event.target instanceof Element)) return
      const btn = event.target.closest<HTMLButtonElement>('[data-copy-code]')
      if (!btn || !container.contains(btn)) return
      const wrapper = btn.closest('.code-block-wrapper')
      const code = wrapper?.querySelector('pre')?.textContent ?? ''
      void navigator.clipboard.writeText(code).then(() => {
        btn.setAttribute('data-copied', 'true')
        btn.setAttribute('aria-label', 'Copied')
        window.setTimeout(() => {
          btn.removeAttribute('data-copied')
          btn.setAttribute('aria-label', 'Copy code')
        }, 1500)
      })
    }

    const handleLinkClick = (event: MouseEvent) => {
      if (!(event.target instanceof Element)) return
      const anchor = event.target.closest<HTMLAnchorElement>('a')
      if (!anchor || !container.contains(anchor)) return
      const href = anchor.getAttribute('href')
      if (!href) return

      if (href.startsWith('#')) {
        event.preventDefault()
        const id = decodeURIComponent(href.slice(1))
        const target = container.querySelector(`#${CSS.escape(id)}`) ?? document.getElementById(id)
        target?.scrollIntoView({ behavior: 'smooth', block: 'start' })
        return
      }

      if (/^[a-zA-Z][a-zA-Z\d+\-.]*:/.test(href) && !href.startsWith('mdow-local:')) {
        event.preventDefault()
        void window.api.openExternal(href)
        return
      }

      const resolved = resolveRelativePath(tabPath, href)
      if (isMarkdownPath(resolved)) {
        event.preventDefault()
        window.dispatchEvent(
          new CustomEvent('mdow:open-markdown-link', { detail: { path: resolved } }),
        )
        onOpenMarkdownLink?.(resolved)
      }
    }

    container.addEventListener('click', handleCopyClick)
    container.addEventListener('click', handleLinkClick)
    return () => {
      container.removeEventListener('click', handleCopyClick)
      container.removeEventListener('click', handleLinkClick)
    }
  }, [contentRef, tabPath, renderResult, onOpenMarkdownLink])
}

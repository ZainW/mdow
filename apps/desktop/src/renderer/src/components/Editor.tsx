import { useEffect, useLayoutEffect, useMemo, useRef, useState } from 'react'
import { useEditor, EditorContent } from '@tiptap/react'
import { useAppStore, type Tab } from '../store/app-store'
import { editorExtensions } from '../lib/editor/schema'
import { parseMarkdown } from '../lib/editor/parser'
import { serializeMarkdown } from '../lib/editor/serializer'
import { computeHeadingIds } from '../lib/editor/extensions/heading-ids'
import { Search } from '../lib/editor/extensions/search'
import { initMermaid, renderMermaidBlocks, updateMermaidTheme } from '../lib/mermaid'
import { getContentFontFamily, getCodeFontFamily } from './SettingsDialog'
import { SearchBar } from './SearchBar'
import { ConflictBanner } from './ConflictBanner'

interface EditorProps {
  tab: Tab
}

export function Editor({ tab }: EditorProps) {
  const wideMode = useAppStore((s) => s.wideMode)
  const zoomLevel = useAppStore((s) => s.zoomLevel)
  const updateTabScroll = useAppStore((s) => s.updateTabScroll)
  const contentFont = useAppStore((s) => s.contentFont)
  const codeFont = useAppStore((s) => s.codeFont)
  const fontSize = useAppStore((s) => s.fontSize)
  const lineHeight = useAppStore((s) => s.lineHeight)
  const setDocHeadings = useAppStore((s) => s.setDocHeadings)
  const setActiveHeadingId = useAppStore((s) => s.setActiveHeadingId)
  const searchOpen = useAppStore((s) => s.searchOpen)
  const setSearchOpen = useAppStore((s) => s.setSearchOpen)
  const markTabWritten = useAppStore((s) => s.markTabWritten)
  const conflict = useAppStore((s) => s.tabConflicts[tab.id])

  const [searchMatchCount, setSearchMatchCount] = useState(0)
  const [searchCurrentIndex, setSearchCurrentIndex] = useState(-1)

  const scrollRef = useRef<HTMLDivElement>(null)
  const containerRef = useRef<HTMLDivElement>(null)
  const prevTabIdRef = useRef(tab.id)

  // Parse markdown to ProseMirror JSON (memoized on tab.id — intentionally omits tab.content
  // because useEditor recreates the editor on tab.id change; content changes are handled below).
  // oxlint-disable-next-line react-hooks/exhaustive-deps
  const initialContent = useMemo(() => parseMarkdown(tab.content).toJSON(), [tab.id])

  const editor = useEditor(
    {
      extensions: [...editorExtensions, Search],
      content: initialContent,
      editable: tab.mode === 'edit',
    },
    [tab.id],
  )

  // External content change: re-set content if tab.content changed.
  useEffect(() => {
    if (!editor) return
    const next = parseMarkdown(tab.content).toJSON()
    const current = editor.getJSON()
    if (JSON.stringify(current) !== JSON.stringify(next)) {
      editor.commands.setContent(next, { emitUpdate: false })
    }
  }, [editor, tab.content])

  // Auto-save edits (debounced).
  useEffect(() => {
    if (!editor) return undefined
    let timer: ReturnType<typeof setTimeout> | undefined
    const handler = () => {
      if (timer) clearTimeout(timer)
      timer = setTimeout(() => {
        const markdown = serializeMarkdown(editor.state.doc)
        if (markdown === tab.content) return
        void (async () => {
          try {
            await window.api.writeFile(tab.path, markdown)
            markTabWritten(tab.path, Date.now())
          } catch (err) {
            console.error('Auto-save failed:', err)
          }
        })()
      }, 500)
    }
    editor.on('update', handler)
    return () => {
      editor.off('update', handler)
      if (timer) clearTimeout(timer)
    }
  }, [editor, tab.path, tab.content, markTabWritten])

  // Update editability when tab mode changes.
  useEffect(() => {
    if (editor) editor.setEditable(tab.mode === 'edit')
  }, [editor, tab.mode])

  // Heading IDs + initial active heading.
  useEffect(() => {
    if (!editor || !containerRef.current) return
    const headings = computeHeadingIds(containerRef.current)
    setDocHeadings(headings)
    setActiveHeadingId(headings[0]?.id ?? null)
  }, [editor, tab.content, setDocHeadings, setActiveHeadingId])

  // Mermaid init.
  useEffect(() => {
    const isDark = document.documentElement.classList.contains('dark')
    initMermaid(isDark)
  }, [])

  // Mermaid render.
  useEffect(() => {
    if (!containerRef.current) return
    const els = containerRef.current.querySelectorAll<HTMLElement>('div[data-type="mermaid"]')
    const blocks: { id: string; code: string }[] = []
    els.forEach((el, i) => {
      const id = `mermaid-${i}`
      el.id = id
      const source = el.getAttribute('data-source') ?? ''
      blocks.push({ id, code: source })
    })
    if (blocks.length > 0) void renderMermaidBlocks(blocks)
  }, [editor, tab.content])

  // Theme observer.
  useEffect(() => {
    const observer = new MutationObserver(() => {
      updateMermaidTheme(document.documentElement.classList.contains('dark'))
    })
    observer.observe(document.documentElement, { attributes: true, attributeFilter: ['class'] })
    return () => observer.disconnect()
  }, [])

  // Copy-code button (delegated click handler).
  useEffect(() => {
    const container = containerRef.current
    if (!container) return undefined
    const handler = (e: MouseEvent) => {
      if (!(e.target instanceof HTMLElement)) return
      const btn = e.target.closest('.copy-code-btn')
      if (!btn) return
      const encoded = btn.getAttribute('data-code')
      if (!encoded) return
      const code = decodeURIComponent(atob(encoded))
      void navigator.clipboard.writeText(code)
      btn.textContent = 'Copied!'
      setTimeout(() => {
        btn.textContent = 'Copy'
      }, 2000)
    }
    container.addEventListener('click', handler)
    return () => container.removeEventListener('click', handler)
  }, [editor, tab.content])

  // Scroll-spy.
  useEffect(() => {
    const root = scrollRef.current
    const container = containerRef.current
    if (!root || !container) return undefined
    const headingEls = container.querySelectorAll<HTMLElement>('h1[id], h2[id], h3[id], h4[id]')
    if (headingEls.length === 0) return undefined
    const observer = new IntersectionObserver(
      (entries) => {
        const visible = entries
          .filter((e) => e.isIntersecting)
          .toSorted((a, b) => a.boundingClientRect.top - b.boundingClientRect.top)
        if (visible[0]) setActiveHeadingId(visible[0].target.id)
      },
      { root, rootMargin: '0px 0px -75% 0px', threshold: 0 },
    )
    for (const el of headingEls) observer.observe(el)
    return () => observer.disconnect()
  }, [editor, tab.content, setActiveHeadingId])

  // Scroll save (debounced).
  useEffect(() => {
    const el = scrollRef.current
    if (!el) return undefined
    let timer: ReturnType<typeof setTimeout>
    const handler = () => {
      clearTimeout(timer)
      timer = setTimeout(() => {
        updateTabScroll(tab.id, el.scrollTop)
      }, 150)
    }
    el.addEventListener('scroll', handler, { passive: true })
    return () => {
      clearTimeout(timer)
      el.removeEventListener('scroll', handler)
    }
  }, [tab.id, updateTabScroll])

  // Scroll restore on tab switch.
  useLayoutEffect(() => {
    if (prevTabIdRef.current !== tab.id) {
      const el = scrollRef.current
      if (el) el.scrollTo(0, tab.scrollPosition)
      prevTabIdRef.current = tab.id
    }
  }, [tab.id, tab.scrollPosition])

  const handleQueryChange = (q: string) => {
    if (!editor) return
    editor.commands.setSearchQuery(q)
    setSearchMatchCount(editor.storage.search.matchCount)
    setSearchCurrentIndex(editor.storage.search.currentIndex)
  }

  const handleNext = () => {
    if (!editor) return
    editor.commands.nextMatch()
    setSearchCurrentIndex(editor.storage.search.currentIndex)
  }

  const handlePrev = () => {
    if (!editor) return
    editor.commands.prevMatch()
    setSearchCurrentIndex(editor.storage.search.currentIndex)
  }

  const handleCloseSearch = () => {
    if (editor) editor.commands.setSearchQuery('')
    setSearchMatchCount(0)
    setSearchCurrentIndex(-1)
    setSearchOpen(false)
  }

  return (
    <div ref={scrollRef} className="group/content relative flex-1 overflow-y-auto">
      {conflict !== undefined && <ConflictBanner tabId={tab.id} diskContent={conflict} />}
      {searchOpen && (
        <SearchBar
          matchCount={searchMatchCount}
          currentIndex={searchCurrentIndex}
          onNext={handleNext}
          onPrev={handlePrev}
          onClose={handleCloseSearch}
          onQueryChange={handleQueryChange}
        />
      )}
      <div
        ref={containerRef}
        className="mx-auto px-12 py-8 text-foreground markdown-body transition-[max-width] duration-200 ease-[cubic-bezier(0.23,1,0.32,1)]"
        style={
          {
            maxWidth: wideMode ? '100%' : '52rem',
            '--md-content-font': getContentFontFamily(contentFont),
            '--md-code-font': getCodeFontFamily(codeFont),
            '--md-font-size': `${fontSize * (zoomLevel / 100)}px`,
            '--md-line-height': String(lineHeight),
          } as React.CSSProperties
        }
      >
        <EditorContent editor={editor} />
      </div>
    </div>
  )
}

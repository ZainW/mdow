import {
  useCallback,
  useEffect,
  useId,
  useMemo,
  useRef,
  useState,
  type KeyboardEvent as ReactKeyboardEvent,
} from 'react'
import { useNavigate } from '@tanstack/react-router'
import { useFocusTrap } from '~/hooks/use-focus-trap'
import { formatShortcut, useModKey } from '~/hooks/use-mod-key'
import { buildSearchIndex, search, type SearchEntry } from '~/lib/search-index'
import { cn } from '~/lib/utils'
import { FileIcon, SearchIcon } from './icons'

export function DocsSearchTrigger({
  onOpen,
  className,
}: {
  onOpen: () => void
  className?: string
}) {
  const modKey = useModKey()
  return (
    <button
      type="button"
      onClick={onOpen}
      className={cn(
        'flex h-10 w-full items-center gap-2.5 rounded-lg border border-border bg-card px-3 text-sm text-muted-foreground',
        'transition-[border-color,color] duration-150 ease hover:border-border-strong hover:text-foreground',
        className,
      )}
    >
      <SearchIcon className="size-4" />
      <span>Search docs</span>
      {modKey && <kbd className="kbd ml-auto">{formatShortcut(modKey, 'K')}</kbd>}
    </button>
  )
}

interface DocsSearchDialogProps {
  entries: SearchEntry[]
  open: boolean
  onOpenChange: (open: boolean) => void
}

/** The one search dialog for the docs. Owns the ⌘K / Ctrl+K shortcut. */
export function DocsSearchDialog({ entries, open, onOpenChange }: DocsSearchDialogProps) {
  const [query, setQuery] = useState('')
  const [selected, setSelected] = useState(0)
  const inputRef = useRef<HTMLInputElement>(null)
  const dialogRef = useRef<HTMLDivElement>(null)
  const listRef = useRef<HTMLUListElement>(null)
  const listId = useId()
  const navigate = useNavigate()

  useFocusTrap(open, dialogRef)

  useEffect(() => {
    buildSearchIndex(entries)
  }, [entries])

  useEffect(() => {
    function onKeyDown(e: KeyboardEvent) {
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'k') {
        e.preventDefault()
        onOpenChange(!open)
      }
    }
    window.addEventListener('keydown', onKeyDown)
    return () => window.removeEventListener('keydown', onKeyDown)
  }, [open, onOpenChange])

  useEffect(() => {
    if (!open) return
    setQuery('')
    setSelected(0)
    inputRef.current?.focus()
    document.body.style.overflow = 'hidden'
    return () => {
      document.body.style.overflow = ''
    }
  }, [open])

  // With no query, list every page so the dialog doubles as a quick switcher.
  const results = useMemo(
    () => (query.trim() ? search(query) : entries.filter((e) => !e.section)),
    [query, entries],
  )

  useEffect(() => {
    listRef.current
      ?.querySelector(`[data-index="${selected}"]`)
      ?.scrollIntoView({ block: 'nearest' })
  }, [selected])

  const goTo = useCallback(
    (entry: SearchEntry) => {
      onOpenChange(false)
      navigate({
        to: '/docs/$',
        params: { _splat: entry.slug },
        hash: entry.section?.id,
      })
    },
    [navigate, onOpenChange],
  )

  function onInputKeyDown(e: ReactKeyboardEvent) {
    if (e.key === 'ArrowDown') {
      e.preventDefault()
      setSelected((s) => Math.min(s + 1, results.length - 1))
    } else if (e.key === 'ArrowUp') {
      e.preventDefault()
      setSelected((s) => Math.max(s - 1, 0))
    } else if (e.key === 'Enter' && results[selected]) {
      e.preventDefault()
      goTo(results[selected])
    } else if (e.key === 'Escape') {
      onOpenChange(false)
    }
  }

  if (!open) return null

  return (
    <div className="fixed inset-0 z-modal flex items-start justify-center px-4 pt-[12vh]">
      <div
        className="animate-overlay-in fixed inset-0 bg-foreground/15 backdrop-blur-[2px] dark:bg-black/50"
        onClick={() => onOpenChange(false)}
        aria-hidden
      />
      <div
        ref={dialogRef}
        role="dialog"
        aria-modal="true"
        aria-label="Search documentation"
        className="animate-pop-in relative w-full max-w-xl overflow-hidden rounded-2xl border border-border bg-popover shadow-[0_24px_64px_-16px_hsl(var(--shadow-color)/0.35)]"
      >
        <div className="flex items-center gap-3 border-b border-border-subtle px-4">
          <SearchIcon className="size-4 shrink-0 text-muted-foreground" />
          <input
            ref={inputRef}
            value={query}
            onChange={(e) => {
              setQuery(e.target.value)
              setSelected(0)
            }}
            onKeyDown={onInputKeyDown}
            placeholder="Search docs…"
            role="combobox"
            aria-expanded="true"
            aria-controls={listId}
            aria-activedescendant={results[selected] ? `${listId}-${selected}` : undefined}
            className="h-14 flex-1 bg-transparent text-base outline-none placeholder:text-faint sm:text-[15px]"
          />
          <button
            type="button"
            onClick={() => onOpenChange(false)}
            className="kbd cursor-pointer hover:text-foreground"
            aria-label="Close search"
          >
            esc
          </button>
        </div>
        {results.length > 0 ? (
          <ul
            ref={listRef}
            id={listId}
            role="listbox"
            aria-label="Results"
            className="max-h-[min(24rem,60vh)] overflow-y-auto p-2"
          >
            {results.map((r, i) => (
              <li
                key={`${r.slug}#${r.section?.id ?? ''}`}
                id={`${listId}-${i}`}
                role="option"
                aria-selected={i === selected}
                data-index={i}
              >
                <button
                  type="button"
                  tabIndex={-1}
                  onMouseMove={() => setSelected(i)}
                  onClick={() => goTo(r)}
                  className={cn(
                    'flex w-full items-start gap-3 rounded-lg px-3 py-2.5 text-left',
                    i === selected && 'bg-muted',
                  )}
                >
                  <span
                    className={cn(
                      'mt-0.5 flex size-6 shrink-0 items-center justify-center rounded-md border border-border-subtle bg-card text-muted-foreground',
                      r.section && 'border-transparent bg-transparent font-mono text-sm',
                    )}
                    aria-hidden
                  >
                    {r.section ? '#' : <FileIcon className="size-3.5" />}
                  </span>
                  <span className="min-w-0">
                    <span className="block truncate text-sm font-medium text-foreground">
                      {r.section ? r.section.text : r.title}
                    </span>
                    <span className="mt-0.5 block truncate text-xs text-muted-foreground">
                      {r.section ? `${r.category} · ${r.title}` : r.description || r.category}
                    </span>
                  </span>
                </button>
              </li>
            ))}
          </ul>
        ) : (
          <p className="px-4 py-10 text-center text-sm text-muted-foreground">
            No results for &ldquo;{query}&rdquo;
          </p>
        )}
        <div className="hidden items-center gap-4 border-t border-border-subtle bg-surface/60 px-4 py-2.5 text-xs text-muted-foreground sm:flex">
          <span className="flex items-center gap-1.5">
            <kbd className="kbd">↑</kbd>
            <kbd className="kbd">↓</kbd>
            to navigate
          </span>
          <span className="flex items-center gap-1.5">
            <kbd className="kbd">↵</kbd>
            to open
          </span>
        </div>
      </div>
    </div>
  )
}

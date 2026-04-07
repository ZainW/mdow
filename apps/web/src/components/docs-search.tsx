import { useState, useEffect, useCallback, useRef } from 'react'
import { useNavigate } from '@tanstack/react-router'
import type { DocMeta } from '~/lib/content'
import { buildSearchIndex, search } from '~/lib/search-index'
import { cn } from '~/lib/utils'

interface DocsSearchProps {
  docs: DocMeta[]
}

export function DocsSearch({ docs }: DocsSearchProps) {
  const [open, setOpen] = useState(false)
  const [query, setQuery] = useState('')
  const [results, setResults] = useState<ReturnType<typeof search>>([])
  const [selected, setSelected] = useState(0)
  const inputRef = useRef<HTMLInputElement>(null)
  const navigate = useNavigate()

  useEffect(() => {
    buildSearchIndex(docs)
  }, [docs])

  useEffect(() => {
    function handleKeyDown(e: KeyboardEvent) {
      if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
        e.preventDefault()
        setOpen((prev) => !prev)
      }
    }
    window.addEventListener('keydown', handleKeyDown)
    return () => window.removeEventListener('keydown', handleKeyDown)
  }, [])

  useEffect(() => {
    if (open) {
      inputRef.current?.focus()
      setQuery('')
      setResults([])
      setSelected(0)
    }
  }, [open])

  const onQueryChange = useCallback((value: string) => {
    setQuery(value)
    setResults(search(value))
    setSelected(0)
  }, [])

  const goTo = useCallback(
    (slug: string) => {
      setOpen(false)
      navigate({ to: '/docs/$', params: { _splat: slug } })
    },
    [navigate],
  )

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === 'ArrowDown') {
        e.preventDefault()
        setSelected((s) => Math.min(s + 1, results.length - 1))
      } else if (e.key === 'ArrowUp') {
        e.preventDefault()
        setSelected((s) => Math.max(s - 1, 0))
      } else if (e.key === 'Enter' && results[selected]) {
        goTo(results[selected].slug)
      } else if (e.key === 'Escape') {
        setOpen(false)
      }
    },
    [results, selected, goTo],
  )

  if (!open) {
    return (
      <button
        type="button"
        onClick={() => setOpen(true)}
        className="flex w-full items-center gap-2 rounded-md border border-border-subtle bg-surface px-3 py-2 text-sm text-muted-foreground transition-colors hover:bg-muted"
      >
        <svg
          width="14"
          height="14"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
          aria-hidden
        >
          <circle cx="11" cy="11" r="8" />
          <path d="m21 21-4.3-4.3" />
        </svg>
        <span>Search docs</span>
        <kbd className="ml-auto rounded border border-border bg-background px-1.5 py-0.5 text-[10px] font-mono">
          ⌘K
        </kbd>
      </button>
    )
  }

  return (
    <div className="fixed inset-0 z-50 flex items-start justify-center pt-[15vh]">
      <div
        className="fixed inset-0 bg-background/70 backdrop-blur-md"
        onClick={() => setOpen(false)}
        onKeyDown={(e) => {
          if (e.key === 'Escape') setOpen(false)
        }}
        role="button"
        tabIndex={-1}
        aria-label="Close search"
      />
      <div className="relative z-10 w-full max-w-xl overflow-hidden rounded-xl border border-border-subtle bg-popover shadow-soft-lg">
        <div className="flex items-center gap-3 border-b border-border-subtle px-4">
          <svg
            width="16"
            height="16"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            strokeLinecap="round"
            strokeLinejoin="round"
            className="text-muted-foreground"
            aria-hidden
          >
            <circle cx="11" cy="11" r="8" />
            <path d="m21 21-4.3-4.3" />
          </svg>
          <input
            ref={inputRef}
            value={query}
            onChange={(e) => onQueryChange(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder="Search docs..."
            className="flex-1 bg-transparent py-4 text-sm outline-none placeholder:text-muted-foreground"
          />
          <kbd className="rounded border border-border bg-background px-1.5 py-0.5 text-[10px] font-mono text-muted-foreground">
            esc
          </kbd>
        </div>
        {results.length > 0 && (
          <ul className="max-h-80 overflow-y-auto py-2">
            {results.map((r, i) => (
              <li key={r.slug}>
                <button
                  type="button"
                  onMouseEnter={() => setSelected(i)}
                  onClick={() => goTo(r.slug)}
                  className={cn(
                    'flex w-full flex-col items-start gap-0.5 px-4 py-2.5 text-left transition-colors',
                    i === selected ? 'bg-surface' : '',
                  )}
                >
                  <span className="text-sm font-medium text-foreground">{r.title}</span>
                  {r.description && (
                    <span className="line-clamp-1 text-xs text-muted-foreground">
                      {r.description}
                    </span>
                  )}
                  <span className="mt-0.5 text-[10px] uppercase tracking-wider text-muted-foreground">
                    {r.category}
                  </span>
                </button>
              </li>
            ))}
          </ul>
        )}
        {query && results.length === 0 && (
          <p className="px-4 py-8 text-center text-sm text-muted-foreground">No results found.</p>
        )}
        <div className="flex items-center gap-3 border-t border-border-subtle bg-surface/50 px-4 py-2 text-[10px] text-muted-foreground">
          <span className="flex items-center gap-1">
            <kbd className="rounded border border-border bg-background px-1 font-mono">↑↓</kbd>{' '}
            navigate
          </span>
          <span className="flex items-center gap-1">
            <kbd className="rounded border border-border bg-background px-1 font-mono">↵</kbd>{' '}
            select
          </span>
        </div>
      </div>
    </div>
  )
}

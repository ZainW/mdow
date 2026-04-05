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
        className="flex w-full items-center gap-2 rounded-md border bg-muted/50 px-3 py-1.5 text-sm text-muted-foreground transition-colors hover:bg-muted"
      >
        <span>Search docs...</span>
        <kbd className="ml-auto rounded border bg-background px-1.5 py-0.5 text-xs">&#8984;K</kbd>
      </button>
    )
  }

  return (
    <div className="fixed inset-0 z-50 flex items-start justify-center pt-[20vh]">
      <div
        className="fixed inset-0 bg-background/60 backdrop-blur-sm"
        onClick={() => setOpen(false)}
        onKeyDown={(e) => {
          if (e.key === 'Escape') setOpen(false)
        }}
        role="button"
        tabIndex={-1}
        aria-label="Close search"
      />
      <div className="relative z-10 w-full max-w-lg rounded-lg border bg-popover shadow-lg">
        <input
          ref={inputRef}
          value={query}
          onChange={(e) => onQueryChange(e.target.value)}
          onKeyDown={handleKeyDown}
          placeholder="Search docs..."
          className="w-full rounded-t-lg border-b bg-transparent px-4 py-3 text-sm outline-none"
        />
        {results.length > 0 && (
          <ul className="max-h-64 overflow-y-auto py-2">
            {results.map((r, i) => (
              <li key={r.slug}>
                <button
                  type="button"
                  onClick={() => goTo(r.slug)}
                  className={cn(
                    'w-full px-4 py-2 text-left text-sm transition-colors',
                    i === selected ? 'bg-muted text-foreground' : 'text-muted-foreground',
                  )}
                >
                  <span className="font-medium">{r.title}</span>
                  <span className="ml-2 text-xs text-muted-foreground">{r.category}</span>
                </button>
              </li>
            ))}
          </ul>
        )}
        {query && results.length === 0 && (
          <p className="px-4 py-6 text-center text-sm text-muted-foreground">No results found.</p>
        )}
      </div>
    </div>
  )
}

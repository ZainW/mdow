import { useEffect, useRef, useState } from 'react'
import { ChevronUp, ChevronDown, Search, X } from 'lucide-react'
import { cn } from '../lib/utils'

interface SearchBarProps {
  matchCount: number
  currentIndex: number
  onNext: () => void
  onPrev: () => void
  onClose: () => void
  onQueryChange: (query: string) => void
}

export function formatMatchCount(query: string, matchCount: number, currentIndex: number): string {
  if (!query) return ''
  if (matchCount === 0) return 'No results'
  return `${currentIndex + 1} / ${matchCount}`
}

export function SearchBar({
  matchCount,
  currentIndex,
  onNext,
  onPrev,
  onClose,
  onQueryChange,
}: SearchBarProps) {
  const inputRef = useRef<HTMLInputElement>(null)
  const [query, setQuery] = useState('')

  useEffect(() => {
    inputRef.current?.focus()
    inputRef.current?.select()
  }, [])

  const handleChange = (value: string) => {
    setQuery(value)
    onQueryChange(value)
  }

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter') {
      e.preventDefault()
      if (e.shiftKey) {
        onPrev()
      } else {
        onNext()
      }
    }
    if (e.key === 'Escape') {
      e.preventDefault()
      onClose()
    }
  }

  return (
    <div className="search-bar floating-surface pointer-events-auto mt-2.5 mr-4 flex h-[38px] w-[340px] items-center gap-1.5 rounded-lg pr-1.5 pl-2.5 text-[13px]">
      <Search className="size-3.5 shrink-0 text-muted-foreground" aria-hidden />
      <input
        ref={inputRef}
        type="text"
        aria-label="Search in document"
        className="h-full min-w-0 flex-1 bg-transparent text-foreground outline-none placeholder:text-muted-foreground"
        placeholder="Find in document"
        value={query}
        onChange={(e) => handleChange(e.target.value)}
        onKeyDown={handleKeyDown}
      />
      <output
        aria-live="polite"
        aria-atomic="true"
        className={cn(
          'shrink-0 pr-1 text-right text-[11.5px] font-medium tabular-nums',
          query && matchCount === 0 ? 'text-destructive' : 'text-muted-foreground',
        )}
      >
        {formatMatchCount(query, matchCount, currentIndex)}
      </output>
      <span aria-hidden className="h-[18px] w-px shrink-0 bg-border-subtle" />
      <FindButton
        label="Previous match"
        title="Previous match (Shift+Enter)"
        disabled={matchCount === 0}
        onClick={onPrev}
      >
        <ChevronUp className="size-3.5" aria-hidden />
      </FindButton>
      <FindButton
        label="Next match"
        title="Next match (Enter)"
        disabled={matchCount === 0}
        onClick={onNext}
      >
        <ChevronDown className="size-3.5" aria-hidden />
      </FindButton>
      <FindButton label="Close search" title="Close (Esc)" onClick={onClose}>
        <X className="size-[13px]" aria-hidden />
      </FindButton>
    </div>
  )
}

function FindButton({
  label,
  title,
  disabled,
  onClick,
  children,
}: {
  label: string
  title: string
  disabled?: boolean
  onClick: () => void
  children: React.ReactNode
}) {
  return (
    <button
      type="button"
      aria-label={label}
      title={title}
      disabled={disabled}
      onClick={onClick}
      className="flex size-6 shrink-0 items-center justify-center rounded-md text-muted-foreground outline-none hover:bg-muted hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring/50 disabled:pointer-events-none disabled:opacity-40"
    >
      {children}
    </button>
  )
}

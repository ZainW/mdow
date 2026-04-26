import { useEffect, useRef, useState } from 'react'
import { Button } from './ui/button'
import { CaretUp, CaretDown, X } from '@phosphor-icons/react'

interface SearchBarProps {
  matchCount: number
  currentIndex: number
  onNext: () => void
  onPrev: () => void
  onClose: () => void
  onQueryChange: (query: string) => void
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
  }, [])

  useEffect(() => {
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
    <div className="search-bar sticky top-0 z-20 ml-auto flex w-fit items-center gap-1.5 rounded-bl-xl border-b border-l border-border/50 bg-background/90 px-3 py-2 shadow-md backdrop-blur-md">
      <input
        ref={inputRef}
        type="text"
        className="search-input h-6 w-44 rounded-md border border-border/60 bg-muted/40 px-2 text-xs outline-none transition-[border-color,box-shadow] duration-150 focus:border-ring focus:shadow-[0_0_0_1px_hsl(var(--ring)/0.3)]"
        placeholder="Search..."
        value={query}
        onChange={(e) => handleChange(e.target.value)}
        onKeyDown={handleKeyDown}
      />
      <span className="min-w-[4rem] text-center text-[10px] tabular-nums text-muted-foreground">
        {query ? (matchCount > 0 ? `${currentIndex + 1} of ${matchCount}` : 'No results') : ''}
      </span>
      <Button
        variant="ghost"
        size="icon-xs"
        onClick={onPrev}
        disabled={matchCount === 0}
        aria-label="Previous match"
        title="Previous match (Shift+Enter)"
        className="active:scale-90 transition-transform"
      >
        <CaretUp className="size-3.5" aria-hidden />
      </Button>
      <Button
        variant="ghost"
        size="icon-xs"
        onClick={onNext}
        disabled={matchCount === 0}
        aria-label="Next match"
        title="Next match (Enter)"
        className="active:scale-90 transition-transform"
      >
        <CaretDown className="size-3.5" aria-hidden />
      </Button>
      <Button
        variant="ghost"
        size="icon-xs"
        onClick={onClose}
        aria-label="Close search"
        title="Close (Esc)"
        className="active:scale-90 transition-transform"
      >
        <X className="size-3.5" aria-hidden />
      </Button>
    </div>
  )
}

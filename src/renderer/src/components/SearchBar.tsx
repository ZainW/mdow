import { useEffect, useRef, useState } from 'react'
import { Button } from './ui/button'
import { ChevronUpIcon, ChevronDownIcon, XIcon } from 'lucide-react'

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

  // Clear query on close and refocus on reopen
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
    <div className="absolute top-0 right-0 z-20 flex items-center gap-1.5 rounded-bl-lg border-b border-l border-border/60 bg-background/95 px-3 py-1.5 shadow-sm backdrop-blur-sm">
      <input
        ref={inputRef}
        type="text"
        className="h-6 w-44 rounded-md border border-border/60 bg-transparent px-2 text-xs outline-none focus:border-ring"
        placeholder="Search..."
        value={query}
        onChange={(e) => handleChange(e.target.value)}
        onKeyDown={handleKeyDown}
      />
      <span className="min-w-[4rem] text-center text-[10px] text-muted-foreground">
        {query ? (matchCount > 0 ? `${currentIndex + 1} of ${matchCount}` : 'No results') : ''}
      </span>
      <Button variant="ghost" size="icon-xs" onClick={onPrev} disabled={matchCount === 0}>
        <ChevronUpIcon className="size-3.5" />
      </Button>
      <Button variant="ghost" size="icon-xs" onClick={onNext} disabled={matchCount === 0}>
        <ChevronDownIcon className="size-3.5" />
      </Button>
      <Button variant="ghost" size="icon-xs" onClick={onClose}>
        <XIcon className="size-3.5" />
      </Button>
    </div>
  )
}

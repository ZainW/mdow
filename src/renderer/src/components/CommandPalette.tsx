import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { useAppStore } from '../store/app-store'
import { useRecents } from '../hooks/useRecents'
import { fuzzySearch, type SearchResult } from '../lib/fuzzy-search'
import { basename } from '../lib/path-utils'

function flattenTree(nodes: any[], result: { path: string; name: string }[] = []) {
  for (const node of nodes) {
    if (node.isDirectory && node.children) {
      flattenTree(node.children, result)
    } else if (!node.isDirectory) {
      result.push({ path: node.path, name: node.name })
    }
  }
  return result
}

export function CommandPalette() {
  const commandPaletteOpen = useAppStore((s) => s.commandPaletteOpen)
  const setCommandPaletteOpen = useAppStore((s) => s.setCommandPaletteOpen)
  const setActiveFile = useAppStore((s) => s.setActiveFile)
  const folderTree = useAppStore((s) => s.folderTree)
  const { data: recents = [] } = useRecents()
  const queryClient = useQueryClient()

  const [query, setQuery] = useState('')
  const [selectedIndex, setSelectedIndex] = useState(0)
  const inputRef = useRef<HTMLInputElement>(null)

  const allFiles = useMemo(() => {
    const folderFiles = flattenTree(folderTree)
    const folderPaths = new Set(folderFiles.map((f) => f.path))
    const recentFiles = recents
      .filter((path) => !folderPaths.has(path))
      .map((path) => ({ path, name: basename(path) }))
    return [...folderFiles, ...recentFiles]
  }, [folderTree, recents])

  const results = useMemo(() => fuzzySearch(query, allFiles), [query, allFiles])

  useEffect(() => {
    if (commandPaletteOpen) {
      setQuery('')
      setSelectedIndex(0)
      setTimeout(() => inputRef.current?.focus(), 0)
    }
  }, [commandPaletteOpen])

  const selectFile = useCallback(
    async (path: string) => {
      setCommandPaletteOpen(false)
      const content = await window.api.readFile(path)
      setActiveFile({ path, content })
      queryClient.invalidateQueries({ queryKey: ['recents'] })
    },
    [setCommandPaletteOpen, setActiveFile, queryClient]
  )

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      switch (e.key) {
        case 'ArrowDown':
          e.preventDefault()
          setSelectedIndex((i) => Math.min(i + 1, results.length - 1))
          break
        case 'ArrowUp':
          e.preventDefault()
          setSelectedIndex((i) => Math.max(i - 1, 0))
          break
        case 'Enter':
          e.preventDefault()
          if (results[selectedIndex]) {
            selectFile(results[selectedIndex].path)
          }
          break
        case 'Escape':
          e.preventDefault()
          setCommandPaletteOpen(false)
          break
      }
    },
    [results, selectedIndex, selectFile, setCommandPaletteOpen]
  )

  if (!commandPaletteOpen) return null

  return (
    <div className="command-palette-overlay" onClick={() => setCommandPaletteOpen(false)}>
      <div className="command-palette" onClick={(e) => e.stopPropagation()}>
        <input
          ref={inputRef}
          className="command-palette-input"
          placeholder="Search files..."
          value={query}
          onChange={(e) => {
            setQuery(e.target.value)
            setSelectedIndex(0)
          }}
          onKeyDown={handleKeyDown}
        />
        <div className="command-palette-results">
          {results.length === 0 && query && (
            <div className="command-palette-empty">No matching files</div>
          )}
          {results.map((result, index) => (
            <div
              key={result.path}
              className={`command-palette-item ${index === selectedIndex ? 'selected' : ''}`}
              onClick={() => selectFile(result.path)}
              onMouseEnter={() => setSelectedIndex(index)}
            >
              <span className="filename">{result.name}</span>
              <span className="filepath">{result.path}</span>
            </div>
          ))}
        </div>
      </div>
    </div>
  )
}

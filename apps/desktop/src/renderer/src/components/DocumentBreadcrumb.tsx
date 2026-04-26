import { useAppStore, type Tab } from '../store/app-store'
import { basename } from '../lib/path-utils'
import { Button } from './ui/button'
import {
  ArrowsHorizontal,
  ArrowsInLineHorizontal,
  CaretRight,
  Pencil,
  PencilSimpleSlash,
} from '@phosphor-icons/react'

interface Props {
  tab: Tab
}

export function DocumentBreadcrumb({ tab }: Props) {
  const wideMode = useAppStore((s) => s.wideMode)
  const toggleWideMode = useAppStore((s) => s.toggleWideMode)
  const toggleTabMode = useAppStore((s) => s.toggleTabMode)
  const openFolderPath = useAppStore((s) => s.openFolderPath)
  const editing = tab.mode === 'edit'

  const filename = basename(tab.path)
  const segments = parentSegments(tab.path, openFolderPath)

  return (
    <div className="flex h-7 shrink-0 items-center gap-2 border-b border-border-subtle bg-background px-3 text-[11px] text-muted-foreground/80">
      <nav
        aria-label="Document path"
        className="flex min-w-0 flex-1 items-center gap-0.5 overflow-hidden"
      >
        <ol className="flex min-w-0 items-center gap-0.5">
          {segmentsWithPath(segments).map(({ key, name }) => (
            <li key={key} className="flex shrink-0 items-center gap-0.5 last:min-w-0 last:shrink">
              <span className="truncate">{name}</span>
              <CaretRight className="size-2.5 shrink-0 text-muted-foreground/40" />
            </li>
          ))}
        </ol>
        <button
          type="button"
          title="Reveal in Finder"
          onClick={() => void window.api.showInFolder(tab.path)}
          className="breadcrumb-file ml-0.5 truncate rounded px-1 py-0.5 font-medium text-foreground/85 hover:bg-muted hover:text-foreground"
        >
          {filename}
        </button>
      </nav>
      <Button
        variant="ghost"
        size="icon-xs"
        aria-label={editing ? 'Stop editing' : 'Edit document'}
        title={editing ? 'Stop editing (Cmd+E)' : 'Edit (Cmd+E)'}
        aria-pressed={editing}
        onClick={() => toggleTabMode(tab.id)}
        className="text-muted-foreground/70 hover:text-foreground"
      >
        {editing ? <PencilSimpleSlash /> : <Pencil />}
      </Button>
      <Button
        variant="ghost"
        size="icon-xs"
        aria-label={wideMode ? 'Exit wide mode' : 'Wide mode'}
        title={wideMode ? 'Constrained width' : 'Full width'}
        aria-pressed={wideMode}
        onClick={toggleWideMode}
        className="text-muted-foreground/70 hover:text-foreground"
      >
        {wideMode ? <ArrowsInLineHorizontal /> : <ArrowsHorizontal />}
      </Button>
    </div>
  )
}

function segmentsWithPath(segments: string[]): Array<{ key: string; name: string }> {
  let acc = ''
  return segments.map((name) => {
    acc = acc ? `${acc}/${name}` : name
    return { key: acc, name }
  })
}

function parentSegments(filePath: string, rootPath: string | null): string[] {
  const parts = filePath.split(/[/\\]/).filter(Boolean)
  // Drop the filename
  const dirs = parts.slice(0, -1)

  // If there's an open folder, anchor at it: show the folder name + everything under it
  if (rootPath) {
    const rootParts = rootPath.split(/[/\\]/).filter(Boolean)
    const rootName = rootParts.at(-1)
    if (rootName) {
      const idx = dirs.lastIndexOf(rootName)
      if (idx >= 0) {
        return dirs.slice(idx)
      }
    }
  }

  // Otherwise, show up to the last 3 directory segments (keeps things compact)
  return dirs.slice(-3)
}

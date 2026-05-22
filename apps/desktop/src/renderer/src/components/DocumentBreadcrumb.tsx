import { useAppStore, type Tab } from '../store/app-store'
import { basename, detectSep } from '../lib/path-utils'
import { Button } from './ui/button'
import { ArrowLeftRight, ChevronRight, FoldHorizontal } from 'lucide-react'

interface Props {
  tab: Tab
}

export function DocumentBreadcrumb({ tab }: Props) {
  const wideMode = useAppStore((s) => s.wideMode)
  const toggleWideMode = useAppStore((s) => s.toggleWideMode)
  const openFolderPath = useAppStore((s) => s.openFolderPath)

  const filename = basename(tab.path)
  const segments = parentSegmentsWithPaths(tab.path, openFolderPath)

  return (
    <div className="flex h-7 shrink-0 items-center gap-2 border-b border-border-subtle bg-background px-3 text-[11px] text-muted-foreground/80">
      <nav
        aria-label="Document path"
        className="flex min-w-0 flex-1 items-center gap-0.5 overflow-hidden"
      >
        <ol className="flex min-w-0 items-center gap-0.5">
          {segments.map((seg) => (
            <li
              key={seg.absolutePath}
              className="flex shrink-0 items-center gap-0.5 last:min-w-0 last:shrink"
            >
              <button
                type="button"
                title={`Reveal ${seg.absolutePath} in folder`}
                onClick={() => void window.api.showInFolder(seg.absolutePath)}
                className="breadcrumb-file truncate rounded px-0.5 hover:bg-muted hover:text-foreground"
              >
                {seg.name}
              </button>
              {/* Filename lives outside this <ol>; the trailing chevron after the last segment is the separator before it. */}
              <ChevronRight className="size-2.5 shrink-0 text-muted-foreground/40" aria-hidden />
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
        aria-label={wideMode ? 'Exit wide mode' : 'Wide mode'}
        title={wideMode ? 'Constrained width' : 'Full width'}
        aria-pressed={wideMode}
        onClick={toggleWideMode}
        className="text-muted-foreground/70 hover:text-foreground"
      >
        {wideMode ? <FoldHorizontal /> : <ArrowLeftRight />}
      </Button>
    </div>
  )
}

interface Segment {
  name: string
  absolutePath: string
}

// Build a list of parent segments paired with their absolute path so each
// is independently revealable in the system file browser.
function parentSegmentsWithPaths(filePath: string, rootPath: string | null): Segment[] {
  const sep = detectSep(filePath)
  const parts = filePath.split(/[/\\]/).filter(Boolean)
  // Drop the filename
  const dirs = parts.slice(0, -1)

  // Slice anchored at an open folder when possible, else show the last 3 dirs.
  let startIndex = Math.max(0, dirs.length - 3)
  if (rootPath) {
    const rootParts = rootPath.split(/[/\\]/).filter(Boolean)
    const rootName = rootParts.at(-1)
    if (rootName) {
      const idx = dirs.lastIndexOf(rootName)
      if (idx >= 0) startIndex = idx
    }
  }

  const leadingSep = filePath.startsWith('/') ? '/' : filePath.startsWith('\\') ? '\\' : ''
  const segments: Segment[] = []
  for (let i = startIndex; i < dirs.length; i++) {
    const absolutePath = leadingSep + dirs.slice(0, i + 1).join(sep)
    segments.push({ name: dirs[i], absolutePath })
  }
  return segments
}

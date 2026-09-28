import { useOpenMarkdownFile } from '../hooks/useOpenMarkdownFile'
import { useOpenFileDialog } from '../hooks/useOpenFileDialog'
import { useOpenFolderDialog } from '../hooks/useOpenFolderDialog'
import { useRecents } from '../hooks/useRecents'
import { basename, parentDir } from '../lib/path-utils'
import { cn, formatShortcut } from '../lib/utils'
import { Logo } from './Logo'
import { File, FileText, FolderOpen, FlaskConical } from 'lucide-react'
import { openDevWorkspace } from '../dev/open-dev-workspace'

const isDev = import.meta.env.DEV
const RECENT_LIMIT = 5

function WelcomeButton({
  primary,
  icon: Icon,
  label,
  shortcut,
  onClick,
}: {
  primary?: boolean
  icon: typeof File
  label: string
  shortcut?: string
  onClick: () => void
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      aria-keyshortcuts={shortcut}
      className={cn(
        'welcome-btn inline-flex h-8 items-center gap-[7px] rounded-[7px] border px-3 text-[13px] font-medium outline-none',
        'focus-visible:ring-2 focus-visible:ring-ring/50 focus-visible:ring-offset-1 focus-visible:ring-offset-background',
        primary
          ? 'border-transparent bg-foreground text-background hover:bg-foreground/90'
          : 'border-border bg-surface-raised text-foreground shadow-(--shadow-raised) hover:bg-muted',
      )}
    >
      <Icon className="size-3.5 shrink-0" aria-hidden />
      {label}
      {shortcut && (
        <kbd className="ml-0.5 font-sans text-[11px] font-medium opacity-55">{shortcut}</kbd>
      )}
    </button>
  )
}

export function WelcomeView() {
  const openMarkdownFile = useOpenMarkdownFile()
  const openFileDialog = useOpenFileDialog()
  const openFolderDialog = useOpenFolderDialog()
  const { data: recents = [] } = useRecents()
  const shownRecents = recents.slice(0, RECENT_LIMIT)

  return (
    <div className="flex flex-1 items-center justify-center overflow-y-auto px-8 py-10">
      <div className="w-full max-w-[400px]">
        <Logo className="size-11 rounded-[11px] ring-[0.5px] ring-border" />
        <h2 className="mt-4 text-[22px]/7 font-semibold tracking-[-0.015em] text-foreground">
          Mdow
        </h2>
        <p className="mt-1 text-sm/[21px] text-muted-foreground">
          Open a Markdown file or folder to start reading.
        </p>
        <div className="mt-5 flex flex-wrap gap-2">
          <WelcomeButton
            primary
            icon={File}
            label="Open File"
            shortcut={formatShortcut('O')}
            onClick={() => void openFileDialog()}
          />
          <WelcomeButton
            icon={FolderOpen}
            label="Open Folder"
            shortcut={formatShortcut('O', { shift: true })}
            onClick={() => void openFolderDialog()}
          />
          {isDev && (
            <WelcomeButton icon={FlaskConical} label="Dev samples" onClick={openDevWorkspace} />
          )}
        </div>

        {shownRecents.length > 0 && (
          <section aria-labelledby="welcome-recent" className="mt-7">
            <div className="mb-1.5 flex items-center px-2.5 text-[11px] font-medium tracking-[0.02em] text-muted-foreground">
              <h3 id="welcome-recent" className="flex-1">
                Recent
              </h3>
              <span>{formatShortcut('K')} to search all</span>
            </div>
            <ul className="border-t border-border-subtle pt-1">
              {shownRecents.map((path) => {
                const dir = parentDir(path, 1)
                return (
                  <li key={path}>
                    <button
                      type="button"
                      title={path}
                      onClick={() => void openMarkdownFile(path)}
                      className="welcome-recent flex h-[34px] w-full items-center gap-[9px] rounded-md px-2.5 text-left text-[13px] text-foreground outline-none hover:bg-sidebar-accent/70 focus-visible:bg-sidebar-accent focus-visible:ring-2 focus-visible:ring-ring/40"
                    >
                      <FileText className="size-3.5 shrink-0 text-muted-foreground" aria-hidden />
                      <span className="min-w-0 flex-1 truncate">{basename(path)}</span>
                      {dir && (
                        <span className="max-w-[40%] shrink-0 truncate text-[11.5px] text-muted-foreground">
                          {dir}
                        </span>
                      )}
                    </button>
                  </li>
                )
              })}
            </ul>
          </section>
        )}

        <p className="mt-[18px] px-2.5 text-xs text-muted-foreground">
          Or drop files and folders anywhere in this window.
        </p>
      </div>
    </div>
  )
}

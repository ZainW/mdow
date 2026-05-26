import { useAppStore, type FileError, type ErrorType } from '../store/app-store'
import { Button } from './ui/button'
import { truncatePathMiddle } from '../lib/path-utils'
import { AlertCircle, FileX, FolderOpen, ShieldAlert, Trash2 } from 'lucide-react'
import type { LucideIcon } from 'lucide-react'
import { iconStroke } from '../lib/icons'

const errorMessages: Record<FileError['type'], { title: string; body: string; icon: LucideIcon }> =
  {
    'not-found': {
      title: 'File not found',
      body: 'This file seems to have wandered off. It may have been moved or renamed.',
      icon: FileX,
    },
    deleted: {
      title: 'File moved or deleted',
      body: 'This file was here a moment ago. Someone (or something) must have moved it.',
      icon: Trash2,
    },
    'permission-denied': {
      title: 'Access denied',
      body: "You don't have permission to read this file. Check the file permissions and try again.",
      icon: ShieldAlert,
    },
    'read-error': {
      title: "Couldn't read file",
      body: 'Something went wrong trying to read this file. It might be corrupted or locked by another process.',
      icon: AlertCircle,
    },
  }

interface ErrorViewProps {
  error: FileError
  tabId: string
}

function getReadErrorType(error: unknown): ErrorType {
  if (error instanceof Error) {
    if (error.message === 'not-found') return 'not-found'
    if (error.message === 'permission-denied') return 'permission-denied'
  }
  return 'read-error'
}

export function ErrorView({ error, tabId }: ErrorViewProps) {
  const clearTabError = useAppStore((s) => s.clearTabError)
  const closeTab = useAppStore((s) => s.closeTab)
  const updateTabContent = useAppStore((s) => s.updateTabContent)
  const setTabError = useAppStore((s) => s.setTabError)
  const msg = errorMessages[error.type]
  const Icon = msg.icon
  const displayPath = truncatePathMiddle(error.path, 48)

  const handleRetry = async () => {
    try {
      const content = await window.api.readFile(error.path)
      updateTabContent(error.path, content)
      clearTabError(tabId)
    } catch (err) {
      setTabError(error.path, { type: getReadErrorType(err), path: error.path })
    }
  }

  const handleShowInFolder = () => {
    void window.api.showInFolder(error.path)
  }

  return (
    <div className="flex flex-1 items-center justify-center" role="alert">
      <div className="flex max-w-sm flex-col items-center gap-4 text-center">
        <div className="flex size-12 items-center justify-center rounded-full bg-muted">
          <Icon className="size-6 text-muted-foreground" strokeWidth={iconStroke.default} />
        </div>
        <div className="space-y-1.5">
          <h2 className="text-base font-medium">{msg.title}</h2>
          <p className="text-sm leading-relaxed text-muted-foreground">{msg.body}</p>
        </div>
        <p
          className="max-w-full truncate font-mono text-xs text-muted-foreground/60"
          title={error.path}
        >
          {displayPath}
        </p>
        <div className="flex gap-2">
          <Button variant="outline" size="sm" onClick={() => void handleRetry()}>
            Try again
          </Button>
          {error.type !== 'permission-denied' && (
            <Button variant="ghost" size="sm" onClick={handleShowInFolder}>
              <FolderOpen className="mr-1.5 size-3.5" strokeWidth={iconStroke.default} />
              Show in folder
            </Button>
          )}
          <Button
            variant="ghost"
            size="sm"
            onClick={() => {
              void window.api.unwatchFile(error.path)
              closeTab(tabId)
            }}
          >
            Close tab
          </Button>
        </div>
      </div>
    </div>
  )
}

import { useEffect, useMemo, useRef, useState } from 'react'
import { Expand, MessageSquare, Minimize2, Send, Square, X } from 'lucide-react'
import { useAppStore, selectActiveTab } from '../../store/app-store'
import { Button } from '../ui/button'
import { Textarea } from '../ui/textarea'
import { Badge } from '../ui/badge'
import { Dialog, DialogContent, DialogHeader, DialogTitle } from '../ui/dialog'
import { cn } from '@renderer/lib/utils'
import { basename } from '../../lib/path-utils'
import { fuzzySearch } from '../../lib/fuzzy-search'
import type { CompanionProviderStatus, TreeNode } from '../../../../shared/types'

function flattenMarkdownPaths(nodes: TreeNode[]): { path: string; name: string }[] {
  const out: { path: string; name: string }[] = []
  const walk = (list: TreeNode[]) => {
    for (const node of list) {
      if (node.isDirectory && node.children) walk(node.children)
      else if (!node.isDirectory) out.push({ path: node.path, name: node.name })
    }
  }
  walk(nodes)
  return out
}

function CompanionSetup({ providers }: { providers: CompanionProviderStatus[] }) {
  const customCommand = useAppStore((s) => s.companionCustomCommand)
  const setCustomCommand = useAppStore((s) => s.setCompanionCustomCommand)
  const preferred = useAppStore((s) => s.companionPreferredProvider)
  const setPreferred = useAppStore((s) => s.setCompanionPreferredProvider)
  const [draft, setDraft] = useState(customCommand)

  return (
    <div className="flex flex-1 flex-col gap-3 overflow-y-auto p-3 text-sm">
      <p className="text-muted-foreground">
        Connect a local ACP agent already on this computer. Mdow will not install packages for you.
      </p>
      <ul className="flex flex-col gap-2">
        {providers.map((p) => (
          <li
            key={p.id}
            className={cn(
              'rounded-md border border-border-subtle p-2',
              preferred === p.id && 'border-primary/40 bg-muted/40',
            )}
          >
            <div className="flex items-center justify-between gap-2">
              <div>
                <p className="font-medium text-foreground">{p.label}</p>
                <p className="text-xs text-muted-foreground">{p.commandDisplay}</p>
              </div>
              <Badge variant={p.availability === 'available' ? 'default' : 'secondary'}>
                {p.availability}
              </Badge>
            </div>
            {p.detail && <p className="mt-1 text-xs text-muted-foreground">{p.detail}</p>}
            {p.availability === 'available' && (
              <Button
                size="sm"
                variant="outline"
                className="mt-2"
                onClick={() => setPreferred(p.id)}
              >
                Use {p.label}
              </Button>
            )}
          </li>
        ))}
      </ul>
      <div className="flex flex-col gap-1.5">
        <label className="text-xs font-medium text-muted-foreground" htmlFor="companion-custom-cmd">
          Custom command
        </label>
        <Textarea
          id="companion-custom-cmd"
          value={draft}
          rows={2}
          placeholder="path/to/agent --acp"
          onChange={(e) => setDraft(e.target.value)}
          className="min-h-0 resize-none text-xs"
        />
        <p className="text-xs text-muted-foreground">
          Custom commands run as local subprocesses from the main process.
        </p>
        <Button
          size="sm"
          variant="secondary"
          onClick={() => {
            setCustomCommand(draft.trim())
            setPreferred('custom')
          }}
        >
          Save custom command
        </Button>
      </div>
      <Button
        size="sm"
        variant="ghost"
        onClick={() => {
          void window.api.detectCompanionProviders().then((list) => {
            useAppStore.getState().setCompanionProviders(list)
          })
        }}
      >
        Retry detection
      </Button>
    </div>
  )
}

function CompanionMessages() {
  const messages = useAppStore((s) => s.companionMessages)
  const bottomRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ block: 'end' })
  }, [messages])

  if (messages.length === 0) {
    return (
      <div className="flex flex-1 items-center justify-center px-4 text-center text-sm text-muted-foreground">
        Ask about the open docs. Use @ to tag a file.
      </div>
    )
  }

  return (
    <div className="flex flex-1 flex-col gap-3 overflow-y-auto p-3">
      {messages.map((m) => (
        <div
          key={m.id}
          className={cn(
            'rounded-md px-2.5 py-2 text-sm leading-5',
            m.role === 'user' ? 'bg-muted/60 text-foreground' : 'bg-transparent text-foreground',
          )}
        >
          <p className="whitespace-pre-wrap">{m.content}</p>
          {m.citations && m.citations.length > 0 && (
            <div className="mt-2 flex flex-wrap gap-1">
              {m.citations.map((c) => (
                <button
                  key={`${m.id}-${c.sourceId}`}
                  type="button"
                  className="rounded-sm bg-muted px-1.5 py-0.5 text-[11px] text-muted-foreground hover:text-foreground"
                  onClick={() => {
                    window.dispatchEvent(
                      new CustomEvent('mdow:open-document-link', {
                        detail: { path: c.path },
                      }),
                    )
                  }}
                >
                  {c.label}
                </button>
              ))}
            </div>
          )}
        </div>
      ))}
      <div ref={bottomRef} />
    </div>
  )
}

function CompanionComposer() {
  const streaming = useAppStore((s) => s.companionStreaming)
  const tags = useAppStore((s) => s.companionTags)
  const addTag = useAppStore((s) => s.addCompanionTag)
  const removeTag = useAppStore((s) => s.removeCompanionTag)
  const appendMessage = useAppStore((s) => s.appendCompanionMessage)
  const contextSummary = useAppStore((s) => s.companionContextSummary)
  const warnings = useAppStore((s) => s.companionWarnings)
  const error = useAppStore((s) => s.companionError)
  const folderTree = useAppStore((s) => s.folderTree)
  const openFolderPath = useAppStore((s) => s.openFolderPath)
  const activeTab = useAppStore(selectActiveTab)
  const preferred = useAppStore((s) => s.companionPreferredProvider)
  const [text, setText] = useState('')
  const [mentionQuery, setMentionQuery] = useState<string | null>(null)

  const candidates = useMemo(() => flattenMarkdownPaths(folderTree), [folderTree])
  const mentionResults = useMemo(() => {
    if (mentionQuery === null) return []
    return fuzzySearch(mentionQuery, candidates).slice(0, 8)
  }, [mentionQuery, candidates])

  const send = async () => {
    const trimmed = text.trim()
    if (!trimmed || streaming) return
    const id = crypto.randomUUID()
    appendMessage({ id, role: 'user', content: trimmed, status: 'complete' })
    setText('')
    setMentionQuery(null)
    try {
      await window.api.sendCompanionMessage({
        text: trimmed,
        activePath: activeTab?.path ?? null,
        openFolderPath,
        tags,
        providerId: preferred ?? undefined,
      })
    } catch (err) {
      useAppStore.getState().applyCompanionUpdate({
        kind: 'error',
        message: err instanceof Error ? err.message : 'Failed to send',
      })
    }
  }

  return (
    <div className="shrink-0 border-t border-border-subtle p-2">
      {(contextSummary || warnings.length > 0 || error) && (
        <div className="mb-2 space-y-1 px-1 text-[11px] text-muted-foreground">
          {contextSummary && <p>{contextSummary}</p>}
          {warnings.map((w) => (
            <p key={w}>{w}</p>
          ))}
          {error && <p className="text-destructive">{error}</p>}
        </div>
      )}
      {tags.length > 0 && (
        <div className="mb-2 flex flex-wrap gap-1 px-1">
          {tags.map((t) => (
            <Badge key={t.sourceId} variant="secondary" className="gap-1 text-[11px]">
              @{basename(t.path)}
              <button
                type="button"
                aria-label={`Remove ${basename(t.path)}`}
                onClick={() => removeTag(t.sourceId)}
              >
                <X className="size-3" />
              </button>
            </Badge>
          ))}
        </div>
      )}
      {mentionQuery !== null && mentionResults.length > 0 && (
        <ul className="mb-2 max-h-36 overflow-y-auto rounded-md border border-border-subtle bg-popover p-1 text-xs">
          {mentionResults.map((r) => (
            <li key={r.path}>
              <button
                type="button"
                className="flex w-full rounded-sm px-2 py-1 text-left hover:bg-muted"
                onClick={() => {
                  addTag({
                    kind: 'file',
                    path: r.path,
                    sourceId: `tag:${r.path}`,
                  })
                  setText((prev) => prev.replace(/@[\w./\\-]*$/, ''))
                  setMentionQuery(null)
                }}
              >
                {r.name}
              </button>
            </li>
          ))}
        </ul>
      )}
      <div className="flex items-end gap-1.5">
        <Textarea
          value={text}
          rows={2}
          placeholder="Ask about these docs…"
          className="min-h-0 flex-1 resize-none text-sm"
          onChange={(e) => {
            const next = e.target.value
            setText(next)
            const at = next.match(/@([\w./\\-]*)$/)
            setMentionQuery(at ? at[1] : null)
          }}
          onKeyDown={(e) => {
            if (e.key === 'Enter' && !e.shiftKey) {
              e.preventDefault()
              void send()
            }
          }}
        />
        <Button
          size="icon-sm"
          variant={streaming ? 'secondary' : 'default'}
          aria-label={streaming ? 'Cancel' : 'Send'}
          onClick={() => {
            if (streaming) void window.api.cancelCompanion()
            else void send()
          }}
        >
          {streaming ? <Square /> : <Send />}
        </Button>
      </div>
    </div>
  )
}

function CompanionBody({
  onExpand,
  onCollapse,
  onClose,
}: {
  onExpand?: () => void
  onCollapse?: () => void
  onClose?: () => void
}) {
  const providers = useAppStore((s) => s.companionProviders)
  const preferred = useAppStore((s) => s.companionPreferredProvider)

  const hasProvider =
    preferred === 'custom' ||
    providers.some((p) => p.id === preferred && p.availability === 'available') ||
    providers.some((p) => p.availability === 'available')

  return (
    <div className="flex h-full min-h-0 flex-col">
      <header className="flex h-(--tabbar-height) shrink-0 items-center gap-1 border-b border-border-subtle px-2">
        <MessageSquare className="size-3.5 text-muted-foreground" aria-hidden />
        <h2 className="min-w-0 flex-1 truncate text-sm font-medium">Companion</h2>
        {onExpand && (
          <Button size="icon-xs" variant="ghost" aria-label="Expand companion" onClick={onExpand}>
            <Expand />
          </Button>
        )}
        {onCollapse && (
          <Button
            size="icon-xs"
            variant="ghost"
            aria-label="Collapse companion"
            onClick={onCollapse}
          >
            <Minimize2 />
          </Button>
        )}
        {onClose && (
          <Button size="icon-xs" variant="ghost" aria-label="Close companion" onClick={onClose}>
            <X />
          </Button>
        )}
      </header>
      {hasProvider ? (
        <>
          <CompanionMessages />
          <CompanionComposer />
        </>
      ) : (
        <CompanionSetup providers={providers} />
      )}
    </div>
  )
}

export function useCompanionBootstrap() {
  useEffect(() => {
    return window.api.onCompanionUpdate((update) => {
      useAppStore.getState().applyCompanionUpdate(update)
    })
  }, [])
}

function refreshCompanionMeta() {
  void window.api.detectCompanionProviders().then((list) => {
    useAppStore.getState().setCompanionProviders(list)
  })
  void window.api.getCompanionSettings().then((settings) => {
    useAppStore.setState({
      companionPreferredProvider: settings.preferredProvider,
      companionCustomCommand: settings.customCommand,
    })
  })
}

export function CompanionPanel() {
  const open = useAppStore((s) => s.companionOpen)
  const setOpen = useAppStore((s) => s.setCompanionOpen)
  const setFullscreen = useAppStore((s) => s.setCompanionFullscreen)

  useEffect(() => {
    if (open) refreshCompanionMeta()
  }, [open])

  return (
    <aside
      aria-label="AI companion"
      className="shrink-0 overflow-hidden border-l border-border-subtle bg-background"
      style={{ width: open ? 'var(--companion-drawer-width, 20rem)' : 0 }}
      aria-hidden={!open}
      inert={!open ? true : undefined}
    >
      <div
        className="flex h-full flex-col"
        style={{ width: 'var(--companion-drawer-width, 20rem)' }}
      >
        <CompanionBody onExpand={() => setFullscreen(true)} onClose={() => setOpen(false)} />
      </div>
    </aside>
  )
}

export function CompanionFullscreen() {
  const open = useAppStore((s) => s.companionFullscreen)
  const setFullscreen = useAppStore((s) => s.setCompanionFullscreen)

  useEffect(() => {
    if (open) refreshCompanionMeta()
  }, [open])

  return (
    <Dialog open={open} onOpenChange={setFullscreen}>
      <DialogContent className="flex h-[min(90vh,52rem)] w-[min(96vw,48rem)] max-w-none flex-col gap-0 overflow-hidden p-0">
        <DialogHeader className="sr-only">
          <DialogTitle>Companion</DialogTitle>
        </DialogHeader>
        <CompanionBody onCollapse={() => setFullscreen(false)} />
      </DialogContent>
    </Dialog>
  )
}

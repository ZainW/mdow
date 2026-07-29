/* oxlint-disable jsx-a11y/no-noninteractive-element-to-interactive-role, jsx-a11y/prefer-tag-over-role -- The custom mention popup follows the ARIA combobox/listbox pattern. */
import { useEffect, useMemo, useRef, useState } from 'react'
import { CircleEllipsis, Expand, MessageSquare, Minimize2, Send, Square, X } from 'lucide-react'
import { useAppStore, selectActiveTab } from '../../store/app-store'
import { Button } from '../ui/button'
import { Textarea } from '../ui/textarea'
import { Badge } from '../ui/badge'
import { Dialog, DialogContent, DialogHeader, DialogTitle } from '../ui/dialog'
import { cn, isMac } from '@renderer/lib/utils'
import { basename } from '../../lib/path-utils'
import { fuzzySearch } from '../../lib/fuzzy-search'
import type { CompanionMessage, CompanionProviderStatus, TreeNode } from '../../../../shared/types'
import {
  Conversation,
  ConversationContent,
  ConversationEmptyState,
} from '../ai-elements/conversation'
import { Message, MessageContent, MessageResponse } from '../ai-elements/message'
import { Reasoning, ReasoningContent, ReasoningTrigger } from '../ai-elements/reasoning'
import { Tool, ToolContent, ToolHeader, ToolInput, ToolOutput } from '../ai-elements/tool'
import { Source, SourcesCollapsible } from '../ai-elements/sources'

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

function openCitation(path: string) {
  window.dispatchEvent(
    new CustomEvent('mdow:open-document-link', {
      detail: { path },
    }),
  )
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

function AssistantParts({ message }: { message: CompanionMessage }) {
  const streaming = message.status === 'streaming'
  const lastTextIndex = message.parts.findLastIndex((p) => p.kind === 'text')
  let thinkingSequence = 0
  let statusSequence = 0
  let textSequence = 0
  return (
    <>
      {streaming && message.parts.length === 0 && (
        <output className="flex items-center gap-2 py-1.5 text-xs text-muted-foreground">
          <CircleEllipsis className="size-3.5 shrink-0 animate-pulse" aria-hidden />
          <span>Connecting to local agent…</span>
        </output>
      )}
      {message.parts.map((part, index) => {
        if (part.kind === 'thinking') {
          const key = `${message.id}-thinking-${thinkingSequence}`
          thinkingSequence += 1
          return (
            <Reasoning key={key} isStreaming={!part.done && streaming} defaultOpen={false}>
              <ReasoningTrigger />
              <ReasoningContent>{part.text}</ReasoningContent>
            </Reasoning>
          )
        }
        if (part.kind === 'tool') {
          return (
            <Tool key={part.toolCallId} defaultOpen={false}>
              <ToolHeader name={part.name} state={part.state} />
              <ToolContent>
                <ToolInput input={part.input} />
                <ToolOutput output={part.output} error={part.error} />
              </ToolContent>
            </Tool>
          )
        }
        if (part.kind === 'status') {
          const key = `${message.id}-status-${statusSequence}`
          statusSequence += 1
          return (
            <p key={key} className="text-xs text-muted-foreground">
              {part.message}
            </p>
          )
        }
        if (part.kind === 'text') {
          const key = `${message.id}-text-${textSequence}`
          textSequence += 1
          return (
            <MessageResponse key={key} streaming={streaming && index === lastTextIndex}>
              {part.text}
            </MessageResponse>
          )
        }
        return null
      })}
      {message.status === 'cancelled' && (
        <p className="text-xs text-muted-foreground">Response cancelled</p>
      )}
      {message.citations && message.citations.length > 0 && (
        <SourcesCollapsible count={message.citations.length} defaultOpen>
          {message.citations.map((c) => (
            <Source
              key={`${message.id}-${c.sourceId}`}
              title={c.label}
              onClick={() => openCitation(c.path)}
            />
          ))}
        </SourcesCollapsible>
      )}
    </>
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
      <ConversationEmptyState
        icon={<MessageSquare className="size-5 text-muted-foreground/60" />}
        title="Ask about these docs"
        description="Use @ to tag a file. Answers stream with thinking, tools, and source chips."
      />
    )
  }

  return (
    <Conversation>
      <ConversationContent>
        {messages.map((m) => (
          <Message key={m.id} from={m.role === 'system' ? 'assistant' : m.role}>
            <MessageContent>
              {m.role === 'user' ? (
                <p className="whitespace-pre-wrap">{m.content}</p>
              ) : (
                <AssistantParts message={m} />
              )}
            </MessageContent>
          </Message>
        ))}
        <div ref={bottomRef} />
      </ConversationContent>
    </Conversation>
  )
}

function CompanionComposer() {
  const streaming = useAppStore((s) => s.companionStreaming)
  const tags = useAppStore((s) => s.companionTags)
  const addTag = useAppStore((s) => s.addCompanionTag)
  const removeTag = useAppStore((s) => s.removeCompanionTag)
  const appendMessage = useAppStore((s) => s.appendCompanionMessage)
  const beginRequest = useAppStore((s) => s.beginCompanionRequest)
  const cancelRequest = useAppStore((s) => s.cancelCompanionRequest)
  const contextSummary = useAppStore((s) => s.companionContextSummary)
  const warnings = useAppStore((s) => s.companionWarnings)
  const error = useAppStore((s) => s.companionError)
  const folderTree = useAppStore((s) => s.folderTree)
  const openFolderPath = useAppStore((s) => s.openFolderPath)
  const activeTab = useAppStore(selectActiveTab)
  const preferred = useAppStore((s) => s.companionPreferredProvider)
  const [text, setText] = useState('')
  const [mentionQuery, setMentionQuery] = useState<string | null>(null)
  const [activeMentionIndex, setActiveMentionIndex] = useState(-1)

  const candidates = useMemo(() => flattenMarkdownPaths(folderTree), [folderTree])
  const mentionResults = useMemo(() => {
    if (mentionQuery === null) return []
    return fuzzySearch(mentionQuery, candidates).slice(0, 8)
  }, [mentionQuery, candidates])

  const selectMention = (result: { path: string; name: string }) => {
    addTag({
      kind: 'file',
      path: result.path,
      sourceId: `tag:${result.path}`,
    })
    setText((previous) => previous.replace(/@[\w./\\-]*$/, ''))
    setMentionQuery(null)
    setActiveMentionIndex(-1)
  }

  const send = async () => {
    const trimmed = text.trim()
    if (!trimmed || useAppStore.getState().companionStreaming) return
    const id = crypto.randomUUID()
    appendMessage({
      id,
      role: 'user',
      content: trimmed,
      parts: [{ kind: 'text', text: trimmed }],
      status: 'complete',
    })
    beginRequest()
    setText('')
    setMentionQuery(null)
    setActiveMentionIndex(-1)
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
    <div className="relative shrink-0 border-t border-border-subtle p-2">
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
        <ul
          id="companion-mention-listbox"
          role="listbox"
          aria-label="Document suggestions"
          className="absolute right-2 bottom-full left-2 z-10 mb-2 max-h-40 overflow-y-auto rounded-md bg-popover p-1 text-xs shadow-lg ring-1 ring-foreground/10 dark:shadow-none"
        >
          {mentionResults.map((result, index) => (
            <li key={result.path} role="presentation">
              <button
                type="button"
                id={`companion-mention-${index}`}
                role="option"
                aria-selected={index === activeMentionIndex}
                className={cn(
                  'flex w-full rounded-sm px-2 py-1.5 text-left',
                  index === activeMentionIndex ? 'bg-muted text-foreground' : 'hover:bg-muted/60',
                )}
                onMouseDown={(event) => event.preventDefault()}
                onClick={() => selectMention(result)}
              >
                {result.name}
              </button>
            </li>
          ))}
        </ul>
      )}
      <div className="flex items-end gap-1.5">
        <Textarea
          name="companion-prompt"
          aria-label="Ask about these docs"
          aria-controls={mentionResults.length > 0 ? 'companion-mention-listbox' : undefined}
          aria-expanded={mentionResults.length > 0}
          aria-activedescendant={
            activeMentionIndex >= 0 ? `companion-mention-${activeMentionIndex}` : undefined
          }
          aria-autocomplete="list"
          value={text}
          rows={2}
          placeholder="Ask about these docs…"
          className="min-h-0 flex-1 resize-none text-sm"
          onChange={(e) => {
            const next = e.target.value
            setText(next)
            const at = next.match(/@([\w./\\-]*)$/)
            setMentionQuery(at ? at[1] : null)
            setActiveMentionIndex(-1)
          }}
          onKeyDown={(e) => {
            if (mentionResults.length > 0) {
              if (e.key === 'ArrowDown') {
                e.preventDefault()
                setActiveMentionIndex((index) => (index + 1) % mentionResults.length)
                return
              }
              if (e.key === 'ArrowUp') {
                e.preventDefault()
                setActiveMentionIndex((index) =>
                  index <= 0 ? mentionResults.length - 1 : index - 1,
                )
                return
              }
              if (e.key === 'Enter' && activeMentionIndex >= 0) {
                e.preventDefault()
                selectMention(mentionResults[activeMentionIndex])
                return
              }
              if (e.key === 'Escape') {
                e.preventDefault()
                setMentionQuery(null)
                setActiveMentionIndex(-1)
                return
              }
            }
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
            if (streaming) {
              void window.api
                .cancelCompanion()
                .then(() => cancelRequest())
                .catch((err: unknown) => {
                  useAppStore.getState().applyCompanionUpdate({
                    kind: 'error',
                    message: err instanceof Error ? err.message : 'Failed to cancel',
                  })
                })
            } else {
              void send()
            }
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
  const customCommand = useAppStore((s) => s.companionCustomCommand)

  const hasProvider =
    (preferred === 'custom' && customCommand.trim().length > 0) ||
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
  const fullscreen = useAppStore((s) => s.companionFullscreen)
  const setOpen = useAppStore((s) => s.setCompanionOpen)
  const setFullscreen = useAppStore((s) => s.setCompanionFullscreen)

  useEffect(() => {
    if (open) refreshCompanionMeta()
  }, [open])

  return (
    <aside
      aria-label="AI companion"
      className={cn(
        'overflow-hidden bg-background',
        'max-lg:fixed max-lg:right-0 max-lg:bottom-0 max-lg:z-40 max-lg:shadow-xl max-lg:ring-1 max-lg:ring-foreground/10 max-lg:dark:shadow-none',
        isMac ? 'max-lg:top-7' : 'max-lg:top-0',
        'lg:relative lg:z-auto lg:shrink-0 lg:border-l lg:border-border-subtle',
        open
          ? 'max-lg:w-[min(24rem,calc(100vw-1rem))] lg:w-(--companion-drawer-width)'
          : 'w-0 max-lg:pointer-events-none',
      )}
      aria-hidden={!open || fullscreen}
      inert={!open || fullscreen ? true : undefined}
    >
      <div className="flex h-full w-full flex-col lg:w-(--companion-drawer-width)">
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
      <DialogContent className="flex h-[min(90vh,52rem)] w-[min(96vw,48rem)] max-w-none flex-col gap-0 overflow-hidden p-0 sm:max-w-none">
        <DialogHeader className="sr-only">
          <DialogTitle>Companion</DialogTitle>
        </DialogHeader>
        <CompanionBody onCollapse={() => setFullscreen(false)} />
      </DialogContent>
    </Dialog>
  )
}

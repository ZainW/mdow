/* oxlint-disable jsx-a11y/no-noninteractive-element-to-interactive-role, jsx-a11y/prefer-tag-over-role -- The custom mention popup follows the ARIA combobox/listbox pattern. */
import { useEffect, useEffectEvent, useMemo, useRef, useState } from 'react'
import { ArrowUp, AtSign, FileText, Square, X } from 'lucide-react'
import { fuzzySearch } from '../../lib/fuzzy-search'
import { basename } from '../../lib/path-utils'
import { cn } from '../../lib/utils'
import { selectActiveTab, useAppStore } from '../../store/app-store'
import { Button } from '../ui/button'
import { CompanionModelPicker } from './CompanionModelPicker'
import { flattenDocuments } from './document-resolver'

const COMPANION_SUBMIT_EVENT = 'mdow:companion-submit'

/** Sends `text` as if the user typed it, e.g. from a suggestion or a retry. */
export function requestCompanionSend(text: string) {
  window.dispatchEvent(new CustomEvent(COMPANION_SUBMIT_EVENT, { detail: { text } }))
}

export function CompanionComposer() {
  const streaming = useAppStore((state) => state.companionStreaming)
  const tags = useAppStore((state) => state.companionTags)
  const addTag = useAppStore((state) => state.addCompanionTag)
  const removeTag = useAppStore((state) => state.removeCompanionTag)
  const appendMessage = useAppStore((state) => state.appendCompanionMessage)
  const beginRequest = useAppStore((state) => state.beginCompanionRequest)
  const cancelRequest = useAppStore((state) => state.cancelCompanionRequest)
  const folderTree = useAppStore((state) => state.folderTree)
  const openFolderPath = useAppStore((state) => state.openFolderPath)
  const activeTab = useAppStore(selectActiveTab)
  const modelState = useAppStore((state) => state.companionModelState)
  const selectModel = useAppStore((state) => state.selectCompanionModel)
  const [text, setText] = useState('')
  const [mentionQuery, setMentionQuery] = useState<string | null>(null)
  const [activeMentionIndex, setActiveMentionIndex] = useState(-1)
  const textareaRef = useRef<HTMLTextAreaElement>(null)

  const candidates = useMemo(() => flattenDocuments(folderTree), [folderTree])
  const mentionResults = useMemo(() => {
    if (mentionQuery === null) return []
    return fuzzySearch(mentionQuery, candidates)
      .filter((result) => result.path !== activeTab?.path)
      .slice(0, 8)
  }, [mentionQuery, candidates, activeTab?.path])

  const send = async (message: string) => {
    const trimmed = message.trim()
    if (!trimmed || useAppStore.getState().companionStreaming) return
    appendMessage({
      id: crypto.randomUUID(),
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
      })
    } catch (sendError) {
      useAppStore.getState().applyCompanionUpdate({
        kind: 'error',
        message: sendError instanceof Error ? sendError.message : 'Failed to send',
      })
    }
  }

  // Suggestions and retries go through the same path as typed messages.
  const onSubmitRequest = useEffectEvent((event: Event) => {
    const detail: unknown = event instanceof CustomEvent ? event.detail : null
    if (typeof detail === 'object' && detail !== null && 'text' in detail) {
      if (typeof detail.text === 'string') void send(detail.text)
    }
  })
  useEffect(() => {
    const handler = (event: Event) => onSubmitRequest(event)
    window.addEventListener(COMPANION_SUBMIT_EVENT, handler)
    return () => window.removeEventListener(COMPANION_SUBMIT_EVENT, handler)
  }, [])

  const selectMention = (result: { path: string; name: string }) => {
    addTag({ kind: 'file', path: result.path, sourceId: `tag:${result.path}` })
    setText((previous) => previous.replace(/@[^\s@]*$/, ''))
    setMentionQuery(null)
    setActiveMentionIndex(-1)
    textareaRef.current?.focus()
  }

  const startMention = () => {
    const textarea = textareaRef.current
    setText((previous) => `${previous}${previous && !previous.endsWith(' ') ? ' ' : ''}@`)
    setMentionQuery('')
    setActiveMentionIndex(0)
    queueMicrotask(() => textarea?.focus())
  }

  const stop = () => {
    void window.api
      .cancelCompanion()
      .then(() => cancelRequest())
      .catch((cancelError: unknown) => {
        useAppStore.getState().applyCompanionUpdate({
          kind: 'error',
          message: cancelError instanceof Error ? cancelError.message : 'Failed to stop',
        })
      })
  }

  const placeholder = activeTab
    ? `Ask about ${basename(activeTab.path)}, or ask for an edit…`
    : 'Ask about your documents…'
  const canSend = text.trim().length > 0

  return (
    <div className="relative px-3 pt-1 pb-3">
      {mentionQuery !== null && mentionResults.length > 0 && (
        <ul
          id="companion-mention-listbox"
          role="listbox"
          aria-label="Documents"
          className="absolute right-3 bottom-full left-3 z-10 mb-1 max-h-48 overflow-y-auto rounded-lg bg-popover p-1 text-xs shadow-lg ring-1 ring-foreground/10 dark:shadow-none"
        >
          {mentionResults.map((result, index) => (
            <li key={result.path} role="presentation">
              <button
                type="button"
                id={`companion-mention-${index}`}
                role="option"
                aria-selected={index === activeMentionIndex}
                className={cn(
                  'flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left',
                  index === activeMentionIndex ? 'bg-muted text-foreground' : 'hover:bg-muted/60',
                )}
                onMouseDown={(event) => event.preventDefault()}
                onClick={() => selectMention(result)}
              >
                <FileText className="size-3.5 shrink-0 text-muted-foreground" aria-hidden />
                <span className="truncate">{result.name}</span>
              </button>
            </li>
          ))}
        </ul>
      )}
      <div className="rounded-xl border border-border bg-background shadow-xs transition-[border-color,box-shadow] focus-within:border-ring focus-within:ring-3 focus-within:ring-ring/20 dark:shadow-none">
        {(activeTab || tags.length > 0) && (
          <div className="flex flex-wrap gap-1 px-2 pt-2">
            {activeTab && (
              <span
                className="inline-flex h-6 max-w-48 items-center gap-1 rounded-md bg-muted px-1.5 text-[11px] text-muted-foreground"
                title={`OpenCode sees that you are viewing ${activeTab.path}`}
              >
                <FileText className="size-3 shrink-0" aria-hidden />
                <span className="truncate">{basename(activeTab.path)}</span>
              </span>
            )}
            {tags.map((tag) => (
              <span
                key={tag.sourceId}
                className="inline-flex h-6 max-w-48 items-center gap-1 rounded-md bg-muted pr-0.5 pl-1.5 text-[11px] text-foreground"
              >
                <AtSign className="size-3 shrink-0 text-muted-foreground" aria-hidden />
                <span className="truncate">{basename(tag.path)}</span>
                <button
                  type="button"
                  className="flex size-5 items-center justify-center rounded-sm text-muted-foreground hover:bg-background hover:text-foreground"
                  aria-label={`Remove ${basename(tag.path)}`}
                  onClick={() => removeTag(tag.sourceId)}
                >
                  <X className="size-3" aria-hidden />
                </button>
              </span>
            ))}
          </div>
        )}
        <textarea
          ref={textareaRef}
          name="companion-prompt"
          aria-label="Message the companion"
          role="combobox"
          aria-controls={mentionResults.length > 0 ? 'companion-mention-listbox' : undefined}
          aria-expanded={mentionResults.length > 0}
          aria-activedescendant={
            activeMentionIndex >= 0 && mentionResults.length > 0
              ? `companion-mention-${activeMentionIndex}`
              : undefined
          }
          aria-autocomplete="list"
          value={text}
          rows={1}
          placeholder={placeholder}
          className="block max-h-48 min-h-11 w-full resize-none bg-transparent px-3 pt-2.5 pb-1 text-sm outline-none [field-sizing:content] placeholder:text-muted-foreground"
          onChange={(event) => {
            const next = event.target.value
            setText(next)
            const at = /(?:^|\s)@([^\s@]*)$/.exec(next)
            setMentionQuery(at ? at[1] : null)
            setActiveMentionIndex(at ? 0 : -1)
          }}
          onKeyDown={(event) => {
            // Enter confirms an IME candidate (CJK input); it must not send or pick a mention.
            if (event.nativeEvent.isComposing || event.keyCode === 229) return
            if (mentionResults.length > 0) {
              if (event.key === 'ArrowDown') {
                event.preventDefault()
                setActiveMentionIndex((index) => (index + 1) % mentionResults.length)
                return
              }
              if (event.key === 'ArrowUp') {
                event.preventDefault()
                setActiveMentionIndex((index) =>
                  index <= 0 ? mentionResults.length - 1 : index - 1,
                )
                return
              }
              if ((event.key === 'Enter' || event.key === 'Tab') && activeMentionIndex >= 0) {
                event.preventDefault()
                selectMention(mentionResults[activeMentionIndex])
                return
              }
              if (event.key === 'Escape') {
                event.preventDefault()
                setMentionQuery(null)
                setActiveMentionIndex(-1)
                return
              }
            }
            if (event.key === 'Enter' && !event.shiftKey) {
              event.preventDefault()
              void send(text)
            }
          }}
        />
        <div className="flex items-center gap-1 px-1.5 pb-1.5">
          <Button
            type="button"
            size="icon-xs"
            variant="ghost"
            className="text-muted-foreground"
            aria-label="Mention a document"
            disabled={candidates.length === 0}
            onClick={startMention}
          >
            <AtSign />
          </Button>
          <div className="min-w-0 flex-1">
            <CompanionModelPicker
              state={modelState}
              onValueChange={(value) => void selectModel(value)}
            />
          </div>
          {streaming ? (
            <Button
              type="button"
              size="icon-sm"
              variant="secondary"
              className="rounded-full"
              aria-label="Stop"
              onClick={stop}
            >
              <Square className="size-3 fill-current" />
            </Button>
          ) : (
            <Button
              type="button"
              size="icon-sm"
              className="rounded-full"
              aria-label="Send"
              disabled={!canSend}
              onClick={() => void send(text)}
            >
              <ArrowUp />
            </Button>
          )}
        </div>
      </div>
    </div>
  )
}

/* oxlint-disable jsx-a11y/no-noninteractive-element-to-interactive-role, jsx-a11y/prefer-tag-over-role -- The custom mention popup follows the ARIA combobox/listbox pattern. */
import { useMemo, useState } from 'react'
import { Send, Square, X } from 'lucide-react'
import type { CompanionProviderId, TreeNode } from '../../../../shared/types'
import { fuzzySearch } from '../../lib/fuzzy-search'
import { basename } from '../../lib/path-utils'
import { cn } from '../../lib/utils'
import { selectActiveTab, useAppStore } from '../../store/app-store'
import { Badge } from '../ui/badge'
import { Button } from '../ui/button'
import { Textarea } from '../ui/textarea'

function flattenMarkdownPaths(nodes: TreeNode[]): { path: string; name: string }[] {
  const output: { path: string; name: string }[] = []
  const walk = (list: TreeNode[]) => {
    for (const node of list) {
      if (node.isDirectory && node.children) walk(node.children)
      else if (!node.isDirectory) output.push({ path: node.path, name: node.name })
    }
  }
  walk(nodes)
  return output
}

export function CompanionComposer({ providerId }: { providerId: CompanionProviderId }) {
  const streaming = useAppStore((state) => state.companionStreaming)
  const tags = useAppStore((state) => state.companionTags)
  const addTag = useAppStore((state) => state.addCompanionTag)
  const removeTag = useAppStore((state) => state.removeCompanionTag)
  const appendMessage = useAppStore((state) => state.appendCompanionMessage)
  const beginRequest = useAppStore((state) => state.beginCompanionRequest)
  const cancelRequest = useAppStore((state) => state.cancelCompanionRequest)
  const error = useAppStore((state) => state.companionError)
  const folderTree = useAppStore((state) => state.folderTree)
  const openFolderPath = useAppStore((state) => state.openFolderPath)
  const activeTab = useAppStore(selectActiveTab)
  const [text, setText] = useState('')
  const [mentionQuery, setMentionQuery] = useState<string | null>(null)
  const [activeMentionIndex, setActiveMentionIndex] = useState(-1)

  const candidates = useMemo(() => flattenMarkdownPaths(folderTree), [folderTree])
  const mentionResults = useMemo(() => {
    if (mentionQuery === null) return []
    return fuzzySearch(mentionQuery, candidates).slice(0, 8)
  }, [mentionQuery, candidates])

  const selectMention = (result: { path: string; name: string }) => {
    addTag({ kind: 'file', path: result.path, sourceId: `tag:${result.path}` })
    setText((previous) => previous.replace(/@[\w./\\-]*$/, ''))
    setMentionQuery(null)
    setActiveMentionIndex(-1)
  }

  const send = async () => {
    const trimmed = text.trim()
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
        providerId,
      })
    } catch (sendError) {
      useAppStore.getState().applyCompanionUpdate({
        kind: 'error',
        message: sendError instanceof Error ? sendError.message : 'Failed to send',
      })
    }
  }

  return (
    <div className="relative p-2 pt-1.5">
      {error && (
        <p role="alert" className="mb-1.5 px-1 text-[11px] text-destructive">
          {error}
        </p>
      )}
      {tags.length > 0 && (
        <div className="mb-1.5 flex flex-wrap gap-1 px-1">
          {tags.map((tag) => (
            <Badge key={tag.sourceId} variant="secondary" className="gap-1 text-[11px]">
              @{basename(tag.path)}
              <button
                type="button"
                aria-label={`Remove ${basename(tag.path)}`}
                onClick={() => removeTag(tag.sourceId)}
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
          onChange={(event) => {
            const next = event.target.value
            setText(next)
            const at = next.match(/@([\w./\\-]*)$/)
            setMentionQuery(at ? at[1] : null)
            setActiveMentionIndex(-1)
          }}
          onKeyDown={(event) => {
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
              if (event.key === 'Enter' && activeMentionIndex >= 0) {
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
                .catch((cancelError: unknown) => {
                  useAppStore.getState().applyCompanionUpdate({
                    kind: 'error',
                    message:
                      cancelError instanceof Error ? cancelError.message : 'Failed to cancel',
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

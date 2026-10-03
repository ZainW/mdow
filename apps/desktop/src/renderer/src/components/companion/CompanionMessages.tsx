import { useEffect, useRef, useState, type ReactNode } from 'react'
import {
  Check,
  Copy,
  FileSearch,
  FolderSearch,
  Lightbulb,
  ListCollapse,
  RotateCcw,
  Scissors,
  SpellCheck,
  Sparkles,
} from 'lucide-react'
import type { CompanionMessage } from '../../../../shared/types'
import { basename } from '../../lib/path-utils'
import { selectActiveTab, useAppStore } from '../../store/app-store'
import {
  Conversation,
  ConversationContent,
  ConversationEmptyState,
  ConversationScrollButton,
} from '../ai-elements/conversation'
import type { DocumentResolver } from '../ai-elements/markdown'
import { Message, MessageContent, MessageResponse } from '../ai-elements/message'
import { Reasoning, ReasoningContent, ReasoningTrigger } from '../ai-elements/reasoning'
import { Shimmer } from '../ai-elements/shimmer'
import { Source, SourcesCollapsible } from '../ai-elements/sources'
import { Tool, ToolContent, ToolHeader, ToolInput, ToolOutput } from '../ai-elements/tool'
import { Button } from '../ui/button'
import { CompanionChangeCard } from './CompanionChangeCard'
import { requestCompanionSend } from './CompanionComposer'
import { useDocumentResolver } from './document-resolver'

interface Suggestion {
  icon: ReactNode
  label: string
  prompt: string
}

function suggestionsFor(documentName: string | null): Suggestion[] {
  const icon = 'size-3.5 shrink-0 text-muted-foreground'
  if (documentName) {
    return [
      {
        icon: <ListCollapse className={icon} aria-hidden />,
        label: 'Summarize this document',
        prompt: `Summarize ${documentName} in a few bullet points.`,
      },
      {
        icon: <Lightbulb className={icon} aria-hidden />,
        label: 'Explain the key ideas',
        prompt: `Explain the key ideas in ${documentName} as if I'm new to the topic.`,
      },
      {
        icon: <SpellCheck className={icon} aria-hidden />,
        label: 'Fix spelling and grammar',
        prompt: `Fix spelling and grammar mistakes in ${documentName}. Don't change the meaning or style.`,
      },
      {
        icon: <Scissors className={icon} aria-hidden />,
        label: 'Make it more concise',
        prompt: `Tighten the writing in ${documentName}: cut repetition and filler, keep every point.`,
      },
    ]
  }
  return [
    {
      icon: <FolderSearch className={icon} aria-hidden />,
      label: "What's in this folder?",
      prompt: 'Give me a quick tour of the documents in this folder.',
    },
    {
      icon: <FileSearch className={icon} aria-hidden />,
      label: 'Find open questions',
      prompt: 'Find open questions and TODOs across these documents.',
    },
  ]
}

function CopyButton({ message }: { message: CompanionMessage }) {
  const [copied, setCopied] = useState(false)
  if (!message.content.trim()) return null
  return (
    <Button
      type="button"
      size="icon-xs"
      variant="ghost"
      aria-label={copied ? 'Copied' : 'Copy response'}
      className="text-muted-foreground opacity-0 transition-opacity group-hover:opacity-100 focus-visible:opacity-100"
      onClick={() => {
        void navigator.clipboard.writeText(message.content).then(() => {
          setCopied(true)
          setTimeout(() => setCopied(false), 1500)
        })
      }}
    >
      {copied ? <Check className="text-emerald-600" /> : <Copy />}
    </Button>
  )
}

function AssistantParts({
  message,
  resolveDocument,
}: {
  message: CompanionMessage
  resolveDocument: DocumentResolver
}) {
  const streaming = message.status === 'streaming'
  const lastTextIndex = message.parts.findLastIndex((part) => part.kind === 'text')
  const lastPart = message.parts.at(-1)
  const waiting =
    streaming &&
    (!lastPart ||
      (lastPart.kind === 'tool' && lastPart.state === 'completed') ||
      (lastPart.kind === 'thinking' && lastPart.done) ||
      (lastPart.kind === 'change' && lastPart.status === 'applied'))
  let thinkingSequence = 0
  let statusSequence = 0
  let textSequence = 0

  return (
    <>
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
        if (part.kind === 'change') {
          return <CompanionChangeCard key={part.toolCallId} part={part} />
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
        const key = `${message.id}-text-${textSequence}`
        textSequence += 1
        return (
          <MessageResponse
            key={key}
            streaming={streaming && index === lastTextIndex && index === message.parts.length - 1}
            resolveDocument={resolveDocument}
          >
            {part.text}
          </MessageResponse>
        )
      })}
      {waiting && (
        <output className="py-0.5 text-xs">
          <Shimmer>{lastPart ? 'Working…' : 'Thinking…'}</Shimmer>
        </output>
      )}
      {message.status === 'cancelled' && <p className="text-xs text-muted-foreground">Stopped</p>}
      {message.citations && message.citations.length > 0 && (
        <SourcesCollapsible count={message.citations.length}>
          {message.citations.map((citation) => (
            <Source
              key={`${message.id}-${citation.sourceId}`}
              title={citation.label}
              onClick={() =>
                window.dispatchEvent(
                  new CustomEvent('mdow:open-document-link', { detail: { path: citation.path } }),
                )
              }
            />
          ))}
        </SourcesCollapsible>
      )}
    </>
  )
}

function TurnError({ onRetry }: { onRetry: () => void }) {
  const error = useAppStore((state) => state.companionError)
  return (
    <div
      role="alert"
      className="flex items-start gap-2 rounded-md bg-destructive/10 px-3 py-2 text-xs text-destructive"
    >
      <p className="min-w-0 flex-1 leading-5">{error ?? 'Something went wrong.'}</p>
      <Button size="xs" variant="ghost" className="-my-0.5 shrink-0" onClick={onRetry}>
        <RotateCcw aria-hidden />
        Retry
      </Button>
    </div>
  )
}

export function CompanionMessages() {
  const messages = useAppStore((state) => state.companionMessages)
  const activeTab = useAppStore(selectActiveTab)
  const openFolderPath = useAppStore((state) => state.openFolderPath)
  const resolveDocument = useDocumentResolver()
  const contentRef = useRef<HTMLDivElement>(null)
  const stickToBottomRef = useRef(true)

  useEffect(() => {
    const el = contentRef.current
    if (!el) return
    const onScroll = () => {
      const distance = el.scrollHeight - el.scrollTop - el.clientHeight
      stickToBottomRef.current = distance < 80
    }
    el.addEventListener('scroll', onScroll, { passive: true })
    return () => el.removeEventListener('scroll', onScroll)
  }, [])

  useEffect(() => {
    const el = contentRef.current
    if (!el || !stickToBottomRef.current || typeof el.scrollTo !== 'function') return
    el.scrollTo({ top: el.scrollHeight })
  }, [messages])

  if (messages.length === 0) {
    const documentName = activeTab ? basename(activeTab.path) : null
    const scope = documentName ?? (openFolderPath ? basename(openFolderPath) : null)
    return (
      <ConversationEmptyState
        icon={
          <span className="mb-1 flex size-9 items-center justify-center rounded-xl bg-muted text-foreground">
            <Sparkles className="size-4" aria-hidden />
          </span>
        }
        title={scope ? `Ask about ${scope}` : 'Ask about your documents'}
        description="Get answers, summaries and edits. Every change comes back as a diff for you to approve."
      >
        <div className="mt-4 flex w-full max-w-72 flex-col gap-1">
          {suggestionsFor(documentName).map((suggestion) => (
            <button
              key={suggestion.label}
              type="button"
              className="flex items-center gap-2.5 rounded-md px-3 py-2 text-left text-xs text-foreground transition-colors hover:bg-muted focus-visible:bg-muted"
              onClick={() => requestCompanionSend(suggestion.prompt)}
            >
              {suggestion.icon}
              {suggestion.label}
            </button>
          ))}
        </div>
      </ConversationEmptyState>
    )
  }

  const lastUser = messages.findLast((message) => message.role === 'user')
  const last = messages.at(-1)

  return (
    <Conversation>
      <ConversationContent ref={contentRef}>
        {messages.map((message) => (
          <Message key={message.id} from={message.role === 'system' ? 'assistant' : message.role}>
            <MessageContent>
              {message.role === 'user' ? (
                <p className="whitespace-pre-wrap">{message.content}</p>
              ) : (
                <AssistantParts message={message} resolveDocument={resolveDocument} />
              )}
            </MessageContent>
            {message.role === 'assistant' && message.status === 'complete' && (
              <div className="-mt-1 flex">
                <CopyButton message={message} />
              </div>
            )}
          </Message>
        ))}
        {last?.status === 'error' && lastUser && (
          <TurnError onRetry={() => requestCompanionSend(lastUser.content)} />
        )}
      </ConversationContent>
      <ConversationScrollButton containerRef={contentRef} />
    </Conversation>
  )
}

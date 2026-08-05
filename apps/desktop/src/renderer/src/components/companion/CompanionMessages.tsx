import { useEffect, useRef, useState } from 'react'
import { CircleEllipsis, Copy, MessageSquare, Check } from 'lucide-react'
import type { CompanionMessage } from '../../../../shared/types'
import { useAppStore } from '../../store/app-store'
import {
  Conversation,
  ConversationContent,
  ConversationEmptyState,
  ConversationScrollButton,
} from '../ai-elements/conversation'
import { Message, MessageContent, MessageResponse } from '../ai-elements/message'
import { Reasoning, ReasoningContent, ReasoningTrigger } from '../ai-elements/reasoning'
import { Source, SourcesCollapsible } from '../ai-elements/sources'
import { Tool, ToolContent, ToolHeader, ToolInput, ToolOutput } from '../ai-elements/tool'
import { Button } from '../ui/button'
import { COMPANION_PREFILL_EVENT } from './CompanionComposer'

const SUGGESTED_PROMPTS = [
  'Summarize this document',
  'What are the key open questions?',
  'Find related notes',
  'Explain the architecture',
]

function openCitation(path: string) {
  window.dispatchEvent(
    new CustomEvent('mdow:open-document-link', {
      detail: { path },
    }),
  )
}

function prefillComposer(text: string) {
  window.dispatchEvent(new CustomEvent(COMPANION_PREFILL_EVENT, { detail: { text } }))
}

function AssistantCopyButton({ message }: { message: CompanionMessage }) {
  const [copied, setCopied] = useState(false)
  if (!message.content.trim()) return null
  return (
    <Button
      type="button"
      size="icon-xs"
      variant="ghost"
      aria-label={copied ? 'Copied' : 'Copy response'}
      className="mt-1 self-start text-muted-foreground opacity-0 transition-opacity focus-visible:opacity-100 group-hover:opacity-100"
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

function AssistantParts({ message }: { message: CompanionMessage }) {
  const streaming = message.status === 'streaming'
  const lastTextIndex = message.parts.findLastIndex((part) => part.kind === 'text')
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
          {message.citations.map((citation) => (
            <Source
              key={`${message.id}-${citation.sourceId}`}
              title={citation.label}
              onClick={() => openCitation(citation.path)}
            />
          ))}
        </SourcesCollapsible>
      )}
      {!streaming && <AssistantCopyButton message={message} />}
    </>
  )
}

export function CompanionMessages() {
  const messages = useAppStore((state) => state.companionMessages)
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
    return (
      <ConversationEmptyState
        icon={<MessageSquare className="size-5 text-muted-foreground/60" />}
        title="Ask about these docs"
        description="The focused document stays lean. Add files with @ or ask across the folder when needed."
      >
        <div className="mt-3 flex w-full max-w-64 flex-col gap-1.5">
          {SUGGESTED_PROMPTS.map((prompt) => (
            <button
              key={prompt}
              type="button"
              className="rounded-md border border-border-subtle bg-background/60 px-3 py-2 text-left text-xs text-muted-foreground transition-colors hover:border-border hover:text-foreground"
              onClick={() => prefillComposer(prompt)}
            >
              {prompt}
            </button>
          ))}
        </div>
      </ConversationEmptyState>
    )
  }

  return (
    <Conversation>
      <ConversationContent ref={contentRef}>
        {messages.map((message) => (
          <Message key={message.id} from={message.role === 'system' ? 'assistant' : message.role}>
            <MessageContent>
              {message.role === 'user' ? (
                <p className="whitespace-pre-wrap">{message.content}</p>
              ) : (
                <AssistantParts message={message} />
              )}
            </MessageContent>
          </Message>
        ))}
      </ConversationContent>
      <ConversationScrollButton containerRef={contentRef} />
    </Conversation>
  )
}

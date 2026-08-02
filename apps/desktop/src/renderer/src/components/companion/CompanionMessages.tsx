import { useEffect, useRef } from 'react'
import { CircleEllipsis, MessageSquare } from 'lucide-react'
import type { CompanionMessage } from '../../../../shared/types'
import { useAppStore } from '../../store/app-store'
import {
  Conversation,
  ConversationContent,
  ConversationEmptyState,
} from '../ai-elements/conversation'
import { Message, MessageContent, MessageResponse } from '../ai-elements/message'
import { Reasoning, ReasoningContent, ReasoningTrigger } from '../ai-elements/reasoning'
import { Source, SourcesCollapsible } from '../ai-elements/sources'
import { Tool, ToolContent, ToolHeader, ToolInput, ToolOutput } from '../ai-elements/tool'

function openCitation(path: string) {
  window.dispatchEvent(
    new CustomEvent('mdow:open-document-link', {
      detail: { path },
    }),
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
    </>
  )
}

export function CompanionMessages() {
  const messages = useAppStore((state) => state.companionMessages)
  const bottomRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ block: 'end' })
  }, [messages])

  if (messages.length === 0) {
    return (
      <ConversationEmptyState
        icon={<MessageSquare className="size-5 text-muted-foreground/60" />}
        title="Ask about these docs"
        description="The focused document stays lean. Add files with @ or ask across the folder when needed."
      />
    )
  }

  return (
    <Conversation>
      <ConversationContent>
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
        <div ref={bottomRef} />
      </ConversationContent>
    </Conversation>
  )
}

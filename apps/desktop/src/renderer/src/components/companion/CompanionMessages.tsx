import {
  Conversation,
  ConversationContent,
  ConversationEmptyState,
  ConversationScrollButton,
} from '@renderer/components/ai-elements/conversation'
import { Message, MessageContent, MessageResponse } from '@renderer/components/ai-elements/message'
import { Sources, SourcesContent, SourcesTrigger } from '@renderer/components/ai-elements/sources'
import { Button } from '@renderer/components/ui/button'
import { useAppStore } from '@renderer/store/app-store'
import type { CompanionCitation } from '../../../../shared/types'
import { useOpenMarkdownFile } from '../../hooks/useOpenMarkdownFile'

export function CompanionMessages() {
  const messages = useAppStore((state) => state.companionMessages)

  return (
    <Conversation className="min-h-0">
      <ConversationContent className="gap-5 p-3">
        {messages.length === 0 ? (
          <ConversationEmptyState
            title="Ask about this document"
            description="The companion can use your open Markdown file and folder context."
          />
        ) : (
          messages.map((message) => {
            const content =
              message.role === 'assistant' && message.status === 'streaming' && !message.content
                ? 'Thinking...'
                : message.content

            return (
              <Message from={message.role} key={message.id}>
                <MessageContent>
                  {message.citations.length > 0 && (
                    <Sources>
                      <SourcesTrigger count={message.citations.length} />
                      <SourcesContent>
                        {message.citations.map((citation) => (
                          <CompanionCitationButton key={citation.sourceId} citation={citation} />
                        ))}
                      </SourcesContent>
                    </Sources>
                  )}
                  <MessageResponse>{content}</MessageResponse>
                </MessageContent>
              </Message>
            )
          })
        )}
      </ConversationContent>
      <ConversationScrollButton />
    </Conversation>
  )
}

function CompanionCitationButton({ citation }: { citation: CompanionCitation }) {
  const openMarkdownFile = useOpenMarkdownFile()

  return (
    <Button
      type="button"
      variant="outline"
      size="xs"
      onClick={() => void openMarkdownFile(citation.path)}
    >
      {citation.title}
    </Button>
  )
}

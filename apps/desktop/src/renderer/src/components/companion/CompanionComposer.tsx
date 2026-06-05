import {
  PromptInput,
  PromptInputBody,
  PromptInputFooter,
  PromptInputSubmit,
  PromptInputTextarea,
  type PromptInputMessage,
} from '@renderer/components/ai-elements/prompt-input'
import { useAppStore } from '@renderer/store/app-store'
import { SquareIcon } from 'lucide-react'
import { useState } from 'react'
import { useCompanionController } from '../../hooks/useCompanionController'

export function CompanionComposer() {
  const streaming = useAppStore((state) => state.companionStreaming)
  const { cancel, send } = useCompanionController()
  const [text, setText] = useState('')
  const trimmed = text.trim()

  function handleSubmit(message: PromptInputMessage) {
    if (streaming) {
      void cancel()
      return
    }

    const nextText = message.text.trim()
    if (!nextText) return

    setText('')
    void send(nextText)
  }

  return (
    <PromptInput className="shrink-0" onSubmit={handleSubmit}>
      <PromptInputBody>
        <PromptInputTextarea
          aria-label="Companion prompt"
          className="text-base md:text-sm"
          onChange={(event) => setText(event.currentTarget.value)}
          placeholder="Ask about this document..."
          value={text}
        />
      </PromptInputBody>
      <PromptInputFooter>
        <div className="text-muted-foreground text-xs">Local ACP</div>
        <PromptInputSubmit
          aria-label={streaming ? 'Cancel companion response' : 'Send companion message'}
          disabled={!streaming && !trimmed}
        >
          {streaming ? <SquareIcon className="size-3" /> : undefined}
        </PromptInputSubmit>
      </PromptInputFooter>
    </PromptInput>
  )
}

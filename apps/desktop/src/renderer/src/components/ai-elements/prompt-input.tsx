import { Button } from '@renderer/components/ui/button'
import { Textarea } from '@renderer/components/ui/textarea'
import { cn } from '@renderer/lib/utils'
import { ArrowUpIcon } from 'lucide-react'
import type { ComponentProps, FormEvent } from 'react'

type PromptInputFile = {
  filename?: string
  mediaType?: string
  type: 'file'
  url: string
}

export type PromptInputMessage = {
  text: string
  files: PromptInputFile[]
}

type PromptInputProps = Omit<ComponentProps<'form'>, 'onSubmit'> & {
  onSubmit?: (message: PromptInputMessage, event: FormEvent<HTMLFormElement>) => void
}

export const PromptInput = ({ className, onSubmit, ...props }: PromptInputProps) => {
  const handleSubmit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault()
    const value = new FormData(event.currentTarget).get('message')

    onSubmit?.(
      {
        files: [],
        text: typeof value === 'string' ? value : '',
      },
      event,
    )
  }

  return (
    <form
      className={cn(
        'border-input bg-background focus-within:ring-ring/50 flex flex-col rounded-xl border shadow-xs focus-within:ring-[3px]',
        className,
      )}
      onSubmit={handleSubmit}
      {...props}
    />
  )
}

type PromptInputBodyProps = ComponentProps<'div'>

export const PromptInputBody = ({ className, ...props }: PromptInputBodyProps) => (
  <div className={cn('flex flex-col', className)} {...props} />
)

type PromptInputFooterProps = ComponentProps<'div'>

export const PromptInputFooter = ({ className, ...props }: PromptInputFooterProps) => (
  <div className={cn('flex items-center justify-between gap-2 p-2', className)} {...props} />
)

type PromptInputSubmitProps = ComponentProps<typeof Button>

export const PromptInputSubmit = ({ children, className, ...props }: PromptInputSubmitProps) => (
  <Button
    className={cn('size-8 rounded-full', className)}
    aria-label="Send message"
    size="icon"
    type="submit"
    {...props}
  >
    {children ?? <ArrowUpIcon className="size-4" />}
  </Button>
)

type PromptInputTextareaProps = ComponentProps<typeof Textarea>

export const PromptInputTextarea = ({
  className,
  name = 'message',
  ...props
}: PromptInputTextareaProps) => (
  <Textarea
    className={cn(
      'max-h-48 min-h-16 resize-none border-0 bg-transparent shadow-none focus-visible:ring-0',
      className,
    )}
    name={name}
    {...props}
  />
)

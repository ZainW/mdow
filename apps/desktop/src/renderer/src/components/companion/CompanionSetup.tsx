import { Button } from '@renderer/components/ui/button'
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@renderer/components/ui/card'
import { Input } from '@renderer/components/ui/input'
import { useAppStore } from '@renderer/store/app-store'
import type { CompanionProviderStatus } from '../../../../shared/types'
import { RefreshCwIcon } from 'lucide-react'
import { useState } from 'react'

const fallbackProviders: Pick<CompanionProviderStatus, 'id' | 'label' | 'command'>[] = [
  { id: 'opencode', label: 'opencode', command: 'opencode acp' },
  {
    id: 'codex',
    label: 'Codex ACP',
    command: 'npx --no-install @zed-industries/codex-acp',
  },
]

export function CompanionSetup() {
  const customCommand = useAppStore((state) => state.companionCustomCommand)
  const providers = useAppStore((state) => state.companionProviders)
  const setCustomCommand = useAppStore((state) => state.setCompanionCustomCommand)
  const setProviders = useAppStore((state) => state.setCompanionProviders)
  const setError = useAppStore((state) => state.setCompanionError)
  const [detecting, setDetecting] = useState(false)

  async function detectProviders() {
    if (typeof window === 'undefined' || !window.api) return

    setDetecting(true)
    setError(null)
    try {
      setProviders(await window.api.detectCompanionProviders())
    } catch (error) {
      setError(error instanceof Error ? error.message : String(error))
    } finally {
      setDetecting(false)
    }
  }

  const rows = providers.length > 0 ? providers : fallbackProviders

  return (
    <Card size="sm" className="bg-background/60">
      <CardHeader>
        <CardTitle>Connect a local companion</CardTitle>
        <CardDescription>
          Install an ACP provider or enter a custom command to start chatting with your Markdown
          context.
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="space-y-2">
          {rows.map((provider) => (
            <div
              className="rounded-md border border-border bg-input/20 px-3 py-2"
              key={provider.id}
            >
              <div className="text-xs font-medium">{provider.label}</div>
              <div className="mt-1 font-mono text-muted-foreground text-xs">{provider.command}</div>
            </div>
          ))}
        </div>
        <div className="space-y-2">
          <label className="text-xs font-medium" htmlFor="companion-custom-command">
            Custom command
          </label>
          <Input
            id="companion-custom-command"
            value={customCommand}
            onChange={(event) => setCustomCommand(event.currentTarget.value)}
            placeholder="custom-acp --stdio"
          />
        </div>
        <Button
          className="w-full"
          disabled={detecting}
          onClick={() => void detectProviders()}
          type="button"
          variant="outline"
        >
          <RefreshCwIcon className="size-3.5" />
          {detecting ? 'Detecting...' : 'Retry detection'}
        </Button>
      </CardContent>
    </Card>
  )
}

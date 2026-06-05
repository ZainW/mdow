import { Badge } from '@renderer/components/ui/badge'
import { getReadyCompanionProvider } from '@renderer/lib/companion-provider'
import { useAppStore } from '@renderer/store/app-store'

export function CompanionStatus() {
  const provider = useAppStore((state) =>
    getReadyCompanionProvider({
      provider: state.companionProvider,
      customCommand: state.companionCustomCommand,
      providers: state.companionProviders,
    }),
  )

  return (
    <Badge variant={provider ? 'secondary' : 'outline'}>{provider?.label ?? 'Not connected'}</Badge>
  )
}

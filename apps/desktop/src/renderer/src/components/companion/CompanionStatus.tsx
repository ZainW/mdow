import { Badge } from '@renderer/components/ui/badge'
import { useAppStore } from '@renderer/store/app-store'

export function CompanionStatus() {
  const provider = useAppStore((state) =>
    state.companionProviders.find((item) => item.status === 'available'),
  )

  return (
    <Badge variant={provider ? 'secondary' : 'outline'}>{provider?.label ?? 'Not connected'}</Badge>
  )
}

import type { CompanionProviderId, CompanionProviderStatus } from '../../../shared/types'

export function getReadyCompanionProvider({
  provider,
  customCommand,
  providers,
}: {
  provider: CompanionProviderId
  customCommand: string
  providers: CompanionProviderStatus[]
}): CompanionProviderStatus | null {
  if (provider === 'auto') {
    return providers.find((item) => item.status === 'available') ?? null
  }

  if (provider === 'custom' && customCommand.trim().length === 0) {
    return null
  }

  return providers.find((item) => item.id === provider && item.status === 'available') ?? null
}

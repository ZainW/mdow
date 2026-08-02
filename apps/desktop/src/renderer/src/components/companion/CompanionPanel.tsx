import { useEffect, type ReactNode } from 'react'
import type { CompanionProviderId, CompanionProviderStatus } from '../../../../shared/types'
import { cn, isMac } from '../../lib/utils'
import { useAppStore } from '../../store/app-store'
import { Badge } from '../ui/badge'
import { Button } from '../ui/button'
import { CompanionComposer } from './CompanionComposer'
import { CompanionContextBar } from './CompanionContextBar'
import { CompanionHeader } from './CompanionHeader'
import { CompanionMessages } from './CompanionMessages'

export function selectAvailableProvider(
  providers: CompanionProviderStatus[],
  preferred: CompanionProviderId | null,
): CompanionProviderId | null {
  const preferredStatus = providers.find((provider) => provider.id === preferred)
  if (preferredStatus?.availability === 'available') return preferredStatus.id
  return providers.find((provider) => provider.availability === 'available')?.id ?? null
}

function CompanionSetup({ providers }: { providers: CompanionProviderStatus[] }) {
  const customCommand = useAppStore((state) => state.companionCustomCommand)
  const setCustomCommand = useAppStore((state) => state.setCompanionCustomCommand)
  const preferred = useAppStore((state) => state.companionPreferredProvider)
  const setPreferred = useAppStore((state) => state.setCompanionPreferredProvider)
  const loadModels = useAppStore((state) => state.loadCompanionModels)

  const chooseCustomExecutable = async () => {
    const executablePath = await window.api.chooseCompanionCustomExecutable()
    if (!executablePath) return
    setCustomCommand(executablePath)
    setPreferred('custom')
    const list = await window.api.detectCompanionProviders()
    useAppStore.getState().setCompanionProviders(list)
    await loadModels()
  }

  return (
    <div className="flex flex-1 flex-col gap-3 overflow-y-auto p-3 text-sm">
      <p className="text-muted-foreground">
        Connect a local ACP agent already on this computer. Mdow will not install packages for you.
      </p>
      <ul className="flex flex-col gap-2">
        {providers.map((provider) => (
          <li
            key={provider.id}
            className={cn(
              'rounded-md border border-border-subtle p-2',
              preferred === provider.id && 'border-primary/40 bg-muted/40',
            )}
          >
            <div className="flex items-center justify-between gap-2">
              <div>
                <p className="font-medium text-foreground">{provider.label}</p>
                <p className="text-xs text-muted-foreground">{provider.commandDisplay}</p>
              </div>
              <Badge variant={provider.availability === 'available' ? 'default' : 'secondary'}>
                {provider.availability}
              </Badge>
            </div>
            {provider.detail && (
              <p className="mt-1 text-xs text-muted-foreground">{provider.detail}</p>
            )}
            {provider.availability === 'available' && (
              <Button
                size="sm"
                variant="outline"
                className="mt-2"
                onClick={() => {
                  setPreferred(provider.id)
                  void loadModels()
                }}
              >
                Use {provider.label}
              </Button>
            )}
          </li>
        ))}
      </ul>
      <div className="flex flex-col gap-1.5">
        <p className="text-xs font-medium text-muted-foreground">Custom ACP executable</p>
        {customCommand && (
          <p className="break-all rounded-md border border-border-subtle bg-muted/40 p-2 font-mono text-xs">
            {customCommand}
          </p>
        )}
        <p className="text-xs text-muted-foreground">
          Choose one executable. Arguments and shell commands are not accepted.
        </p>
        <Button size="sm" variant="secondary" onClick={() => void chooseCustomExecutable()}>
          Choose executable…
        </Button>
      </div>
      <Button
        size="sm"
        variant="ghost"
        onClick={() => {
          void window.api.detectCompanionProviders().then((list) => {
            useAppStore.getState().setCompanionProviders(list)
          })
        }}
      >
        Retry detection
      </Button>
    </div>
  )
}

function CompanionBody({
  layout = 'drawer',
  onExpand,
  onBack,
  onClose,
}: {
  layout?: 'drawer' | 'workspace'
  onExpand?: () => void
  onBack?: () => void
  onClose?: () => void
}) {
  const providers = useAppStore((state) => state.companionProviders)
  const preferred = useAppStore((state) => state.companionPreferredProvider)
  const trace = useAppStore((state) => state.companionContextTrace)
  const warnings = useAppStore((state) => state.companionWarnings)
  const providerId = selectAvailableProvider(providers, preferred)

  return (
    <div className="flex h-full min-h-0 flex-col">
      <CompanionHeader layout={layout} onBack={onBack} onExpand={onExpand} onClose={onClose} />
      <div
        className={cn(
          'flex min-h-0 flex-1 flex-col',
          layout === 'workspace' && 'mx-auto w-full max-w-5xl',
        )}
      >
        {providerId ? (
          <>
            <CompanionMessages />
            <div className="shrink-0 border-t border-border-subtle">
              <CompanionContextBar trace={trace} warnings={warnings} />
              <CompanionComposer providerId={providerId} />
            </div>
          </>
        ) : (
          <CompanionSetup providers={providers} />
        )}
      </div>
    </div>
  )
}

export function useCompanionBootstrap() {
  useEffect(() => {
    return window.api.onCompanionUpdate((update) => {
      useAppStore.getState().applyCompanionUpdate(update)
    })
  }, [])
}

async function refreshCompanionMeta() {
  const [providers, settings] = await Promise.all([
    window.api.detectCompanionProviders(),
    window.api.getCompanionSettings(),
  ])
  const preferredProvider = selectAvailableProvider(providers, settings.preferredProvider)
  useAppStore.setState({
    companionProviders: providers,
    companionPreferredProvider: preferredProvider,
    companionCustomCommand: settings.customCommand,
  })
  if (preferredProvider !== settings.preferredProvider) {
    await window.api.saveCompanionSettings({ preferredProvider })
  }
  if (preferredProvider) await useAppStore.getState().loadCompanionModels()
}

export function CompanionPanel() {
  const presentation = useAppStore((state) => state.companionPresentation)
  const setPresentation = useAppStore((state) => state.setCompanionPresentation)
  const open = presentation === 'drawer'

  useEffect(() => {
    if (open) void refreshCompanionMeta()
  }, [open])

  return (
    <aside
      aria-label="AI companion"
      className={cn(
        'overflow-hidden bg-background',
        'max-lg:fixed max-lg:right-0 max-lg:bottom-0 max-lg:z-40 max-lg:shadow-xl max-lg:ring-1 max-lg:ring-foreground/10 max-lg:dark:shadow-none',
        isMac ? 'max-lg:top-7' : 'max-lg:top-0',
        'lg:relative lg:z-auto lg:shrink-0 lg:border-l lg:border-border-subtle',
        open
          ? 'max-lg:w-[min(24rem,calc(100vw-1rem))] lg:w-(--companion-drawer-width)'
          : 'w-0 max-lg:pointer-events-none',
      )}
      aria-hidden={!open}
      inert={!open ? true : undefined}
    >
      <div className="flex h-full w-full flex-col lg:w-(--companion-drawer-width)">
        <CompanionBody
          onExpand={() => setPresentation('workspace')}
          onClose={() => setPresentation('closed')}
        />
      </div>
    </aside>
  )
}

export function CompanionWorkspace() {
  const presentation = useAppStore((state) => state.companionPresentation)
  const setPresentation = useAppStore((state) => state.setCompanionPresentation)

  useEffect(() => {
    if (presentation === 'workspace') void refreshCompanionMeta()
  }, [presentation])

  if (presentation !== 'workspace') return null

  return (
    <section
      aria-label="AI companion workspace"
      className="flex min-h-0 flex-1 flex-col overflow-hidden bg-background"
    >
      <CompanionBody layout="workspace" onBack={() => setPresentation('drawer')} />
    </section>
  )
}

export function CompanionShell({ children }: { children: ReactNode }) {
  const presentation = useAppStore((state) => state.companionPresentation)
  return presentation === 'workspace' ? <CompanionWorkspace /> : children
}

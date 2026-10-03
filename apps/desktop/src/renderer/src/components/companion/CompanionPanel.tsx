import { useEffect, useRef, type KeyboardEvent, type ReactNode } from 'react'
import { LoaderCircle, Terminal } from 'lucide-react'
import type { CompanionRuntimeStatus } from '../../../../shared/types'
import { cn, isMac } from '../../lib/utils'
import { useAppStore } from '../../store/app-store'
import { Button } from '../ui/button'
import { CompanionComposer } from './CompanionComposer'
import { CompanionHeader } from './CompanionHeader'
import { CompanionMessages } from './CompanionMessages'

const INSTALL_COMMAND = 'curl -fsSL https://opencode.ai/install | bash'

function CommandLine({ command }: { command: string }) {
  return (
    <code className="flex items-center gap-2 rounded-md bg-muted px-2.5 py-2 font-mono text-[11px] break-all text-foreground select-all">
      <Terminal className="size-3.5 shrink-0 text-muted-foreground" aria-hidden />
      {command}
    </code>
  )
}

function SetupMessage({
  title,
  children,
  action,
}: {
  title: string
  children: ReactNode
  action?: ReactNode
}) {
  return (
    <div className="flex flex-1 flex-col justify-center gap-3 overflow-y-auto px-5 py-6 text-sm">
      <h3 className="font-medium text-foreground">{title}</h3>
      <div className="flex flex-col gap-3 text-xs leading-5 text-muted-foreground">{children}</div>
      {action}
    </div>
  )
}

function CompanionSetup({ status }: { status: CompanionRuntimeStatus | null }) {
  const connect = useAppStore((state) => state.connectCompanion)
  const openFolderPath = useAppStore((state) => state.openFolderPath)
  const retry = (
    <div className="flex gap-2">
      <Button size="sm" variant="secondary" onClick={() => void connect(openFolderPath)}>
        Check again
      </Button>
      <Button
        size="sm"
        variant="ghost"
        onClick={() => void window.api.openExternal('https://opencode.ai/docs')}
      >
        OpenCode docs
      </Button>
    </div>
  )

  if (!status) {
    return (
      <output className="flex flex-1 items-center justify-center gap-2 text-xs text-muted-foreground">
        <LoaderCircle className="size-3.5 motion-safe:animate-spin" aria-hidden />
        Starting OpenCode…
      </output>
    )
  }

  if (status.availability === 'missing') {
    return (
      <SetupMessage title="Connect OpenCode" action={retry}>
        <p>
          The companion runs on OpenCode, using the models and subscriptions you already use there.
          Install OpenCode 2, then sign in to a provider:
        </p>
        <CommandLine command={INSTALL_COMMAND} />
        <CommandLine command="opencode auth login" />
      </SetupMessage>
    )
  }

  if (status.availability === 'outdated') {
    return (
      <SetupMessage title="Update OpenCode" action={retry}>
        <p>OpenCode {status.version} is installed, but the companion needs OpenCode 2 or newer.</p>
        <CommandLine command="opencode upgrade" />
      </SetupMessage>
    )
  }

  return (
    <SetupMessage title="OpenCode didn’t start" action={retry}>
      <p>{status.detail ?? 'Something went wrong while starting OpenCode.'}</p>
    </SetupMessage>
  )
}

function NoModels({ reason }: { reason?: string }) {
  const connect = useAppStore((state) => state.connectCompanion)
  const openFolderPath = useAppStore((state) => state.openFolderPath)
  return (
    <SetupMessage
      title="Connect a model provider"
      action={
        <Button
          size="sm"
          variant="secondary"
          className="w-fit"
          onClick={() => void connect(openFolderPath)}
        >
          Check again
        </Button>
      }
    >
      <p>{reason ?? 'OpenCode has no models available yet.'}</p>
      <CommandLine command="opencode auth login" />
    </SetupMessage>
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
  const status = useAppStore((state) => state.companionStatus)
  const modelState = useAppStore((state) => state.companionModelState)
  const hasMessages = useAppStore((state) => state.companionMessages.length > 0)
  const ready = status?.availability === 'available'
  const noModels = ready && !modelState.stale && modelState.options.length === 0

  return (
    <div className="flex h-full min-h-0 flex-col">
      <CompanionHeader layout={layout} onBack={onBack} onExpand={onExpand} onClose={onClose} />
      <div
        className={cn(
          'flex min-h-0 flex-1 flex-col',
          layout === 'workspace' && 'mx-auto w-full max-w-3xl',
        )}
      >
        {!ready ? (
          <CompanionSetup status={status} />
        ) : noModels && !hasMessages ? (
          <NoModels reason={modelState.unavailableReason} />
        ) : (
          <>
            <CompanionMessages />
            <CompanionComposer />
          </>
        )}
      </div>
    </div>
  )
}

function useConnectWhenShown(visible: boolean) {
  const openFolderPath = useAppStore((state) => state.openFolderPath)
  useEffect(() => {
    if (!visible) return
    const { companionStatus, connectCompanion } = useAppStore.getState()
    if (companionStatus?.availability !== 'available') void connectCompanion(openFolderPath)
  }, [visible, openFolderPath])
}

export function CompanionPanel() {
  const presentation = useAppStore((state) => state.companionPresentation)
  const setPresentation = useAppStore((state) => state.setCompanionPresentation)
  const open = presentation === 'drawer'
  const asideRef = useRef<HTMLElement>(null)
  const returnFocusRef = useRef<HTMLElement | null>(null)
  const wasOpenRef = useRef(open)

  useConnectWhenShown(open)

  // Move focus into the drawer when the user opens it, and hand it back when it closes.
  // The initial mount is skipped so a restored session never steals focus on launch.
  useEffect(() => {
    const wasOpen = wasOpenRef.current
    wasOpenRef.current = open
    if (open === wasOpen) return
    const aside = asideRef.current
    if (open) {
      const active = document.activeElement
      returnFocusRef.current = active instanceof HTMLElement ? active : null
      const target =
        aside?.querySelector<HTMLElement>('textarea') ??
        aside?.querySelector<HTMLElement>('button:not([disabled])')
      target?.focus()
      return
    }
    if (aside?.contains(document.activeElement)) returnFocusRef.current?.focus()
    returnFocusRef.current = null
  }, [open])

  const handleKeyDown = (event: KeyboardEvent<HTMLElement>) => {
    if (event.key !== 'Escape' || event.defaultPrevented) return
    // Portaled popups (model picker, context popover) bubble through React but not the DOM.
    if (!(event.target instanceof Node) || !event.currentTarget.contains(event.target)) return
    event.stopPropagation()
    setPresentation('closed')
  }

  return (
    // oxlint-disable-next-line jsx-a11y/no-noninteractive-element-interactions -- Escape closes the drawer from any control inside it
    <aside
      ref={asideRef}
      aria-label="AI companion"
      onKeyDown={handleKeyDown}
      className={cn(
        'overflow-hidden bg-background',
        'max-lg:fixed max-lg:right-0 max-lg:bottom-0 max-lg:z-(--z-drawer) max-lg:shadow-xl max-lg:ring-1 max-lg:ring-foreground/10 max-lg:dark:shadow-none',
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

  useConnectWhenShown(presentation === 'workspace')

  if (presentation !== 'workspace') return null

  const handleKeyDown = (event: KeyboardEvent<HTMLElement>) => {
    if (event.key !== 'Escape' || event.defaultPrevented) return
    if (!(event.target instanceof Node) || !event.currentTarget.contains(event.target)) return
    event.stopPropagation()
    setPresentation('drawer')
  }

  return (
    // oxlint-disable-next-line jsx-a11y/no-noninteractive-element-interactions -- Escape returns to the document from any control inside it
    <section
      aria-label="AI companion workspace"
      onKeyDown={handleKeyDown}
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

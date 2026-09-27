import { lazy, useEffect, type ReactNode } from 'react'
import { useAppStore } from '../../store/app-store'
import { DeferredMount } from '../DeferredMount'

// The companion UI is sizable and most sessions never open it, so it loads on first use.
const CompanionPanel = lazy(() =>
  import('./CompanionPanel').then((mod) => ({ default: mod.CompanionPanel })),
)
const CompanionWorkspace = lazy(() =>
  import('./CompanionPanel').then((mod) => ({ default: mod.CompanionWorkspace })),
)

export function useCompanionBootstrap() {
  useEffect(() => {
    return window.api.onCompanionUpdate((update) => {
      useAppStore.getState().applyCompanionUpdate(update)
    })
  }, [])
}

export function LazyCompanionPanel() {
  const presentation = useAppStore((state) => state.companionPresentation)
  return (
    <DeferredMount when={presentation !== 'closed'}>
      <CompanionPanel />
    </DeferredMount>
  )
}

export function LazyCompanionShell({ children }: { children: ReactNode }) {
  const presentation = useAppStore((state) => state.companionPresentation)
  if (presentation !== 'workspace') return children
  return (
    <DeferredMount when>
      <CompanionWorkspace />
    </DeferredMount>
  )
}

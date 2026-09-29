import { basename, dirname, sep } from 'node:path'
import { IS_MAC, revealInFileManager } from '../lib/platform'
import {
  activateTab,
  closeTab,
  setPrefs,
  toggleOverlay,
  toggleSidebar,
  useApp,
  type Tab,
} from '../store'
import { METRICS, useUi } from './context'
import { activateOnEnter, Icon, IconButton, Label } from './primitives'

/** The strip beside the traffic lights. We paint it because the native titlebar is hidden. */
export function Titlebar() {
  const { theme } = useUi()
  const sidebarOpen = useApp((state) => state.sidebarOpen)
  return (
    <div
      style={{
        display: 'flex',
        alignItems: 'center',
        height: METRICS.titlebarHeight,
        paddingLeft: IS_MAC ? METRICS.trafficLightClearance : 8,
        flexShrink: 0,
        backgroundColor: theme.sidebar,
        borderBottomWidth: 1,
        borderColor: theme.border,
      }}
    >
      <IconButton
        icon="sidebar"
        label={sidebarOpen ? 'Hide sidebar' : 'Show sidebar'}
        testId="toggle-sidebar"
        onClick={toggleSidebar}
      />
    </div>
  )
}

export function TabBar() {
  const { theme } = useUi()
  const tabs = useApp((state) => state.tabs)
  const activePath = useApp((state) => state.activePath)
  return (
    <div
      style={{
        display: 'flex',
        alignItems: 'center',
        height: METRICS.chromeRowHeight,
        borderBottomWidth: 1,
        borderColor: theme.border,
        flexShrink: 0,
      }}
    >
      <div
        role="tablist"
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: 1,
          flexGrow: 1,
          minWidth: 0,
          paddingLeft: 6,
          paddingRight: 6,
          overflowX: 'scroll',
        }}
      >
        {tabs.length === 0 ? (
          <div style={{ paddingLeft: 6 }}>
            <Label color={theme.mutedForeground}>No document</Label>
          </div>
        ) : (
          tabs.map((tab) => <TabItem key={tab.path} tab={tab} active={tab.path === activePath} />)
        )}
      </div>
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: 2,
          height: '100%',
          paddingLeft: 8,
          paddingRight: 8,
          borderLeftWidth: 1,
          borderColor: theme.border,
          flexShrink: 0,
        }}
      >
        <IconButton
          icon="search"
          label="Find"
          testId="find-button"
          onClick={() => toggleOverlay('find')}
        />
        <IconButton
          icon="command"
          label="Command palette"
          testId="palette-button"
          onClick={() => toggleOverlay('palette')}
        />
        <IconButton icon="settings" label="Settings" onClick={() => toggleOverlay('settings')} />
      </div>
    </div>
  )
}

function TabItem({ tab, active }: { tab: Tab; active: boolean }) {
  const { theme } = useUi()
  const title = basename(tab.path)
  return (
    <div
      role="tab"
      aria-selected={active}
      aria-label={title}
      testId={`tab-${title}`}
      tabIndex={0}
      onClick={() => activateTab(tab.path)}
      onAuxClick={(event) => {
        if (event.button === 1) closeTab(tab.path)
      }}
      onKeyDown={activateOnEnter(() => activateTab(tab.path))}
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: 6,
        height: METRICS.tabHeight + 4,
        maxWidth: METRICS.tabMaxWidth,
        paddingLeft: 10,
        paddingRight: 4,
        borderRadius: 6,
        borderWidth: 1,
        borderColor: active ? theme.border : '#00000000',
        backgroundColor: active ? theme.surfaceRaised : undefined,
        cursor: 'pointer',
        flexShrink: 0,
        hover: active ? undefined : { backgroundColor: theme.sidebarAccent },
        userSelect: 'none',
      }}
    >
      <Icon
        name={tab.document.ok ? 'file' : 'alert-circle'}
        size={14}
        color={active ? theme.foreground : theme.mutedForeground}
      />
      <Label
        color={active ? theme.foreground : theme.mutedForeground}
        weight={active ? 500 : 400}
        style={{ whiteSpace: 'nowrap', textOverflow: 'ellipsis', minWidth: 0, flexShrink: 1 }}
      >
        {title}
      </Label>
      <div
        role="button"
        aria-label={`Close ${title}`}
        tabIndex={-1}
        onClick={() => closeTab(tab.path)}
        onKeyDown={activateOnEnter(() => closeTab(tab.path))}
        style={{
          width: 20,
          height: 20,
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          borderRadius: 4,
          flexShrink: 0,
          cursor: 'pointer',
          hover: { backgroundColor: theme.muted },
        }}
      >
        <Icon name="x" size={12} />
      </div>
    </div>
  )
}

export function Breadcrumb({ tab }: { tab: Tab | null }) {
  const { theme } = useUi()
  const wideMode = useApp((state) => state.prefs.wideMode)
  const title = tab?.document.ok ? tab.document.parsed.title : null
  const segments = tab ? breadcrumbSegments(tab.path) : []
  return (
    <div
      style={{
        display: 'flex',
        alignItems: 'center',
        height: METRICS.breadcrumbHeight,
        paddingLeft: 14,
        paddingRight: 8,
        gap: 4,
        borderBottomWidth: 1,
        borderColor: theme.border,
        flexShrink: 0,
        userSelect: 'none',
      }}
    >
      <div style={{ display: 'flex', alignItems: 'center', gap: 4, flexGrow: 1, minWidth: 0 }}>
        {tab ? (
          segments.map((segment, index) => {
            const last = index === segments.length - 1
            return (
              <div
                key={segment.path}
                style={{ display: 'flex', alignItems: 'center', gap: 4, minWidth: 0 }}
              >
                {index > 0 ? <Icon name="chevron-right" size={12} /> : null}
                <div
                  role="button"
                  tabIndex={0}
                  onClick={() => void revealInFileManager(segment.path)}
                  onKeyDown={activateOnEnter(() => void revealInFileManager(segment.path))}
                  style={{
                    paddingLeft: 2,
                    paddingRight: 2,
                    borderRadius: 4,
                    cursor: 'pointer',
                    minWidth: 0,
                    hover: { backgroundColor: theme.sidebarAccent },
                  }}
                >
                  <Label
                    color={last ? theme.foreground : theme.mutedForeground}
                    style={{ whiteSpace: 'nowrap', textOverflow: 'ellipsis' }}
                  >
                    {last && title ? title : segment.name}
                  </Label>
                </div>
              </div>
            )
          })
        ) : (
          <Label color={theme.mutedForeground}>Welcome</Label>
        )}
      </div>
      {tab ? (
        <IconButton
          icon="expand"
          label={wideMode ? 'Exit wide mode' : 'Wide mode'}
          active={wideMode}
          onClick={() => setPrefs({ wideMode: !wideMode })}
        />
      ) : null}
    </div>
  )
}

/** The file plus its last three parent folders. */
export function breadcrumbSegments(path: string) {
  const segments: { name: string; path: string }[] = [{ name: basename(path), path }]
  let current = dirname(path)
  while (segments.length < 4 && current && current !== dirname(current)) {
    segments.unshift({ name: basename(current), path: current })
    current = dirname(current)
  }
  return segments.filter((segment) => segment.name && segment.name !== sep)
}

export function ReloadBanner() {
  const { theme } = useUi()
  return (
    <div
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: 8,
        paddingLeft: 14,
        paddingRight: 14,
        paddingTop: 8,
        paddingBottom: 8,
        backgroundColor: theme.muted,
        borderBottomWidth: 1,
        borderColor: theme.border,
        flexShrink: 0,
      }}
    >
      <Icon name="alert-circle" size={14} color={theme.destructive} />
      <Label color={theme.mutedForeground}>
        Couldn't reload this file. Showing the last version that loaded.
      </Label>
    </div>
  )
}

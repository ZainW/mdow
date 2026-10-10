import { basename, dirname, sep } from 'node:path'
import { documentKind } from '../lib/documents'
import { openExternal, revealInFileManager } from '../lib/platform'
import { activateTab, closeTab, setPrefs, toggleOverlay, useApp, type Tab } from '../store'
import { METRICS, useUi } from './context'
import { activateOnEnter, Icon, IconButton, Label } from './primitives'
import { READER_ICONS, type ReaderIconName } from './reader-icons'

/**
 * The strip the traffic lights sit in. Like the desktop app it holds nothing: the sidebar
 * toggles with ⌘B and the View menu.
 */
export function Titlebar() {
  const { theme } = useUi()
  return (
    <div
      style={{ height: METRICS.titlebarHeight, flexShrink: 0, backgroundColor: theme.background }}
    />
  )
}

function ChromeIcon({ name, size, color }: { name: ReaderIconName; size: number; color: string }) {
  return (
    <svg source={READER_ICONS[name]} style={{ width: size, height: size, flexShrink: 0, color }} />
  )
}

export function TabBar() {
  const { theme } = useUi()
  const tabs = useApp((state) => state.tabs)
  const activePath = useApp((state) => state.activePath)
  const activeIndex = tabs.findIndex((tab) => tab.path === activePath)
  return (
    <div
      style={{
        display: 'flex',
        alignItems: 'stretch',
        height: METRICS.chromeRowHeight,
        borderBottomWidth: 1,
        borderColor: theme.borderSubtle,
        backgroundColor: theme.background,
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
        {tabs.map((tab, index) => (
          <TabItem
            key={tab.path}
            tab={tab}
            active={index === activeIndex}
            // A hairline between two inactive neighbours, as on desktop.
            separator={index > 0 && index !== activeIndex && index - 1 !== activeIndex}
          />
        ))}
      </div>
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: 2,
          paddingLeft: 6,
          paddingRight: 6,
          borderLeftWidth: tabs.length > 0 ? 1 : 0,
          borderColor: theme.borderSubtle,
          flexShrink: 0,
        }}
      >
        <IconButton
          icon="search"
          label="Find"
          testId="find-button"
          size={24}
          onClick={() => toggleOverlay('find')}
        />
        <IconButton
          icon="command"
          label="Command palette"
          testId="palette-button"
          size={24}
          onClick={() => toggleOverlay('palette')}
        />
      </div>
    </div>
  )
}

function TabItem({ tab, active, separator }: { tab: Tab; active: boolean; separator: boolean }) {
  const { theme, scale } = useUi()
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
        position: 'relative',
        display: 'flex',
        alignItems: 'center',
        gap: 6,
        height: METRICS.tabHeight,
        maxWidth: METRICS.tabMaxWidth,
        paddingLeft: 10,
        paddingRight: 4,
        borderRadius: 6,
        borderWidth: 1,
        borderColor: active ? theme.borderSubtle : '#00000000',
        backgroundColor: active ? theme.background : undefined,
        boxShadow: active
          ? { offsetX: 0, offsetY: 1, blurRadius: 2, spreadRadius: 0, color: '#0000000a' }
          : undefined,
        cursor: 'pointer',
        flexShrink: 0,
        userSelect: 'none',
      }}
    >
      {separator ? (
        <div
          style={{
            position: 'absolute',
            left: -1,
            top: 7,
            width: 1,
            height: 14,
            backgroundColor: theme.borderSubtle,
          }}
        />
      ) : null}
      {tab.document?.ok || tab.document === null ? (
        <ChromeIcon name="file-text" size={14} color={theme.mutedForeground} />
      ) : (
        <Icon name="alert-circle" size={14} color={theme.destructive} />
      )}
      <Label
        size={scale.controlFont}
        color={active ? theme.foreground : theme.mutedForeground}
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
          width: 24,
          height: 24,
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          borderRadius: 4,
          flexShrink: 0,
          cursor: 'pointer',
          opacity: active ? 0.5 : 0,
          hover: { opacity: 0.9, backgroundColor: theme.muted },
        }}
      >
        <Icon name="x" size={12} color={theme.mutedForeground} />
      </div>
    </div>
  )
}

export function Breadcrumb({ tab }: { tab: Tab }) {
  const { theme, scale } = useUi()
  const wideMode = useApp((state) => state.prefs.wideMode)
  const title = tab.document?.ok ? tab.document.parsed.title : null
  const segments = breadcrumbSegments(tab.path)
  const parents = segments.slice(0, -1)
  const file = segments.at(-1)
  const textSize = scale.controlXsFont + 1
  return (
    <div
      style={{
        display: 'flex',
        alignItems: 'center',
        height: METRICS.breadcrumbHeight,
        paddingLeft: 12,
        paddingRight: 12,
        gap: 8,
        borderBottomWidth: 1,
        borderColor: theme.borderSubtle,
        backgroundColor: theme.background,
        flexShrink: 0,
        userSelect: 'none',
      }}
    >
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: 2,
          flexGrow: 1,
          minWidth: 0,
          overflow: 'hidden',
        }}
      >
        {parents.map((segment) => (
          <div
            key={segment.path}
            // Folder names give way before the file name in a narrow window.
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: 2,
              flexShrink: 1,
              minWidth: 0,
              overflow: 'hidden',
            }}
          >
            <Crumb
              label={segment.name}
              path={segment.path}
              size={textSize}
              color={theme.mutedForeground}
            />
            <Icon name="chevron-right" size={10} color={`${theme.mutedForeground}`} />
          </div>
        ))}
        {file ? (
          <Crumb
            label={title ?? file.name}
            path={file.path}
            size={textSize}
            color={theme.foreground}
            weight={500}
          />
        ) : null}
      </div>
      {documentKind(tab.path) === 'html' ? (
        <IconButton
          icon="external-link"
          label="Open in browser"
          testId="open-in-browser"
          size={20}
          onClick={() => void openExternal(tab.path)}
        />
      ) : null}
      <div
        role="button"
        aria-label={wideMode ? 'Exit wide mode' : 'Wide mode'}
        aria-pressed={wideMode}
        tabIndex={0}
        onClick={() => setPrefs({ wideMode: !wideMode })}
        onKeyDown={activateOnEnter(() => setPrefs({ wideMode: !wideMode }))}
        style={{
          width: 20,
          height: 20,
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          borderRadius: 6,
          cursor: 'pointer',
          hover: { backgroundColor: theme.muted },
        }}
      >
        <ChromeIcon
          name={wideMode ? 'fold-horizontal' : 'arrow-left-right'}
          size={12}
          color={theme.mutedForeground}
        />
      </div>
    </div>
  )
}

function Crumb({
  label,
  path,
  size,
  color,
  weight,
}: {
  label: string
  path: string
  size: number
  color: string
  weight?: number
}) {
  const { theme } = useUi()
  return (
    <div
      role="button"
      tabIndex={0}
      onClick={() => void revealInFileManager(path)}
      onKeyDown={activateOnEnter(() => void revealInFileManager(path))}
      style={{
        paddingLeft: 2,
        paddingRight: 2,
        paddingTop: 1,
        paddingBottom: 1,
        borderRadius: 4,
        cursor: 'pointer',
        minWidth: 0,
        hover: { backgroundColor: theme.muted },
      }}
    >
      <Label
        size={size}
        color={color}
        weight={weight}
        style={{ whiteSpace: 'nowrap', textOverflow: 'ellipsis' }}
      >
        {label}
      </Label>
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

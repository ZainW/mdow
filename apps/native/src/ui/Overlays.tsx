import { basename, dirname } from 'node:path'
import { useMemo, useState, type ReactNode } from 'react'
import {
  COMMANDS,
  displayKeys,
  subsequenceScore,
  type CommandId,
  type CommandSpec,
} from '../lib/commands'
import {
  CODE_FONTS,
  COLUMN_WIDTHS,
  CONTENT_FONTS,
  DEFAULT_PREFS,
  INTERFACE_SCALES,
  LABELS,
  THEME_MODES,
} from '../lib/prefs'
import { workspaceFiles } from '../lib/workspace'
import { IS_MAC } from '../lib/platform'
import { openDocument, setOverlay, setPrefs, useApp, zoomBy, type UpdateStatus } from '../store'
import { UI_FONT, useUi } from './context'
import { activateOnEnter, Button, Icon, IconButton, Kbd, Label, Segmented } from './primitives'

function Modal({
  children,
  width,
  top,
  testId,
}: {
  children: ReactNode
  width: number
  top?: number
  testId?: string
}) {
  const { theme } = useUi()
  return (
    <div
      style={{
        position: 'absolute',
        top: 0,
        left: 0,
        right: 0,
        bottom: 0,
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        justifyContent: top === undefined ? 'center' : 'flex-start',
        paddingTop: top ?? 0,
        backgroundColor: theme.scheme === 'dark' ? '#00000080' : '#0000002e',
        pointerEvents: 'auto',
      }}
    >
      <div
        testId={testId}
        onMouseDownOutside={() => setOverlay(null)}
        style={{
          display: 'flex',
          flexDirection: 'column',
          width,
          maxHeight: '80%',
          borderRadius: 12,
          borderWidth: 1,
          borderColor: theme.border,
          backgroundColor: theme.surfaceRaised,
          overflow: 'hidden',
          boxShadow: {
            offsetX: 0,
            offsetY: 16,
            blurRadius: 48,
            spreadRadius: 0,
            color: '#00000040',
          },
        }}
      >
        {children}
      </div>
    </div>
  )
}

// ---------------------------------------------------------------------------
// Command palette
// ---------------------------------------------------------------------------

type PaletteItem =
  | { kind: 'command'; spec: CommandSpec }
  | { kind: 'file'; path: string; recent: boolean }

const MAX_RESULTS = 60

const sectionOf = (item: PaletteItem) => (item.kind === 'command' ? 'Actions' : 'Files')

export function paletteItems(query: string, recents: string[], files: string[]): PaletteItem[] {
  const seen = new Set<string>()
  const fileItems: PaletteItem[] = []
  for (const path of [...recents, ...files]) {
    if (seen.has(path)) continue
    seen.add(path)
    fileItems.push({ kind: 'file', path, recent: recents.includes(path) })
  }
  const commands: PaletteItem[] = COMMANDS.filter((spec) => spec.palette).map((spec) => ({
    kind: 'command',
    spec,
  }))
  if (!query.trim()) return [...commands, ...fileItems].slice(0, MAX_RESULTS)
  const scored: [number, PaletteItem][] = []
  for (const item of [...commands, ...fileItems]) {
    const label = item.kind === 'command' ? item.spec.title : basename(item.path)
    const score = subsequenceScore(query.trim(), label)
    if (score !== null) scored.push([score, item])
  }
  return scored
    .toSorted((a, b) => b[0] - a[0])
    .slice(0, MAX_RESULTS)
    .map(([, item]) => item)
}

export function CommandPalette({ onCommand }: { onCommand: (command: CommandId) => void }) {
  const { theme, scale } = useUi()
  const recents = useApp((state) => state.recents)
  const workspace = useApp((state) => state.workspace)
  const files = useMemo(
    () => (workspace?.scan.ok ? workspaceFiles(workspace.scan.root) : []),
    [workspace],
  )
  const [query, setQuery] = useState('')
  const [selected, setSelected] = useState(0)
  const items = useMemo(() => paletteItems(query, recents, files), [query, recents, files])
  const current = Math.min(selected, Math.max(0, items.length - 1))

  const invoke = (item: PaletteItem | undefined) => {
    if (!item) return
    setOverlay(null)
    if (item.kind === 'command') onCommand(item.spec.id)
    else openDocument(item.path)
  }

  return (
    <Modal width={560} top={72} testId="palette">
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: 10,
          paddingLeft: 16,
          paddingRight: 16,
          height: 52,
          borderBottomWidth: 1,
          borderColor: theme.border,
          flexShrink: 0,
        }}
      >
        <Icon name="search" />
        <input
          testId="palette-input"
          autoFocus
          value={query}
          placeholder="Type a command or file name"
          theme={{ caret: theme.primary }}
          onChange={(event) => {
            setQuery(event.value ?? '')
            setSelected(0)
          }}
          onSubmit={() => invoke(items[current])}
          onKeyDown={(event) => {
            if (event.key === 'down') setSelected(Math.min(items.length - 1, current + 1))
            else if (event.key === 'up') setSelected(Math.max(0, current - 1))
            else if (event.key === 'escape') setOverlay(null)
          }}
          style={{
            flexGrow: 1,
            minWidth: 0,
            height: 32,
            fontSize: scale.controlFont + 3,
            fontFamily: UI_FONT,
            color: theme.foreground,
          }}
        />
      </div>
      <div
        style={{
          display: 'flex',
          flexDirection: 'column',
          overflowY: 'scroll',
          padding: 6,
          minHeight: 0,
          flexShrink: 1,
        }}
      >
        {items.length === 0 ? (
          <div style={{ padding: 16 }}>
            <Label color={theme.mutedForeground}>No matches</Label>
          </div>
        ) : null}
        {items.map((item, index) => {
          const section = sectionOf(item)
          const previous = items[index - 1]
          const header = !previous || sectionOf(previous) !== section
          const active = index === current
          return (
            <div
              key={item.kind === 'command' ? item.spec.id : item.path}
              style={{ display: 'flex', flexDirection: 'column' }}
            >
              {header ? (
                <div
                  style={{ paddingLeft: 10, paddingTop: index === 0 ? 4 : 10, paddingBottom: 4 }}
                >
                  <Label size={scale.controlXsFont + 1} weight={600} color={theme.mutedForeground}>
                    {section.toUpperCase()}
                  </Label>
                </div>
              ) : null}
              <div
                role="option"
                aria-selected={active}
                tabIndex={-1}
                onClick={() => invoke(item)}
                onKeyDown={activateOnEnter(() => invoke(item))}
                onMouseEnter={() => setSelected(index)}
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: 10,
                  height: 34,
                  paddingLeft: 10,
                  paddingRight: 10,
                  borderRadius: 6,
                  cursor: 'pointer',
                  backgroundColor: active ? theme.sidebarAccent : undefined,
                  userSelect: 'none',
                }}
              >
                <Icon name={item.kind === 'command' ? 'command' : 'file'} size={14} />
                <Label
                  style={{
                    whiteSpace: 'nowrap',
                    textOverflow: 'ellipsis',
                    flexShrink: 1,
                    minWidth: 0,
                  }}
                >
                  {item.kind === 'command' ? item.spec.title : basename(item.path)}
                </Label>
                <div style={{ flexGrow: 1 }} />
                {item.kind === 'command' && item.spec.keys ? (
                  <Kbd>{displayKeys(item.spec.keys)}</Kbd>
                ) : null}
                {item.kind === 'file' ? (
                  <Label
                    size={scale.controlFont}
                    color={theme.mutedForeground}
                    style={{
                      whiteSpace: 'nowrap',
                      textOverflow: 'ellipsis',
                      flexShrink: 1,
                      minWidth: 0,
                    }}
                  >
                    {basename(dirname(item.path)) || (item.recent ? 'Recent' : '')}
                  </Label>
                ) : null}
              </div>
            </div>
          )
        })}
      </div>
    </Modal>
  )
}

// ---------------------------------------------------------------------------
// Settings
// ---------------------------------------------------------------------------

function SettingRow({ label, children }: { label: string; children: ReactNode }) {
  const { theme } = useUi()
  return (
    <div
      style={{
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between',
        gap: 16,
        paddingTop: 10,
        paddingBottom: 10,
        borderBottomWidth: 1,
        borderColor: theme.borderSubtle,
      }}
    >
      <Label>{label}</Label>
      {children}
    </div>
  )
}

function Section({ title, children }: { title: string; children: ReactNode }) {
  const { theme, scale } = useUi()
  return (
    <div style={{ display: 'flex', flexDirection: 'column', paddingTop: 14 }}>
      <Label size={scale.controlXsFont + 1} weight={600} color={theme.mutedForeground}>
        {title.toUpperCase()}
      </Label>
      {children}
    </div>
  )
}

export function Settings({ onCheckForUpdates }: { onCheckForUpdates: () => void }) {
  const { theme, scale } = useUi()
  const prefs = useApp((state) => state.prefs)
  const update = useApp((state) => state.update)
  return (
    <Modal width={520} testId="settings">
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          paddingLeft: 20,
          paddingRight: 10,
          height: 52,
          borderBottomWidth: 1,
          borderColor: theme.border,
          flexShrink: 0,
        }}
      >
        <Label size={scale.controlFont + 3} weight={600}>
          Settings
        </Label>
        <IconButton icon="x" label="Close settings" onClick={() => setOverlay(null)} />
      </div>
      <div
        style={{
          display: 'flex',
          flexDirection: 'column',
          overflowY: 'scroll',
          paddingLeft: 20,
          paddingRight: 20,
          paddingBottom: 20,
          minHeight: 0,
          flexShrink: 1,
        }}
      >
        <Section title="Appearance">
          <SettingRow label="Theme">
            <Segmented
              value={prefs.theme}
              options={THEME_MODES}
              labels={LABELS.theme}
              onChange={(theme) => setPrefs({ theme })}
            />
          </SettingRow>
          <SettingRow label="Interface size">
            <Segmented
              value={prefs.interfaceScale}
              options={INTERFACE_SCALES}
              labels={LABELS.interfaceScale}
              onChange={(interfaceScale) => setPrefs({ interfaceScale })}
            />
          </SettingRow>
        </Section>
        <Section title="Reading">
          <SettingRow label="Text font">
            <Segmented
              value={prefs.contentFont}
              options={CONTENT_FONTS}
              labels={LABELS.contentFont}
              onChange={(contentFont) => setPrefs({ contentFont })}
            />
          </SettingRow>
          <SettingRow label="Code font">
            <Segmented
              value={prefs.codeFont}
              options={CODE_FONTS}
              labels={LABELS.codeFont}
              onChange={(codeFont) => setPrefs({ codeFont })}
            />
          </SettingRow>
          <SettingRow label="Reading width">
            <Segmented
              value={prefs.readingWidth}
              options={COLUMN_WIDTHS}
              labels={LABELS.readingWidth}
              onChange={(readingWidth) => setPrefs({ readingWidth, wideMode: false })}
            />
          </SettingRow>
          <SettingRow label="Zoom">
            <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
              <Button small onClick={() => zoomBy(-1)}>
                −
              </Button>
              <div
                role="button"
                tabIndex={0}
                onClick={() => setPrefs({ zoomLevel: 100 })}
                onKeyDown={activateOnEnter(() => setPrefs({ zoomLevel: 100 }))}
                style={{ width: 52, display: 'flex', justifyContent: 'center', cursor: 'pointer' }}
              >
                <Label>{`${prefs.zoomLevel}%`}</Label>
              </div>
              <Button small onClick={() => zoomBy(1)}>
                +
              </Button>
            </div>
          </SettingRow>
        </Section>
        {IS_MAC ? (
          <Section title="Updates">
            <SettingRow label={updateLabel(update)}>
              <Button small onClick={onCheckForUpdates}>
                Check Now
              </Button>
            </SettingRow>
          </Section>
        ) : null}
        <div style={{ display: 'flex', paddingTop: 20 }}>
          <Button small variant="destructive" onClick={() => setPrefs({ ...DEFAULT_PREFS })}>
            Reset all settings
          </Button>
        </div>
      </div>
    </Modal>
  )
}

export function updateLabel(update: UpdateStatus) {
  switch (update.state) {
    case 'idle':
      return 'Mdow checks for updates daily'
    case 'checking':
      return 'Checking for updates…'
    case 'available':
      return `Version ${update.version} is available`
    case 'downloading':
      return `Downloading ${update.version}…`
    case 'ready':
      return `Version ${update.version} is ready — restart to finish`
    case 'up-to-date':
      return 'Mdow is up to date'
    case 'failed':
      return "Couldn't check for updates"
    case 'unavailable':
      return 'Updates are unavailable in this build'
  }
}

// ---------------------------------------------------------------------------
// Keyboard shortcuts
// ---------------------------------------------------------------------------

export function Shortcuts() {
  const { theme, scale } = useUi()
  return (
    <Modal width={460} testId="shortcuts">
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          paddingLeft: 20,
          paddingRight: 10,
          height: 52,
          borderBottomWidth: 1,
          borderColor: theme.border,
          flexShrink: 0,
        }}
      >
        <Label size={scale.controlFont + 3} weight={600}>
          Keyboard Shortcuts
        </Label>
        <IconButton icon="x" label="Close" onClick={() => setOverlay(null)} />
      </div>
      <div
        style={{
          display: 'flex',
          flexDirection: 'column',
          overflowY: 'scroll',
          paddingLeft: 20,
          paddingRight: 20,
          paddingTop: 8,
          paddingBottom: 16,
          minHeight: 0,
          flexShrink: 1,
        }}
      >
        {COMMANDS.filter((spec) => spec.keys).map((spec) => (
          <div
            key={spec.id}
            style={{
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'space-between',
              paddingTop: 7,
              paddingBottom: 7,
            }}
          >
            <Label color={theme.mutedForeground}>{spec.title}</Label>
            <Kbd>{displayKeys(spec.keys!)}</Kbd>
          </div>
        ))}
      </div>
    </Modal>
  )
}

import { basename, dirname } from 'node:path'
import { useMemo, useState, type ReactNode } from 'react'
import {
  COMMAND_BY_ID,
  COMMANDS,
  displayKeys,
  subsequenceScore,
  type CommandId,
  type CommandSpec,
} from '../lib/commands'
import {
  CODE_FONT_FAMILY,
  CODE_FONTS,
  COLUMN_WIDTHS,
  CONTENT_FONT_FAMILY,
  CONTENT_FONTS,
  DEFAULT_PREFS,
  INTERFACE_SCALES,
  LABELS,
  READER_FONT_SIZE,
  THEME_MODES,
  type ThemeMode,
} from '../lib/prefs'
import type { Theme } from '../lib/theme'
import { workspaceFiles } from '../lib/workspace'
import { useEnter } from '../lib/motion'
import { IS_MAC } from '../lib/platform'
import {
  openDocumentAsync,
  setOverlay,
  setPrefs,
  useApp,
  zoomBy,
  type UpdateStatus,
} from '../store'
import { UI_FONT, useUi } from './context'
import type { IconName } from './icons'
import { activateOnEnter, Button, Icon, Kbd, Label, withAlpha } from './primitives'

/** Desktop `--popover`: a touch darker than the page's raised surfaces in dark mode. */
function popoverColor(theme: Theme) {
  return theme.scheme === 'dark' ? theme.surfaceWell : theme.surfaceRaised
}

/**
 * The desktop dialog shell: a 12px-radius popover with a 1px `foreground/10` ring over a
 * `black/60` (dark) or `black/30` (light) scrim.
 *
 * `top` pins the panel that far down the window (the palette sits at 20%); otherwise it is centred
 * with 24px of breathing room, so tall dialogs stop at the window height minus 3rem and scroll.
 * `animate` fades the scrim and lifts the panel a few pixels. The palette leaves it off: it is
 * opened from the keyboard many times a day and must feel instant.
 */
function Modal({
  children,
  width,
  top,
  testId,
  animate = false,
}: {
  children: ReactNode
  width: number
  top?: `${number}%`
  testId?: string
  animate?: boolean
}) {
  const { theme } = useUi()
  const enter = useEnter(animate)
  return (
    <div
      style={{
        opacity: enter,
        position: 'absolute',
        top: 0,
        left: 0,
        right: 0,
        bottom: 0,
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        justifyContent: top === undefined ? 'center' : 'flex-start',
        paddingTop: top === undefined ? 24 : 0,
        paddingBottom: top === undefined ? 24 : 0,
        backgroundColor: theme.scheme === 'dark' ? '#00000099' : '#0000004d',
        pointerEvents: 'auto',
      }}
    >
      {top === undefined ? null : <div style={{ height: top, flexShrink: 0 }} />}
      <div
        style={{
          position: 'relative',
          top: 8 * (1 - enter),
          display: 'flex',
          maxHeight: '100%',
          minHeight: 0,
          flexShrink: 1,
        }}
      >
        <div
          testId={testId}
          onMouseDownOutside={() => setOverlay(null)}
          style={{
            display: 'flex',
            flexDirection: 'column',
            width,
            maxHeight: '100%',
            borderRadius: 12,
            borderWidth: 1,
            borderColor: withAlpha(theme.foreground, 0.1),
            backgroundColor: popoverColor(theme),
            overflow: 'hidden',
          }}
        >
          {children}
        </div>
      </div>
    </div>
  )
}

/** Title, description and the ghost close button every desktop dialog header carries. */
function DialogHeader({
  title,
  description,
  closeLabel,
}: {
  title: string
  description?: string
  closeLabel: string
}) {
  const { theme } = useUi()
  return (
    <div
      style={{
        position: 'relative',
        display: 'flex',
        flexDirection: 'column',
        gap: 4,
        flexShrink: 0,
      }}
    >
      <Label size={14} weight={500} style={{ lineHeight: 20 }}>
        {title}
      </Label>
      {description ? (
        <Label size={12} color={theme.mutedForeground} style={{ lineHeight: 19.5 }}>
          {description}
        </Label>
      ) : null}
      {/* Pinned 8px from the panel corner, out of the title's flow, as on desktop. */}
      <div
        role="button"
        aria-label={closeLabel}
        tabIndex={0}
        onClick={() => setOverlay(null)}
        onKeyDown={activateOnEnter(() => setOverlay(null))}
        style={{
          position: 'absolute',
          top: -8,
          right: -8,
          width: 24,
          height: 24,
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          borderRadius: 6,
          cursor: 'pointer',
          hover: { backgroundColor: theme.muted },
        }}
      >
        <Icon name="x" size={12} color={theme.foreground} />
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

/** Palette dressing for each command: the desktop's icon, label and one-line hint. */
const PALETTE_META: Partial<Record<CommandId, { icon: IconName; hint: string; label?: string }>> = {
  'open-file': { icon: 'file-plus', hint: 'Choose a markdown or HTML document' },
  'open-folder': { icon: 'folder-open', hint: 'Browse a folder of documents' },
  'close-tab': { icon: 'x', hint: 'Close the active document' },
  'toggle-sidebar': { icon: 'sidebar', hint: 'Show or hide navigation' },
  'sidebar-recents': { icon: 'clock', hint: 'Show recently opened documents' },
  'sidebar-folder': { icon: 'folder', hint: 'Show the open folder' },
  'sidebar-outline': { icon: 'list', hint: 'Show the document headings' },
  'toggle-wide-mode': { icon: 'arrow-left-right', hint: 'Switch reading width' },
  'column-standard': { icon: 'move-horizontal', hint: 'The narrowest reading column' },
  'column-comfortable': { icon: 'move-horizontal', hint: 'A little more room per line' },
  'column-wide': { icon: 'move-horizontal', hint: 'The widest reading column' },
  'theme-system': { icon: 'monitor', hint: 'Follow the system appearance' },
  'theme-light': { icon: 'sun', hint: 'Always use the light theme' },
  'theme-dark': { icon: 'moon', hint: 'Always use the dark theme' },
  'zoom-in': { icon: 'zoom-in', hint: 'Make the text larger' },
  'zoom-out': { icon: 'zoom-out', hint: 'Make the text smaller' },
  'zoom-reset': { icon: 'rotate-ccw', hint: 'Reset the text size' },
  find: { icon: 'file-search', hint: 'Search the active document', label: 'Find in Document' },
  settings: { icon: 'settings', hint: 'Theme, fonts, updates', label: 'Settings' },
  shortcuts: { icon: 'keyboard', hint: 'View available shortcuts' },
  'check-for-updates': {
    icon: 'refresh-cw',
    hint: 'Look for a newer version',
    label: 'Check for Updates',
  },
}

const commandLabel = (spec: CommandSpec) => PALETTE_META[spec.id]?.label ?? spec.title

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
    const label = item.kind === 'command' ? commandLabel(item.spec) : basename(item.path)
    const score = subsequenceScore(query.trim(), label)
    if (score !== null) scored.push([score, item])
  }
  return scored
    .toSorted((a, b) => b[0] - a[0])
    .slice(0, MAX_RESULTS)
    .map(([, item]) => item)
}

function FooterHint({ keys, children }: { keys: string; children: string }) {
  const { theme } = useUi()
  return (
    <div style={{ display: 'flex', alignItems: 'center', gap: 4 }}>
      <Kbd>{keys}</Kbd>
      <Label size={10} color={theme.mutedForeground}>
        {children}
      </Label>
    </div>
  )
}

export function CommandPalette({ onCommand }: { onCommand: (command: CommandId) => void }) {
  const { theme } = useUi()
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
    else void openDocumentAsync(item.path)
  }

  return (
    <Modal width={512} top="20%" testId="palette">
      <div style={{ display: 'flex', flexDirection: 'column', padding: 4, minHeight: 0 }}>
        <div style={{ display: 'flex', padding: 4, paddingBottom: 0, flexShrink: 0 }}>
          <div
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: 6,
              flexGrow: 1,
              height: 32,
              paddingLeft: 8,
              paddingRight: 8,
              borderRadius: 6,
              borderWidth: 1,
              borderColor: theme.border,
              backgroundColor: withAlpha(theme.border, theme.scheme === 'dark' ? 0.3 : 0.2),
            }}
          >
            <Icon name="search" size={14} color={withAlpha(theme.foreground, 0.5)} />
            <input
              testId="palette-input"
              autoFocus
              value={query}
              placeholder="Search files and commands…"
              theme={{ caret: theme.primary, textDim: theme.mutedForeground }}
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
                height: 30,
                fontSize: 12,
                fontFamily: UI_FONT,
                color: theme.foreground,
              }}
            />
          </div>
        </div>
        <div
          style={{
            display: 'flex',
            flexDirection: 'column',
            overflowY: 'scroll',
            padding: 4,
            maxHeight: 288,
            minHeight: 0,
            flexShrink: 1,
          }}
        >
          {items.length === 0 ? (
            <div
              style={{
                display: 'flex',
                justifyContent: 'center',
                paddingTop: 24,
                paddingBottom: 24,
              }}
            >
              <Label size={12}>No matching files or commands</Label>
            </div>
          ) : null}
          {items.map((item, index) => {
            const section = sectionOf(item)
            const previous = items[index - 1]
            const header = !previous || sectionOf(previous) !== section
            const active = index === current
            const meta = item.kind === 'command' ? PALETTE_META[item.spec.id] : undefined
            const detail =
              item.kind === 'command'
                ? (meta?.hint ?? '')
                : basename(dirname(item.path)) || (item.recent ? 'Recent' : '')
            return (
              <div
                key={item.kind === 'command' ? item.spec.id : item.path}
                style={{ display: 'flex', flexDirection: 'column' }}
              >
                {header ? (
                  <div
                    style={{
                      display: 'flex',
                      alignItems: 'center',
                      height: 28,
                      paddingLeft: 10,
                      paddingRight: 10,
                      marginTop: index === 0 ? 0 : 4,
                      flexShrink: 0,
                    }}
                  >
                    <Label size={12} weight={500} color={theme.mutedForeground}>
                      {section}
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
                    gap: 8,
                    height: 32,
                    paddingLeft: 10,
                    paddingRight: 10,
                    borderRadius: 6,
                    flexShrink: 0,
                    cursor: 'pointer',
                    backgroundColor: active ? theme.muted : undefined,
                    userSelect: 'none',
                  }}
                >
                  <Icon
                    name={item.kind === 'command' ? (meta?.icon ?? 'command') : 'file-text'}
                    size={14}
                    color={theme.foreground}
                  />
                  <Label
                    size={12}
                    style={{
                      whiteSpace: 'nowrap',
                      textOverflow: 'ellipsis',
                      flexShrink: 1,
                      minWidth: 0,
                    }}
                  >
                    {item.kind === 'command' ? commandLabel(item.spec) : basename(item.path)}
                  </Label>
                  <div style={{ flexGrow: 1 }} />
                  {detail ? (
                    <Label
                      size={item.kind === 'command' ? 10 : 11}
                      color={
                        item.kind === 'command' && active ? theme.foreground : theme.mutedForeground
                      }
                      style={{
                        whiteSpace: 'nowrap',
                        textOverflow: 'ellipsis',
                        flexShrink: 1,
                        minWidth: 0,
                      }}
                    >
                      {detail}
                    </Label>
                  ) : null}
                </div>
              </div>
            )
          })}
        </div>
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'flex-end',
            gap: 12,
            paddingLeft: 12,
            paddingRight: 12,
            paddingTop: 6,
            paddingBottom: 6,
            borderTopWidth: 1,
            borderColor: theme.borderSubtle,
            backgroundColor: withAlpha(theme.muted, 0.4),
            flexShrink: 0,
          }}
        >
          <FooterHint keys="↵">run/open</FooterHint>
          <FooterHint keys="esc">dismiss</FooterHint>
        </div>
      </div>
    </Modal>
  )
}

// ---------------------------------------------------------------------------
// Settings
// ---------------------------------------------------------------------------

/** A label stacked over its control, the desktop settings rhythm. */
function Field({ label, children }: { label: string; children: ReactNode }) {
  const { theme, scale } = useUi()
  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 6, flexShrink: 0 }}>
      <Label
        size={scale.controlFont}
        weight={500}
        color={theme.mutedForeground}
        style={{ lineHeight: scale.controlFont }}
      >
        {label}
      </Label>
      {children}
    </div>
  )
}

/**
 * The desktop's full-width outline toggle group: equal cells on a muted track, the pressed one
 * lifted onto the input colour with a faint ring.
 */
function PresetGroup<T extends string>({
  value,
  options,
  labels,
  icons,
  onChange,
}: {
  value: T
  options: readonly T[]
  labels: Record<T, string>
  icons?: Record<T, IconName>
  onChange: (value: T) => void
}) {
  const { theme, scale } = useUi()
  return (
    <div
      role="radiogroup"
      style={{ display: 'flex', padding: 2, borderRadius: 6, backgroundColor: theme.muted }}
    >
      {options.map((option, index) => {
        const pressed = option === value
        const first = index === 0
        const last = index === options.length - 1
        return (
          <div
            key={option}
            role="radio"
            aria-checked={pressed}
            aria-label={labels[option]}
            tabIndex={0}
            onClick={() => onChange(option)}
            onKeyDown={activateOnEnter(() => onChange(option))}
            style={{
              flexGrow: 1,
              flexBasis: 0,
              minWidth: 0,
              height: scale.buttonHeight,
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              gap: 6,
              borderWidth: 1,
              borderLeftWidth: first || pressed ? 1 : 0,
              borderColor: pressed ? withAlpha(theme.foreground, 0.15) : theme.border,
              borderTopLeftRadius: pressed || first ? 5 : 0,
              borderBottomLeftRadius: pressed || first ? 5 : 0,
              borderTopRightRadius: pressed || last ? 5 : 0,
              borderBottomRightRadius: pressed || last ? 5 : 0,
              backgroundColor: pressed
                ? theme.scheme === 'dark'
                  ? theme.border
                  : theme.background
                : undefined,
              boxShadow: pressed
                ? { offsetX: 0, offsetY: 1, blurRadius: 2, spreadRadius: 0, color: '#0000000d' }
                : undefined,
              hover: pressed ? undefined : { backgroundColor: theme.sidebarAccent },
              cursor: 'pointer',
              userSelect: 'none',
            }}
          >
            {icons ? (
              <Icon name={icons[option]} size={14} color={theme.foreground} filled={pressed} />
            ) : null}
            <Label size={scale.controlFont} weight={500}>
              {labels[option]}
            </Label>
          </div>
        )
      })}
    </div>
  )
}

/** A font choice: a sample glyph in that face over its name; the chosen one is ringed and dotted. */
function FontTile({
  active,
  label,
  family,
  glyph,
  glyphSize,
  onClick,
}: {
  active: boolean
  label: string
  family: string
  glyph: string
  glyphSize: number
  onClick: () => void
}) {
  const { theme } = useUi()
  const dark = theme.scheme === 'dark'
  // The ring is a wrapper border, not a spread shadow: gpuix paints shadows under the fill, so
  // one would show through the tile's translucent active wash.
  return (
    <div
      role="radio"
      aria-checked={active}
      aria-label={label}
      tabIndex={0}
      onClick={onClick}
      onKeyDown={activateOnEnter(onClick)}
      style={{
        flexGrow: 1,
        flexBasis: 0,
        minWidth: 0,
        display: 'flex',
        borderRadius: 7,
        borderWidth: 1,
        borderColor: active ? withAlpha(theme.foreground, dark ? 0.2 : 0.1) : '#00000000',
        cursor: 'pointer',
        userSelect: 'none',
      }}
    >
      <div
        style={{
          position: 'relative',
          flexGrow: 1,
          minWidth: 0,
          display: 'flex',
          flexDirection: 'column',
          alignItems: 'center',
          justifyContent: 'center',
          gap: 6,
          paddingTop: 9,
          paddingBottom: 9,
          paddingLeft: 7,
          paddingRight: 7,
          borderRadius: 6,
          borderWidth: 1,
          borderColor: active ? withAlpha(theme.foreground, dark ? 0.3 : 0.25) : theme.borderSubtle,
          backgroundColor: active
            ? withAlpha(theme.foreground, dark ? 0.06 : 0.04)
            : theme.background,
          hover: active
            ? undefined
            : { borderColor: theme.border, backgroundColor: withAlpha(theme.muted, 0.6) },
        }}
      >
        {active ? (
          <div
            style={{
              position: 'absolute',
              top: 6,
              right: 6,
              width: 6,
              height: 6,
              borderRadius: 3,
              backgroundColor: theme.primary,
            }}
          />
        ) : null}
        <text
          style={{
            fontFamily: family,
            fontSize: glyphSize,
            lineHeight: glyphSize,
            whiteSpace: 'nowrap',
            color: active ? theme.foreground : withAlpha(theme.foreground, 0.7),
          }}
        >
          {glyph}
        </text>
        <Label
          size={10}
          weight={500}
          color={active ? theme.foreground : theme.mutedForeground}
          style={{ lineHeight: 10, whiteSpace: 'nowrap' }}
        >
          {label}
        </Label>
      </div>
    </div>
  )
}

/** Decorative sample paragraph set in the chosen reading and code fonts. */
function FontPreview() {
  const { theme, reader } = useUi()
  const size = READER_FONT_SIZE
  const line = Math.round(size * 1.65 * 10) / 10
  const body = withAlpha(theme.foreground, 0.85)
  return (
    <div
      style={{
        display: 'flex',
        flexDirection: 'column',
        gap: 6,
        paddingLeft: 16,
        paddingRight: 16,
        paddingTop: 14,
        paddingBottom: 14,
        borderRadius: 8,
        borderWidth: 1,
        borderColor: theme.borderSubtle,
        backgroundColor: withAlpha(theme.muted, 0.4),
        flexShrink: 0,
        userSelect: 'none',
      }}
    >
      <text
        style={{
          fontFamily: reader.contentFont,
          fontSize: size * 1.25,
          lineHeight: Math.round(size * 1.25 * 1.25),
          fontWeight: 600,
          color: theme.foreground,
        }}
      >
        The quiet morning
      </text>
      <div style={{ display: 'flex', flexWrap: 'wrap', alignItems: 'center', columnGap: 4 }}>
        <text
          style={{ fontFamily: reader.contentFont, fontSize: size, lineHeight: line, color: body }}
        >
          Words on the page settle into their rhythm, and
        </text>
        <div
          style={{
            display: 'flex',
            paddingLeft: 4,
            paddingRight: 4,
            borderRadius: 4,
            backgroundColor: theme.muted,
          }}
        >
          <text
            style={{
              fontFamily: reader.codeFont,
              fontSize: Math.round(size * 0.9 * 10) / 10,
              lineHeight: Math.round(size * 1.2),
              color: theme.foreground,
            }}
          >
            ligatures
          </text>
        </div>
        <text
          style={{ fontFamily: reader.contentFont, fontSize: size, lineHeight: line, color: body }}
        >
          too.
        </text>
      </div>
    </div>
  )
}

const THEME_ICONS: Record<ThemeMode, IconName> = { system: 'monitor', light: 'sun', dark: 'moon' }

export function Settings({ onCheckForUpdates }: { onCheckForUpdates: () => void }) {
  const { theme, scale } = useUi()
  const prefs = useApp((state) => state.prefs)
  const update = useApp((state) => state.update)
  const zoomCell = {
    flexGrow: 1,
    flexBasis: 0,
    height: scale.buttonHeight,
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    borderWidth: 1,
    borderColor: theme.border,
    cursor: 'pointer',
    userSelect: 'none',
    hover: { backgroundColor: theme.sidebarAccent },
  } as const
  return (
    <Modal width={448} testId="settings" animate>
      <div
        style={{
          display: 'flex',
          flexDirection: 'column',
          gap: 20,
          padding: 16,
          overflowY: 'scroll',
          minHeight: 0,
          flexShrink: 1,
        }}
      >
        <DialogHeader
          title="Settings"
          description="Tune how markdown reads."
          closeLabel="Close settings"
        />
        <FontPreview />
        <Field label="Theme">
          <PresetGroup
            value={prefs.theme}
            options={THEME_MODES}
            labels={LABELS.theme}
            icons={THEME_ICONS}
            onChange={(theme) => setPrefs({ theme })}
          />
        </Field>
        <Field label="Interface scale">
          <PresetGroup
            value={prefs.interfaceScale}
            options={INTERFACE_SCALES}
            labels={LABELS.interfaceScale}
            onChange={(interfaceScale) => setPrefs({ interfaceScale })}
          />
        </Field>
        <Field label="Reading width">
          <PresetGroup
            value={prefs.readingWidth}
            options={COLUMN_WIDTHS}
            labels={LABELS.readingWidth}
            onChange={(readingWidth) => setPrefs({ readingWidth, wideMode: false })}
          />
        </Field>
        <Field label="Content font">
          <div style={{ display: 'flex', gap: 6 }}>
            {CONTENT_FONTS.map((font) => (
              <FontTile
                key={font}
                active={prefs.contentFont === font}
                label={LABELS.contentFont[font]}
                family={CONTENT_FONT_FAMILY[font]}
                glyph="Aa"
                glyphSize={18}
                onClick={() => setPrefs({ contentFont: font })}
              />
            ))}
          </div>
        </Field>
        <Field label="Code font">
          <div style={{ display: 'flex', gap: 6 }}>
            {CODE_FONTS.map((font) => (
              <FontTile
                key={font}
                active={prefs.codeFont === font}
                label={LABELS.codeFont[font]}
                family={CODE_FONT_FAMILY[font]}
                glyph="() => {}"
                glyphSize={13}
                onClick={() => setPrefs({ codeFont: font })}
              />
            ))}
          </div>
        </Field>
        <Field label="Zoom">
          <div
            style={{ display: 'flex', padding: 2, borderRadius: 6, backgroundColor: theme.muted }}
          >
            <div
              role="button"
              aria-label="Zoom out"
              tabIndex={0}
              onClick={() => zoomBy(-1)}
              onKeyDown={activateOnEnter(() => zoomBy(-1))}
              style={{ ...zoomCell, borderTopLeftRadius: 5, borderBottomLeftRadius: 5 }}
            >
              <Icon name="zoom-out" size={14} color={theme.foreground} />
            </div>
            <div
              role="button"
              aria-label="Reset zoom"
              tabIndex={0}
              onClick={() => setPrefs({ zoomLevel: 100 })}
              onKeyDown={activateOnEnter(() => setPrefs({ zoomLevel: 100 }))}
              style={{ ...zoomCell, borderLeftWidth: 0, borderRightWidth: 0 }}
            >
              <Label size={scale.controlFont} weight={500}>{`${prefs.zoomLevel}%`}</Label>
            </div>
            <div
              role="button"
              aria-label="Zoom in"
              tabIndex={0}
              onClick={() => zoomBy(1)}
              onKeyDown={activateOnEnter(() => zoomBy(1))}
              style={{ ...zoomCell, borderTopRightRadius: 5, borderBottomRightRadius: 5 }}
            >
              <Icon name="zoom-in" size={14} color={theme.foreground} />
            </div>
          </div>
        </Field>
        {IS_MAC ? (
          <div style={{ display: 'flex', flexDirection: 'column', gap: 8, flexShrink: 0 }}>
            <Label size={14} weight={500}>
              Updates
            </Label>
            <div
              style={{
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'space-between',
                gap: 12,
              }}
            >
              <Label size={14} color={theme.mutedForeground} style={{ flexShrink: 1, minWidth: 0 }}>
                {updateLabel(update)}
              </Label>
              <Button small onClick={onCheckForUpdates}>
                Check Now
              </Button>
            </div>
          </div>
        ) : null}
        <div
          style={{
            display: 'flex',
            justifyContent: 'flex-end',
            paddingTop: 12,
            borderTopWidth: 1,
            borderColor: theme.borderSubtle,
            flexShrink: 0,
          }}
        >
          <Button small onClick={() => setPrefs({ ...DEFAULT_PREFS })}>
            Reset to defaults
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

/** The desktop dialog's sections and sentence-case labels, over native's own bindings. */
const SHORTCUT_GROUPS: { heading: string; items: [CommandId, string][] }[] = [
  {
    heading: 'Navigation',
    items: [
      ['command-palette', 'Command palette'],
      ['find', 'Find in document'],
      ['find-next', 'Find next'],
      ['find-previous', 'Find previous'],
      ['next-tab', 'Next tab'],
      ['previous-tab', 'Previous tab'],
      ['close-tab', 'Close tab'],
    ],
  },
  {
    heading: 'View',
    items: [
      ['toggle-sidebar', 'Toggle sidebar'],
      ['sidebar-recents', 'Show recents'],
      ['sidebar-folder', 'Show folder'],
      ['sidebar-outline', 'Show outline'],
      ['toggle-wide-mode', 'Toggle wide mode'],
      ['zoom-in', 'Zoom in'],
      ['zoom-out', 'Zoom out'],
      ['zoom-reset', 'Reset zoom'],
      ['shortcuts', 'All shortcuts'],
    ],
  },
  {
    heading: 'Files',
    items: [
      ['open-file', 'Open file'],
      ['open-folder', 'Open folder'],
    ],
  },
  { heading: 'App', items: [['settings', 'Settings']] },
]

const MODIFIER_ORDER = ['⌃', '⌘', '⌥', '⇧']
const KEY_GLYPHS: Record<string, string> = { '=': '+', '-': '−' }

/** One chip per key, modifiers first in the desktop's ⌃ ⌘ ⌥ ⇧ order. */
export function keyChips(keys: string): string[] {
  const shown = displayKeys(keys)
  if (shown.includes('+') && shown.length > 1 && !IS_MAC) {
    return shown.split(/\+(?=.)/).map((key) => KEY_GLYPHS[key] ?? key)
  }
  const modifiers = MODIFIER_ORDER.filter((modifier) => shown.includes(modifier))
  const rest = shown.replace(/[⌃⌘⌥⇧]/g, '')
  return [...modifiers, KEY_GLYPHS[rest] ?? rest]
}

export function Shortcuts() {
  const { theme } = useUi()
  return (
    <Modal width={384} testId="shortcuts" animate>
      <div
        style={{
          display: 'flex',
          flexDirection: 'column',
          gap: 16,
          paddingTop: 16,
          paddingBottom: 16,
          overflowY: 'scroll',
          minHeight: 0,
          flexShrink: 1,
        }}
      >
        <div
          style={{ display: 'flex', flexDirection: 'column', paddingLeft: 16, paddingRight: 16 }}
        >
          <DialogHeader title="Keyboard Shortcuts" closeLabel="Close" />
        </div>
        <div style={{ display: 'flex', flexDirection: 'column', flexShrink: 0 }}>
          {SHORTCUT_GROUPS.map((group, index) => (
            <div
              key={group.heading}
              style={{
                display: 'flex',
                flexDirection: 'column',
                marginTop: index > 0 ? 6 : 0,
                paddingTop: index > 0 ? 6 : 0,
                borderTopWidth: index > 0 ? 1 : 0,
                borderColor: theme.borderSubtle,
              }}
            >
              <div style={{ paddingLeft: 16, paddingRight: 16, paddingTop: 4, paddingBottom: 4 }}>
                <Label size={10} weight={500} color={theme.mutedForeground}>
                  {group.heading.toUpperCase()}
                </Label>
              </div>
              {group.items.map(([id, label]) => {
                const keys = COMMAND_BY_ID.get(id)?.keys
                if (!keys) return null
                return (
                  <div
                    key={id}
                    style={{
                      display: 'flex',
                      alignItems: 'center',
                      justifyContent: 'space-between',
                      height: 32,
                      paddingLeft: 16,
                      paddingRight: 16,
                    }}
                  >
                    <Label size={14} color={theme.mutedForeground}>
                      {label}
                    </Label>
                    <div style={{ display: 'flex', alignItems: 'center', gap: 4 }}>
                      {keyChips(keys).map((key) => (
                        <Kbd key={key}>{key}</Kbd>
                      ))}
                    </div>
                  </div>
                )
              })}
            </div>
          ))}
        </div>
      </div>
    </Modal>
  )
}

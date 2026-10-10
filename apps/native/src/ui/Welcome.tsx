import { basename } from 'node:path'
import { openDocumentAsync, useApp } from '../store'
import { UI_MONO, useUi } from './context'
import { ICONS } from './icons'
import { activateOnEnter, Button, Icon, Label, withAlpha } from './primitives'

const LOGO = `data:image/svg+xml;base64,${Buffer.from(ICONS['mdow-logo']).toString('base64')}`
const LOGO_SIZE = 48

export function Welcome({
  onOpenFile,
  onOpenFolder,
}: {
  onOpenFile: () => void
  onOpenFolder: () => void
}) {
  const { theme } = useUi()
  const recents = useApp((state) => state.recents).slice(0, 6)
  const split = recents.length > 0
  const align = split ? 'flex-start' : 'center'
  return (
    <div
      testId="welcome"
      style={{
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        justifyContent: 'center',
        flexGrow: 1,
        minHeight: 0,
      }}
    >
      {/* Desktop: max-w-3xl, px-10, gap-10; intro and recents split 21fr / 19fr. */}
      <div
        style={{
          display: 'flex',
          justifyContent: 'center',
          gap: 40,
          width: '100%',
          maxWidth: 768,
          paddingLeft: 40,
          paddingRight: 40,
        }}
      >
        <div
          style={{
            display: 'flex',
            flexDirection: 'column',
            alignItems: align,
            gap: 12,
            flexGrow: split ? 21 : 0,
            flexBasis: split ? 0 : undefined,
            minWidth: 0,
          }}
        >
          {/* <svg> tints every shape one colour; the full-colour logo needs <img>. */}
          <img
            src={LOGO}
            alt="Mdow"
            style={{
              width: LOGO_SIZE,
              height: LOGO_SIZE,
              borderRadius: LOGO_SIZE * 0.22,
              marginBottom: split ? 0 : 4,
              boxShadow: {
                offsetX: 0,
                offsetY: 0,
                blurRadius: 0,
                spreadRadius: 1,
                color: withAlpha(theme.border, 0.4),
              },
            }}
          />
          <Label size={24} weight={600} style={{ lineHeight: 32 }}>
            Mdow
          </Label>
          <Label
            size={14}
            color={theme.mutedForeground}
            style={{ textAlign: split ? 'left' : 'center', lineHeight: 22.75, maxWidth: 352 }}
          >
            A quiet markdown viewer. Drop a file anywhere, or open one below.
          </Label>
          <div style={{ display: 'flex', gap: 8, marginTop: 8 }}>
            <Button icon="file" small muted onClick={onOpenFile} testId="welcome-open-file">
              Open File
            </Button>
            <Button icon="folder-open" small muted onClick={onOpenFolder}>
              Open Folder
            </Button>
          </div>
          <DropHint />
        </div>
        {split ? <RecentColumn recents={recents} /> : null}
      </div>
    </div>
  )
}

/**
 * The card under the buttons: "Anywhere in this window, drop .md or .html files or a folder."
 * It lights up like the desktop's while a file is dragged over the window (macOS reports the
 * drag through the store's `dragging` flag); the drop itself lands anywhere.
 */
function DropHint() {
  const { theme, scale } = useUi()
  const dragging = useApp((state) => state.dragging)
  // gpuix text doesn't flow across styled runs, so each word is its own run and the row wraps
  // between them. A trailing no-break space keeps the gap without indenting a wrapped line.
  const words = (value: string, color: string, weight = 400) =>
    value.split(' ').map((word, index, all) =>
      word === '' ? null : (
        <Label
          key={`${value}:${word}`}
          size={scale.controlFont}
          weight={weight}
          color={color}
          style={{ whiteSpace: 'nowrap', lineHeight: 19.5 }}
        >
          {index < all.length - 1 ? `${word}\u00a0` : word}
        </Label>
      ),
    )
  const muted = theme.mutedForeground
  return (
    <div
      testId="drop-zone"
      style={{
        display: 'flex',
        flexWrap: 'wrap',
        alignItems: 'center',
        marginTop: 12,
        paddingLeft: 12,
        paddingRight: 12,
        paddingTop: 19,
        paddingBottom: 19,
        borderRadius: 8,
        borderWidth: 1,
        borderColor: dragging ? withAlpha(theme.primary, 0.3) : withAlpha(theme.border, 0.7),
        backgroundColor: dragging ? withAlpha(theme.primary, 0.05) : withAlpha(theme.muted, 0.2),
        maxWidth: 448,
        userSelect: 'none',
      }}
    >
      {/* One run, so the phrase reads (and is found) as a whole; it always fits the card. */}
      <Label
        size={scale.controlFont}
        weight={500}
        color={dragging ? theme.primary : withAlpha(theme.foreground, 0.9)}
        style={{ whiteSpace: 'nowrap', lineHeight: 19.5 }}
      >
        {dragging ? 'Release to open in Mdow' : 'Anywhere in this window'}
      </Label>
      {words(', drop ', muted)}
      <Chip>.md</Chip>
      {words('or ', muted)}
      <Chip>.html</Chip>
      {words('files or a folder.', muted)}
    </div>
  )
}

function Chip({ children }: { children: string }) {
  const { theme, scale } = useUi()
  return (
    <div
      style={{
        paddingLeft: 4,
        paddingRight: 4,
        paddingTop: 1,
        paddingBottom: 1,
        borderRadius: 2,
        backgroundColor: theme.muted,
        marginRight: 3.5,
      }}
    >
      <text
        style={{
          fontFamily: UI_MONO,
          fontSize: scale.controlFont - 1,
          color: withAlpha(theme.foreground, 0.8),
        }}
      >
        {children}
      </text>
    </div>
  )
}

function RecentColumn({ recents }: { recents: string[] }) {
  const { theme, scale } = useUi()
  return (
    <div
      style={{
        display: 'flex',
        flexDirection: 'column',
        gap: 8,
        flexGrow: 19,
        flexBasis: 0,
        minWidth: 0,
      }}
    >
      <text
        style={{
          fontFamily: UI_MONO,
          fontSize: scale.controlFont - 1,
          color: withAlpha(theme.mutedForeground, 0.9),
        }}
      >
        RECENT
      </text>
      <div style={{ display: 'flex', flexDirection: 'column', gap: 1 }}>
        {recents.map((path) => (
          <div
            key={path}
            role="button"
            tabIndex={0}
            onClick={() => void openDocumentAsync(path)}
            onKeyDown={activateOnEnter(() => void openDocumentAsync(path))}
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: 8,
              height: 32,
              paddingLeft: 8,
              paddingRight: 8,
              borderRadius: 6,
              cursor: 'pointer',
              hover: { backgroundColor: withAlpha(theme.muted, theme.scheme === 'dark' ? 0.5 : 1) },
              userSelect: 'none',
            }}
          >
            <Icon name="file-text" size={14} color={withAlpha(theme.mutedForeground, 0.6)} />
            <Label
              size={scale.controlFont}
              color={theme.mutedForeground}
              style={{ whiteSpace: 'nowrap', textOverflow: 'ellipsis', flexShrink: 1, minWidth: 0 }}
            >
              {basename(path)}
            </Label>
          </div>
        ))}
      </div>
    </div>
  )
}

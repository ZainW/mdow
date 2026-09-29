import { basename, dirname } from 'node:path'
import { openDocument, useApp } from '../store'
import { useUi } from './context'
import { ICONS } from './icons'
import { activateOnEnter, Button, Icon, Label } from './primitives'

const LOGO = `data:image/svg+xml;base64,${Buffer.from(ICONS['mdow-logo']).toString('base64')}`

export function Welcome({
  onOpenFile,
  onOpenFolder,
}: {
  onOpenFile: () => void
  onOpenFolder: () => void
}) {
  const { theme, scale } = useUi()
  const recents = useApp((state) => state.recents).slice(0, 5)
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
        padding: 32,
      }}
    >
      <div
        style={{
          display: 'flex',
          flexDirection: 'column',
          alignItems: 'center',
          gap: 10,
          width: 360,
        }}
      >
        {/* <svg> tints every shape one colour; the full-colour logo needs <img>. */}
        <img src={LOGO} alt="Mdow" style={{ width: 56, height: 56, marginBottom: 6 }} />
        <Label size={scale.controlFont + 12} weight={600}>
          Mdow
        </Label>
        <Label
          size={scale.controlFont + 2}
          color={theme.mutedForeground}
          style={{ textAlign: 'center' }}
        >
          A quiet markdown viewer. Drop a file anywhere, or open one below.
        </Label>
        <div style={{ display: 'flex', gap: 8, marginTop: 12 }}>
          <Button icon="file" variant="primary" onClick={onOpenFile} testId="welcome-open-file">
            Open File
          </Button>
          <Button icon="folder-open" onClick={onOpenFolder}>
            Open Folder
          </Button>
        </div>
        <DropZone />
        {recents.length > 0 ? (
          <div
            style={{
              display: 'flex',
              flexDirection: 'column',
              width: '100%',
              marginTop: 28,
              gap: 2,
            }}
          >
            <div style={{ paddingLeft: 10, paddingBottom: 6 }}>
              <Label size={scale.controlXsFont + 1} weight={600} color={theme.mutedForeground}>
                RECENT
              </Label>
            </div>
            {recents.map((path) => (
              <div
                key={path}
                role="button"
                tabIndex={0}
                onClick={() => openDocument(path)}
                onKeyDown={activateOnEnter(() => openDocument(path))}
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: 10,
                  paddingLeft: 10,
                  paddingRight: 10,
                  paddingTop: 7,
                  paddingBottom: 7,
                  borderRadius: 6,
                  cursor: 'pointer',
                  hover: { backgroundColor: theme.sidebarAccent },
                  userSelect: 'none',
                }}
              >
                <Icon name="file" size={14} />
                <Label
                  style={{
                    whiteSpace: 'nowrap',
                    textOverflow: 'ellipsis',
                    flexShrink: 1,
                    minWidth: 0,
                  }}
                >
                  {basename(path)}
                </Label>
                <div style={{ flexGrow: 1 }} />
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
                  {basename(dirname(path))}
                </Label>
              </div>
            ))}
          </div>
        ) : null}
      </div>
    </div>
  )
}

/** Lights up while a file is dragged over the window. The drop itself lands anywhere. */
function DropZone() {
  const { theme, scale } = useUi()
  const dragging = useApp((state) => state.dragging)
  return (
    <div
      testId="drop-zone"
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: 12,
        width: '100%',
        height: 76,
        marginTop: 24,
        paddingLeft: 16,
        paddingRight: 16,
        borderRadius: 8,
        borderWidth: 1,
        borderColor: dragging ? theme.primary : theme.border,
        backgroundColor: dragging ? theme.selection : undefined,
        userSelect: 'none',
      }}
    >
      <div
        style={{
          width: 32,
          height: 32,
          flexShrink: 0,
          borderRadius: 7,
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          backgroundColor: theme.muted,
        }}
      >
        <Icon name="file" size={17} color={dragging ? theme.primary : theme.mutedForeground} />
      </div>
      <div style={{ display: 'flex', flexDirection: 'column', gap: 3, minWidth: 0 }}>
        <Label size={scale.controlFont} weight={500}>
          {dragging ? 'Release to open in Mdow' : 'Anywhere in this window'}
        </Label>
        <div style={{ display: 'flex', alignItems: 'center' }}>
          <Label size={scale.controlXsFont + 1} color={theme.mutedForeground}>
            Drop
          </Label>
          <Label mono size={scale.controlXsFont + 1} style={{ paddingLeft: 3, paddingRight: 3 }}>
            .md
          </Label>
          <Label size={scale.controlXsFont + 1} color={theme.mutedForeground}>
            files or a folder
          </Label>
        </div>
      </div>
    </div>
  )
}

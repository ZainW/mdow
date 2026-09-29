import { dismissUpdate, useApp } from '../store'
import { bannerCopy, installUpdate, openReleases, restartToUpdate } from '../update-flow'
import { useUi } from './context'
import { Button, Icon, IconButton, Label } from './primitives'

export function UpdateBanner({ beforeRestart }: { beforeRestart: () => void }) {
  const { theme } = useUi()
  const update = useApp((state) => state.update)
  const dismissed = useApp((state) => state.updateDismissed)
  const copy = bannerCopy(update)
  if (!copy || dismissed) return null
  const action =
    update.state === 'available'
      ? { label: 'Download', run: () => void installUpdate() }
      : update.state === 'ready'
        ? { label: 'Restart', run: () => restartToUpdate(beforeRestart) }
        : update.state === 'unavailable' || update.state === 'failed'
          ? { label: 'Releases', run: openReleases }
          : null
  return (
    <div
      testId="update-banner"
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: 10,
        paddingLeft: 14,
        paddingRight: 6,
        height: 40,
        backgroundColor: theme.muted,
        borderBottomWidth: 1,
        borderColor: theme.border,
        flexShrink: 0,
      }}
    >
      <Icon name={update.state === 'failed' ? 'alert-circle' : 'check'} size={14} />
      <Label style={{ flexGrow: 1, minWidth: 0, whiteSpace: 'nowrap', textOverflow: 'ellipsis' }}>
        {copy}
      </Label>
      {action ? (
        <Button small onClick={action.run} testId="update-action">
          {action.label}
        </Button>
      ) : null}
      <IconButton icon="x" label="Dismiss" onClick={dismissUpdate} />
    </div>
  )
}

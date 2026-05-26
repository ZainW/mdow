import { Dialog, DialogContent, DialogHeader, DialogTitle } from './ui/dialog'
import { isMac } from '@renderer/lib/utils'

const mod = isMac ? '⌘' : 'Ctrl'

interface ShortcutGroup {
  heading: string
  items: { label: string; keys: string }[]
}

const groups: ShortcutGroup[] = [
  {
    heading: 'Files',
    items: [
      { label: 'Open file', keys: `${mod} O` },
      { label: 'Open folder', keys: `${mod} ⇧ O` },
    ],
  },
  {
    heading: 'Navigation',
    items: [
      { label: 'Command palette', keys: `${mod} K` },
      { label: 'Find in document', keys: `${mod} F` },
      { label: 'Switch tab (1–9)', keys: `${mod} 1–9` },
      { label: 'Close tab', keys: `${mod} W` },
    ],
  },
  {
    heading: 'View',
    items: [
      { label: 'Toggle sidebar', keys: `${mod} B` },
      { label: 'Keyboard shortcuts', keys: `${mod} /` },
      { label: 'Zoom in', keys: `${mod} +` },
      { label: 'Zoom out', keys: `${mod} -` },
      { label: 'Reset zoom', keys: `${mod} 0` },
      { label: 'Toggle full screen', keys: isMac ? '⌃ ⌘ F' : 'F11' },
    ],
  },
  {
    heading: 'App',
    items: [{ label: 'Settings', keys: `${mod} ,` }],
  },
]

interface ShortcutsDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
}

export function ShortcutsDialog({ open, onOpenChange }: ShortcutsDialogProps) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-sm">
        <DialogHeader>
          <DialogTitle>Keyboard Shortcuts</DialogTitle>
        </DialogHeader>
        <div className="-mx-4 flex flex-col">
          {groups.map((group, gi) => (
            <section
              key={group.heading}
              className={gi > 0 ? 'mt-1.5 border-t border-border-subtle pt-1.5' : undefined}
            >
              <h3 className="px-4 pt-1 pb-1 text-[10px] font-medium uppercase tracking-wider text-muted-foreground/70">
                {group.heading}
              </h3>
              <ul className="flex flex-col">
                {group.items.map((s) => (
                  <li key={s.label} className="flex items-center justify-between px-4 py-1.5">
                    <span className="text-sm text-muted-foreground">{s.label}</span>
                    <kbd className="rounded bg-muted px-2 py-0.5 font-mono text-[11px] text-muted-foreground">
                      {s.keys}
                    </kbd>
                  </li>
                ))}
              </ul>
            </section>
          ))}
        </div>
      </DialogContent>
    </Dialog>
  )
}

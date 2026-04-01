import { Dialog, DialogContent, DialogHeader, DialogTitle } from './ui/dialog'

const isMac = navigator.platform.includes('Mac')
const mod = isMac ? '⌘' : 'Ctrl'

const shortcuts = [
  { label: 'Open file', keys: `${mod} O` },
  { label: 'Open folder', keys: `${mod} ⇧ O` },
  { label: 'Command palette', keys: `${mod} K` },
  { label: 'Find in document', keys: `${mod} F` },
  { label: 'Toggle sidebar', keys: `${mod} B` },
  { label: 'Zoom in', keys: `${mod} +` },
  { label: 'Zoom out', keys: `${mod} -` },
  { label: 'Reset zoom', keys: `${mod} 0` },
  { label: 'Toggle full screen', keys: isMac ? '⌃ ⌘ F' : 'F11' },
  { label: 'Settings', keys: `${mod} ,` },
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
        <div className="-mx-4 divide-y divide-border">
          {shortcuts.map((s) => (
            <div key={s.label} className="flex items-center justify-between px-4 py-2">
              <span className="text-sm text-muted-foreground">{s.label}</span>
              <kbd className="rounded bg-muted px-2 py-0.5 font-mono text-xs text-muted-foreground">
                {s.keys}
              </kbd>
            </div>
          ))}
        </div>
      </DialogContent>
    </Dialog>
  )
}

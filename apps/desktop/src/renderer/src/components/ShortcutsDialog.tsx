import { Dialog, DialogContent, DialogHeader, DialogTitle } from './ui/dialog'
import { Kbd, KbdGroup } from './ui/kbd'
import { cheatSheetColumns } from '@renderer/lib/cheat-sheet'
import { isMac } from '@renderer/lib/utils'

// One source of truth with the hold-⌘ cheat sheet, so the two lists can't drift apart.
const groups = cheatSheetColumns(isMac).flatMap((column) => column.sections)

interface ShortcutsDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
}

export function ShortcutsDialog({ open, onOpenChange }: ShortcutsDialogProps) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-h-[calc(100dvh-3rem)] overflow-y-auto sm:max-w-sm">
        <DialogHeader>
          <DialogTitle>Keyboard Shortcuts</DialogTitle>
        </DialogHeader>
        <div className="-mx-4 flex flex-col">
          {groups.map((group, gi) => (
            <section
              key={group.heading}
              className={gi > 0 ? 'mt-1.5 border-t border-border-subtle pt-1.5' : undefined}
            >
              <h3 className="px-4 pt-1 pb-1 text-[10px] font-medium uppercase tracking-wider text-muted-foreground-subtle">
                {group.heading}
              </h3>
              <ul className="flex flex-col">
                {group.items.map((item) => (
                  <li key={item.label} className="flex items-center justify-between px-4 py-1.5">
                    <span className="text-sm text-muted-foreground">{item.label}</span>
                    <KbdGroup>
                      {item.keys.map((key) => (
                        <Kbd key={`${item.label}-${key}`}>{key}</Kbd>
                      ))}
                    </KbdGroup>
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

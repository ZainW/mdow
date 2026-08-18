import { cheatSheetColumns } from '@renderer/lib/cheat-sheet'
import { isMacPlatform } from '@renderer/lib/utils'
import { Kbd, KbdGroup } from './ui/kbd'
import { useCheatSheetHold } from '@renderer/hooks/useCheatSheetHold'

export function CheatSheetOverlay() {
  const open = useCheatSheetHold()
  if (!open) return null

  const isMac = isMacPlatform()
  const mod = isMac ? '⌘' : 'Ctrl'
  const columns = cheatSheetColumns(isMac)

  return (
    <dialog
      open
      aria-modal="false"
      aria-label="Keyboard cheat sheet"
      className="pointer-events-none fixed bottom-6 left-1/2 z-[45] m-0 w-[min(720px,calc(100%-2rem))] max-w-none -translate-x-1/2 rounded-xl border-0 bg-popover px-5 pt-4 pb-3 text-popover-foreground shadow-[0_12px_32px_oklch(0.13_0.02_50/0.1)] ring-1 ring-foreground/10 motion-safe:animate-in motion-safe:fade-in-0 motion-safe:slide-in-from-bottom-1 motion-safe:duration-100 dark:shadow-none"
    >
      <div className="grid grid-cols-[1.15fr_1fr_0.9fr] gap-x-7">
        {columns.map((column, index) => (
          <div
            key={column.sections.map((section) => section.heading).join('-')}
            className={index > 0 ? 'border-l border-border-subtle pl-7' : undefined}
          >
            {column.sections.map((section, sectionIndex) => (
              <section key={section.heading} className={sectionIndex > 0 ? 'mt-3.5' : undefined}>
                <h2 className="mb-2 text-[10px] font-medium tracking-wider text-muted-foreground/70 uppercase">
                  {section.heading}
                </h2>
                <ul>
                  {section.items.map((item) => (
                    <li
                      key={item.label}
                      className="flex h-[26px] items-center justify-between gap-3"
                    >
                      <span className="truncate text-[13px] text-muted-foreground">
                        {item.label}
                      </span>
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
        ))}
      </div>
      <div className="mt-3 flex items-center justify-between border-t border-border-subtle pt-2.5 text-[11px] text-muted-foreground/70">
        <p>
          Hold <span className="font-medium text-accent">{mod}</span> · release to dismiss
        </p>
        <p>{mod}/ for full list</p>
      </div>
    </dialog>
  )
}

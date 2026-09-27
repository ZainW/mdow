import { render, screen, within } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import { ShortcutsDialog } from './ShortcutsDialog'
import { cheatSheetColumns } from '@renderer/lib/cheat-sheet'
import { isMac } from '@renderer/lib/utils'

describe('ShortcutsDialog', () => {
  it('lists every shortcut from the cheat sheet data', () => {
    render(<ShortcutsDialog open onOpenChange={() => {}} />)
    const dialog = screen.getByRole('dialog', { name: 'Keyboard Shortcuts' })
    const sections = cheatSheetColumns(isMac).flatMap((column) => column.sections)

    for (const section of sections) {
      expect(within(dialog).getByRole('heading', { name: section.heading })).toBeInTheDocument()
      for (const item of section.items) {
        expect(within(dialog).getByText(item.label)).toBeInTheDocument()
      }
    }
  })

  it('includes tab cycling shortcuts that used to be missing', () => {
    render(<ShortcutsDialog open onOpenChange={() => {}} />)
    expect(screen.getByText('Next tab')).toBeInTheDocument()
    expect(screen.getByText('Previous tab')).toBeInTheDocument()
  })
})

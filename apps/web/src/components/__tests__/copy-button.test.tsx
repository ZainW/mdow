import { render, screen, fireEvent, waitFor } from '@testing-library/react'
import { describe, it, expect, vi } from 'vitest'
import { CopyButton } from '../copy-button'

describe('CopyButton', () => {
  it('copies value to the clipboard on click', async () => {
    const writeText = vi.fn().mockResolvedValue(undefined)
    Object.assign(navigator, { clipboard: { writeText } })
    render(<CopyButton value="hello" />)
    fireEvent.click(screen.getByRole('button'))
    await waitFor(() => expect(writeText).toHaveBeenCalledWith('hello'))
  })

  it('shows copied state briefly after click', async () => {
    Object.assign(navigator, {
      clipboard: { writeText: vi.fn().mockResolvedValue(undefined) },
    })
    render(<CopyButton value="x" />)
    fireEvent.click(screen.getByRole('button'))
    await waitFor(() => expect(screen.getByLabelText('Copied')).toBeInTheDocument())
  })
})

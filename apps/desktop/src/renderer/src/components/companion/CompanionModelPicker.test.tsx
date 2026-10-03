import { fireEvent, render, screen } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import type { CompanionModelState } from '../../../../shared/types'
import { CompanionModelPicker } from './CompanionModelPicker'

const modelState: CompanionModelState = {
  options: [
    {
      value: 'opencode/claude-sonnet-5-5',
      name: 'Claude Sonnet 5.5',
      providerId: 'opencode',
      providerName: 'OpenCode Zen',
    },
    {
      value: 'opencode-go/kimi-k2.5',
      name: 'Kimi K2.5',
      providerId: 'opencode-go',
      providerName: 'OpenCode Go',
    },
  ],
  currentValue: 'opencode/claude-sonnet-5-5',
  stale: false,
}

describe('CompanionModelPicker', () => {
  it('groups models by OpenCode provider and selects through Base UI', async () => {
    const onValueChange = vi.fn()
    render(<CompanionModelPicker state={modelState} onValueChange={onValueChange} />)

    fireEvent.click(screen.getByRole('combobox', { name: 'Model: Claude Sonnet 5.5' }))
    expect(await screen.findByText('OpenCode Zen')).toBeVisible()
    expect(screen.getByText('OpenCode Go')).toBeVisible()

    fireEvent.click(screen.getByText('Kimi K2.5'))
    expect(onValueChange).toHaveBeenCalledWith('opencode-go/kimi-k2.5')
  })

  it('disables selection while models are loading', () => {
    render(
      <CompanionModelPicker
        state={{ options: [], currentValue: null, stale: true, unavailableReason: 'Starting' }}
        onValueChange={vi.fn()}
      />,
    )

    expect(screen.getByRole('combobox', { name: 'Model: Loading models…' })).toBeDisabled()
  })
})

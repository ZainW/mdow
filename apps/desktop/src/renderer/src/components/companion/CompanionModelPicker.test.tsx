import { fireEvent, render, screen } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import type { CompanionModelState } from '../../../../shared/types'
import { CompanionModelPicker } from './CompanionModelPicker'

const modelState: CompanionModelState = {
  options: [
    { value: 'openai/gpt-5.4', name: 'GPT-5.4', provider: 'openai' },
    {
      value: 'opencode/claude-sonnet-4-5',
      name: 'Claude Sonnet 4.5',
      provider: 'opencode',
    },
    { value: 'opencode-go/kimi-k2.5', name: 'Kimi K2.5', provider: 'opencode-go' },
  ],
  currentValue: 'openai/gpt-5.4',
  stale: false,
}

describe('CompanionModelPicker', () => {
  it('groups only the live model options and selects through Base UI', async () => {
    const onValueChange = vi.fn()
    render(
      <CompanionModelPicker state={modelState} onValueChange={onValueChange} disabled={false} />,
    )

    fireEvent.click(screen.getByRole('combobox', { name: 'Model' }))
    expect(await screen.findByText('ChatGPT subscription')).toBeVisible()
    expect(screen.getByText('OpenCode Zen')).toBeVisible()
    expect(screen.getByText('OpenCode Go')).toBeVisible()

    fireEvent.click(screen.getByText('Kimi K2.5'))
    expect(onValueChange).toHaveBeenCalledWith('opencode-go/kimi-k2.5')
  })

  it('disables selection when the live session is stale', () => {
    render(
      <CompanionModelPicker
        state={{
          options: [],
          currentValue: null,
          stale: true,
          unavailableReason: 'Start Companion to load models',
        }}
        onValueChange={vi.fn()}
      />,
    )

    expect(screen.getByRole('combobox', { name: 'Model' })).toBeDisabled()
  })
})

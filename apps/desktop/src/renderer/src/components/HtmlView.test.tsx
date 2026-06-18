import { render, screen, waitFor } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { HtmlView } from './HtmlView'
import type { Tab } from '../store/app-store'
import { createMinimalWindowApi, stubWindowApi } from '../test/stubWindowApi'

const resolveLocalUrl = vi.fn()

stubWindowApi(() => createMinimalWindowApi({ resolveLocalUrl }))

const tab: Tab = {
  id: 'tab-html',
  path: '/tmp/preview.html',
  content: '<h1>Preview</h1>',
  scrollPosition: 0,
}

describe('HtmlView', () => {
  beforeEach(() => {
    resolveLocalUrl.mockReset()
    resolveLocalUrl.mockResolvedValue('mdow-local://local/%2Ftmp%2Fpreview.html')
  })

  it('renders html documents in a sandboxed iframe', async () => {
    render(<HtmlView tab={tab} />)

    await waitFor(() => {
      expect(resolveLocalUrl).toHaveBeenCalledWith('/tmp/preview.html')
    })

    const frame = await screen.findByTitle('preview.html preview')
    expect(frame).toHaveAttribute('sandbox', '')
    expect(frame).toHaveAttribute('src', expect.stringContaining('mdow-local://local/'))
    expect(frame).toHaveAttribute('src', expect.stringContaining('?v='))
  })

  it('shows a readable error when a local url cannot be resolved', async () => {
    resolveLocalUrl.mockResolvedValue('')

    render(<HtmlView tab={tab} />)

    expect(await screen.findByText('This HTML document could not be rendered.')).toBeInTheDocument()
  })
})

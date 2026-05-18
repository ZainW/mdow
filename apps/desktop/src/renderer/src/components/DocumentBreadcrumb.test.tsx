import { render, screen, fireEvent } from '@testing-library/react'
import { describe, expect, it, vi, beforeEach } from 'vitest'
import { DocumentBreadcrumb } from './DocumentBreadcrumb'
import { useAppStore, type Tab } from '../store/app-store'

const showInFolder = vi.fn()

beforeEach(() => {
  showInFolder.mockClear()
  // @ts-expect-error — minimal window.api stub for the test
  globalThis.window.api = {
    showInFolder,
  }
  useAppStore.setState({
    wideMode: false,
    openFolderPath: null,
  })
})

const tab: Tab = {
  id: 'tab-1',
  path: '/Users/zain/projects/mdow/notes/readme.md',
  content: '',
  scrollPosition: 0,
}

describe('DocumentBreadcrumb', () => {
  it('renders each parent segment as its own button', () => {
    render(<DocumentBreadcrumb tab={tab} />)
    // With no openFolderPath, shows the last 3 parent dirs as buttons.
    expect(screen.getByRole('button', { name: 'mdow' })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'notes' })).toBeInTheDocument()
  })

  it('calls window.api.showInFolder with the segment absolute path on click', () => {
    render(<DocumentBreadcrumb tab={tab} />)
    fireEvent.click(screen.getByRole('button', { name: 'notes' }))
    expect(showInFolder).toHaveBeenLastCalledWith('/Users/zain/projects/mdow/notes')
  })

  it('opens the file itself when the filename is clicked', () => {
    render(<DocumentBreadcrumb tab={tab} />)
    fireEvent.click(screen.getByText('readme.md'))
    expect(showInFolder).toHaveBeenLastCalledWith(tab.path)
  })

  it('does not render a trailing chevron after the filename', () => {
    const { container } = render(<DocumentBreadcrumb tab={tab} />)
    // The chevrons live as <svg> inside the <ol>; the filename lives outside <ol>.
    const ol = container.querySelector('ol')!
    const chevrons = ol.querySelectorAll('svg')
    // We expect (n - 1) chevrons between segments + 1 chevron before the filename.
    // Concretely with 3 segments: 2 between + 1 before filename = 3.
    expect(chevrons.length).toBeGreaterThan(0)
    // The filename is rendered outside the ol, so the last element of ol must be an svg, not a span with file text.
    expect(ol.lastElementChild?.tagName.toLowerCase()).toBe('svg')
  })
})

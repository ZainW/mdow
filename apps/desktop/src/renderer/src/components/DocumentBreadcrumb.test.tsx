import { render, screen, fireEvent } from '@testing-library/react'
import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest'
import { DocumentBreadcrumb } from './DocumentBreadcrumb'
import { useAppStore, type Tab } from '../store/app-store'

const showInFolder = vi.fn()
let originalApi: unknown

beforeEach(() => {
  showInFolder.mockClear()
  originalApi = globalThis.window.api
  // @ts-expect-error — minimal window.api stub for the test
  globalThis.window.api = {
    showInFolder,
  }
  useAppStore.setState({
    wideMode: false,
    openFolderPath: null,
  })
})

afterEach(() => {
  if (originalApi === undefined) {
    // @ts-expect-error — minimal stub teardown
    delete globalThis.window.api
  } else {
    globalThis.window.api = originalApi as typeof globalThis.window.api
  }
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

  it('renders one chevron per parent segment (no trailing chevron after the filename)', () => {
    // /Users/zain/projects/mdow/notes/readme.md with no openFolderPath shows
    // the last 3 parent dirs: projects/mdow/notes → 3 chevrons in the <ol>.
    const { container } = render(<DocumentBreadcrumb tab={tab} />)
    const ol = container.querySelector('ol')!
    const chevrons = ol.querySelectorAll('svg')
    expect(chevrons.length).toBe(3)
    // The filename lives outside the <ol>, and nothing comes between the
    // <ol> and the filename — the filename is the immediate next sibling.
    const filenameBtn = screen.getByText('readme.md').closest('button')!
    expect(ol.nextElementSibling).toBe(filenameBtn)
  })

  it('renders zero chevrons when the file has no parent segments to show', () => {
    const rootTab: Tab = {
      id: 'tab-root',
      path: '/readme.md',
      content: '',
      scrollPosition: 0,
    }
    const { container } = render(<DocumentBreadcrumb tab={rootTab} />)
    const ol = container.querySelector('ol')!
    expect(ol.querySelectorAll('svg').length).toBe(0)
  })
})

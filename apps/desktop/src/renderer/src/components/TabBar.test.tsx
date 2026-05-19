import { render, screen, fireEvent } from '@testing-library/react'
import { describe, expect, it, beforeEach } from 'vitest'
import { TabBar } from './TabBar'
import { useAppStore } from '../store/app-store'

function seedTabs(paths: string[]) {
  useAppStore.setState({
    tabs: paths.map((p, i) => ({
      id: `tab-${i}`,
      path: p,
      content: '',
      scrollPosition: 0,
    })),
    activeTabId: 'tab-0',
  })
}

describe('TabBar', () => {
  beforeEach(() => {
    useAppStore.setState({
      tabs: [],
      activeTabId: null,
    })
  })

  it('exposes aria-setsize/posinset on each tab', () => {
    seedTabs(['/a/one.md', '/a/two.md', '/a/three.md'])
    render(<TabBar />)
    const mainTabs = screen.getAllByRole('tab')
    expect(mainTabs).toHaveLength(3)
    expect(mainTabs[0].getAttribute('aria-setsize')).toBe('3')
    expect(mainTabs[0].getAttribute('aria-posinset')).toBe('1')
    expect(mainTabs[2].getAttribute('aria-posinset')).toBe('3')
  })

  it('wraps the row in a role=tablist', () => {
    seedTabs(['/a/one.md'])
    render(<TabBar />)
    expect(screen.getByRole('tablist', { name: 'Open documents' })).toBeInTheDocument()
  })

  it('opens a role=menu on right-click with menuitem children', () => {
    seedTabs(['/a/one.md', '/a/two.md'])
    render(<TabBar />)
    const tab = screen.getAllByRole('button', { name: /one\.md/ })[0]
    fireEvent.contextMenu(tab.parentElement!.parentElement!, { clientX: 50, clientY: 20 })
    const menu = screen.getByRole('menu')
    expect(menu).toBeInTheDocument()
    const items = screen.getAllByRole('menuitem')
    // Close, Close Others, Close to the Right, Close All, Copy Path, Reveal/Show
    expect(items.length).toBe(6)
    expect(items[0].textContent).toContain('Close')
  })

  it('disables "Close to the Right" on the last tab and "Close Others" with one tab', () => {
    seedTabs(['/a/one.md'])
    render(<TabBar />)
    const tab = screen.getAllByRole('button', { name: /one\.md/ })[0]
    fireEvent.contextMenu(tab.parentElement!.parentElement!, { clientX: 50, clientY: 20 })
    const closeOthers = screen.getByRole('menuitem', { name: /Close Others/ })
    const closeRight = screen.getByRole('menuitem', { name: /Close to the Right/ })
    expect(closeOthers).toBeDisabled()
    expect(closeRight).toBeDisabled()
  })

  it('shows a kbd hint for the Close action', () => {
    seedTabs(['/a/one.md'])
    render(<TabBar />)
    const tab = screen.getAllByRole('button', { name: /one\.md/ })[0]
    fireEvent.contextMenu(tab.parentElement!.parentElement!, { clientX: 50, clientY: 20 })
    const closeItem = screen.getByRole('menuitem', { name: /Close.*W/ })
    expect(closeItem.querySelector('kbd')).not.toBeNull()
  })

  it('marks only the active tab as tabIndex=0', () => {
    seedTabs(['/a/one.md', '/a/two.md', '/a/three.md'])
    // active is tab-0 from seedTabs
    render(<TabBar />)
    const tabs = screen.getAllByRole('tab')
    expect(tabs[0].getAttribute('tabindex')).toBe('0')
    expect(tabs[1].getAttribute('tabindex')).toBe('-1')
    expect(tabs[2].getAttribute('tabindex')).toBe('-1')
  })

  it('ArrowRight moves focus to the next tab and activates it', () => {
    seedTabs(['/a/one.md', '/a/two.md', '/a/three.md'])
    render(<TabBar />)
    const tabs = screen.getAllByRole('tab')
    tabs[0].focus()
    fireEvent.keyDown(tabs[0], { key: 'ArrowRight' })
    expect(document.activeElement).toBe(tabs[1])
    expect(useAppStore.getState().activeTabId).toBe('tab-1')
  })

  it('navigates menu items with ArrowDown/ArrowUp', () => {
    seedTabs(['/a/one.md', '/a/two.md'])
    render(<TabBar />)
    const tab = screen.getAllByRole('button', { name: /one\.md/ })[0]
    fireEvent.contextMenu(tab.parentElement!.parentElement!, { clientX: 50, clientY: 20 })
    const items = screen.getAllByRole('menuitem')
    // First item should be focused on open
    expect(document.activeElement).toBe(items[0])
    fireEvent.keyDown(document, { key: 'ArrowDown' })
    // Close Others is disabled when only 2 tabs and right-clicked the first?
    // tabCount=2 so hasOthers=true → enabled
    expect(document.activeElement).toBe(items[1])
    fireEvent.keyDown(document, { key: 'ArrowUp' })
    expect(document.activeElement).toBe(items[0])
  })
})

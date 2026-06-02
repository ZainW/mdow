import { beforeEach, describe, expect, it, vi } from 'vitest'

const mermaidMock = vi.hoisted(() => ({
  loadCount: 0,
  initialize: vi.fn(),
  render: vi.fn(),
}))

vi.mock('mermaid', () => {
  mermaidMock.loadCount += 1
  return {
    default: {
      initialize: mermaidMock.initialize,
      render: mermaidMock.render,
    },
  }
})

beforeEach(() => {
  vi.resetModules()
  vi.clearAllMocks()
  mermaidMock.loadCount = 0
  mermaidMock.render.mockResolvedValue({ svg: '<svg><text>ok</text></svg>' })
  document.body.replaceChildren()
})

describe('mermaid renderer', () => {
  it('does not load Mermaid while only applying startup theme state', async () => {
    const { initMermaid, updateMermaidTheme } = await import('./mermaid')

    initMermaid(false)
    updateMermaidTheme(true)

    expect(mermaidMock.loadCount).toBe(0)
  })

  it('loads Mermaid on demand when rendering diagram blocks', async () => {
    const { initMermaid, renderMermaidBlocks } = await import('./mermaid')
    const el = document.createElement('div')
    el.id = 'diagram-1'
    document.body.append(el)

    initMermaid(false)
    await renderMermaidBlocks([{ id: 'diagram-1', code: 'flowchart TD\n  A --> B' }])

    expect(mermaidMock.loadCount).toBe(1)
    expect(mermaidMock.initialize).toHaveBeenCalled()
    expect(mermaidMock.render).toHaveBeenCalledWith('diagram-1-svg', 'flowchart TD\n  A --> B')
    expect(el.querySelector('svg')).toBeInTheDocument()
    expect(el.getAttribute('role')).toBe('img')
  })

  it('self-initializes when a block renders before startup initialization runs', async () => {
    const { renderMermaidBlock } = await import('./mermaid')
    const el = document.createElement('div')
    el.id = 'diagram-race'
    document.body.append(el)

    await renderMermaidBlock({ id: 'diagram-race', code: 'flowchart TD\n  A --> B' })

    expect(mermaidMock.initialize).toHaveBeenCalled()
    expect(mermaidMock.render).toHaveBeenCalledWith('diagram-race-svg', 'flowchart TD\n  A --> B')
    expect(el.querySelector('svg')).toBeInTheDocument()
  })

  it('reuses cached SVG output for repeated renders', async () => {
    const { initMermaid, renderMermaidBlock } = await import('./mermaid')
    const el = document.createElement('div')
    el.id = 'diagram-2'
    document.body.append(el)

    initMermaid(false)
    await renderMermaidBlock({ id: 'diagram-2', code: 'flowchart TD\n  C --> D' })
    await renderMermaidBlock({ id: 'diagram-2', code: 'flowchart TD\n  C --> D' })

    expect(mermaidMock.render).toHaveBeenCalledTimes(1)
  })
})

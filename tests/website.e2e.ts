import { test } from '@e2e-dev/web'
import { expect } from 'e2e'

test('the home page links to the docs', async ({ app, screen, browser }) => {
  await app.open('/')
  await expect(screen.getByRole('heading', 'A quiet place to read markdown.')).toBeVisible()
  await screen.getByRole('navigation', 'Main').getByRole('link', 'Docs').tap()
  await expect(browser).toHaveURL('/docs/getting-started')
  await expect(screen.getByRole('heading', { name: 'Getting Started', level: 1 })).toBeVisible()
})

test('docs search jumps to a matching section', async ({ app, screen, browser }) => {
  await app.open('/docs/getting-started')
  await screen.getByRole('button', 'Search docs ⌘K').tap()
  const search = screen.getByRole('dialog', 'Search documentation')
  await search.getByRole('combobox').fill('mermaid')
  await expect(search.getByRole('option', 'Mermaid diagrams Guide · Features')).toBeVisible()
  await search.getByRole('combobox').press('Enter')
  await expect(browser).toHaveURL('/docs/features#mermaid-diagrams')
  await expect(screen.getByRole('heading', 'Mermaid diagrams')).toBeVisible()
})

test('the theme toggle switches to dark mode', async ({ app, screen }) => {
  await app.open('/')
  await screen.getByRole('button', 'Switch to dark mode').tap()
  await expect(screen.getByRole('button', 'Switch to light mode')).toBeVisible()
})

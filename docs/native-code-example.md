# TypeScript Command Palette Example

This example shows a small command palette model with typed actions, filtered
results, and keyboard-driven selection.

```typescript
type CommandKind = 'file' | 'navigation' | 'workspace'

interface Command {
  readonly id: string
  readonly title: string
  readonly kind: CommandKind
  readonly shortcut?: string
  run(): Promise<void> | void
}

const commands: Command[] = [
  {
    id: 'open-file',
    title: 'Open File',
    kind: 'file',
    shortcut: 'cmd+o',
    run: () => window.api.openFile(),
  },
  {
    id: 'toggle-sidebar',
    title: 'Toggle Sidebar',
    kind: 'navigation',
    shortcut: 'cmd+b',
    run: () => window.api.toggleSidebar(),
  },
]
```

Filtering keeps the command title, kind, and shortcut searchable while preserving
the original command order.

```typescript
function scoreCommand(command: Command, query: string): number {
  const normalizedQuery = query.trim().toLowerCase()
  if (normalizedQuery.length === 0) return 1

  const haystack = [
    command.title,
    command.kind,
    command.shortcut ?? '',
  ].join(' ').toLowerCase()

  if (haystack.startsWith(normalizedQuery)) return 100
  if (haystack.includes(normalizedQuery)) return 50
  return 0
}
```

The palette state is a narrow reducer so keyboard handlers can move selection
without re-rendering unrelated application state.

```typescript
type PaletteState = {
  query: string
  selectedIndex: number
  visibleCommands: Command[]
}

function moveSelection(state: PaletteState, offset: number): PaletteState {
  const total = state.visibleCommands.length
  if (total === 0) return state

  return {
    ...state,
    selectedIndex: (state.selectedIndex + offset + total) % total,
  }
}

async function submitSelection(state: PaletteState): Promise<void> {
  const command = state.visibleCommands[state.selectedIndex]
  if (!command) return

  await command.run()
}
```

The result is a predictable command palette where every interaction is typed,
searchable, and easy to trigger from the keyboard.

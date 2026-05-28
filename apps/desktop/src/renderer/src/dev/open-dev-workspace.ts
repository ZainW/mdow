import { useAppStore } from '../store/app-store'
import uiShowcase from './fixtures/ui-showcase.md?raw'
import prDiff from './fixtures/pr-diff.md?raw'

const DEV_SHOWCASE_PATH = 'dev/ui-showcase.md'
const DEV_DIFF_PATH = 'dev/pr-diff.md'

/** Opens in-memory dev tabs for UI work (markdown + diff sample). */
export function openDevWorkspace(): void {
  const { openTab } = useAppStore.getState()
  openTab({ path: DEV_SHOWCASE_PATH, content: uiShowcase })
  openTab({ path: DEV_DIFF_PATH, content: prDiff })
}

export const DEV_WORKSPACE_PATHS = [DEV_SHOWCASE_PATH, DEV_DIFF_PATH] as const

import type { ElectronAPI } from '../shared/api-types'

declare global {
  interface Window {
    api: ElectronAPI
  }
}

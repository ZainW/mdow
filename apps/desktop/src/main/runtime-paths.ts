import { join } from 'path'

export function getDevelopmentUserDataPath(appDataPath: string): string {
  return join(appDataPath, 'Mdow Development')
}

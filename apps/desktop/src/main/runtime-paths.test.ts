import { join } from 'path'
import { describe, expect, it } from 'vitest'
import { getDevelopmentUserDataPath } from './runtime-paths'

describe('getDevelopmentUserDataPath', () => {
  it('isolates the dev instance from the installed Mdow user data and single-instance lock', () => {
    const appData = join('/Users', 'example', 'Library', 'Application Support')
    const installedUserData = join(appData, 'Mdow')

    const developmentUserData = getDevelopmentUserDataPath(appData)

    expect(developmentUserData).toBe(join(appData, 'Mdow Development'))
    expect(developmentUserData).not.toBe(installedUserData)
  })
})

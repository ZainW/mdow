/**
 * Compile Mdow Native into one executable with `bun build --compile`.
 *
 * `@gpuix/native` picks its platform addon with a runtime `require` Bun cannot follow, so
 * the compiled binary would ship without it. This embeds the host's `.node` file explicitly
 * and points napi-rs's `NAPI_RS_NATIVE_LIBRARY_PATH` override at the embedded copy.
 *
 * Usage: bun scripts/build.ts [--outfile dist/mdow-native]
 */
import { mkdirSync, rmSync, writeFileSync } from 'node:fs'
import { createRequire } from 'node:module'
import { dirname, join, resolve } from 'node:path'

const root = resolve(import.meta.dir, '..')
const args = process.argv.slice(2)
const outfile = resolve(root, args[args.indexOf('--outfile') + 1] || 'dist/mdow-native')

const TARGETS: Record<string, string> = {
  'darwin-arm64': 'darwin-arm64',
  'linux-x64': 'linux-x64-gnu',
  'win32-x64': 'win32-x64-msvc',
}
const target = TARGETS[`${process.platform}-${process.arch}`]
if (!target) throw new Error(`gpuix has no prebuilt addon for ${process.platform}-${process.arch}`)

// Resolve the platform package the way @gpuix/native itself would.
const nativeRequire = createRequire(
  createRequire(join(root, 'package.json')).resolve('@gpuix/native'),
)
const addonPackage = dirname(nativeRequire.resolve(`@gpuix/native-${target}/package.json`))
const addon = join(addonPackage, `gpuix-native.${target}.node`)

const entry = join(root, 'dist', '.entry.ts')
mkdirSync(dirname(entry), { recursive: true })
writeFileSync(
  entry,
  [
    `import addon from ${JSON.stringify(addon)} with { type: 'file' }`,
    `process.env.NAPI_RS_NATIVE_LIBRARY_PATH ||= addon`,
    `await import(${JSON.stringify(join(root, 'src/main.tsx'))})`,
    '',
  ].join('\n'),
)

try {
  const result = Bun.spawnSync(
    ['bun', 'build', '--compile', '--minify', '--sourcemap', entry, '--outfile', outfile],
    { cwd: root, stdout: 'inherit', stderr: 'inherit' },
  )
  if (result.exitCode !== 0) process.exit(result.exitCode ?? 1)
} finally {
  rmSync(entry, { force: true })
}
console.log(`built ${outfile}`)

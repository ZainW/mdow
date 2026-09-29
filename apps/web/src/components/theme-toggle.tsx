import { useTheme } from '~/hooks/use-theme'
import { MoonIcon, SunIcon } from './icons'

export function ThemeToggle() {
  const { theme, toggleTheme } = useTheme()

  return (
    <button
      type="button"
      onClick={toggleTheme}
      className="btn btn-ghost relative size-9 rounded-lg p-0"
      aria-label={`Switch to ${theme === 'dark' ? 'light' : 'dark'} mode`}
    >
      {/* Both icons are rendered and cross-faded with CSS so the server render
          and the hydrated render match regardless of the stored theme. */}
      <SunIcon className="absolute size-[18px] scale-50 opacity-0 transition-[opacity,transform] duration-200 ease-out dark:scale-100 dark:opacity-100" />
      <MoonIcon className="absolute size-[18px] transition-[opacity,transform] duration-200 ease-out dark:scale-50 dark:opacity-0" />
    </button>
  )
}

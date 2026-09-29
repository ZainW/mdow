// Embedded with `with { type: 'file' }` so a compiled binary carries them. Linux copies them
// to a real folder for fontconfig; macOS registers the .app's Resources copy instead.
import charterBold from '../../assets/fonts/Charter-Bold.ttf' with { type: 'file' }
import charterBoldItalic from '../../assets/fonts/Charter-BoldItalic.ttf' with { type: 'file' }
import charterItalic from '../../assets/fonts/Charter-Italic.ttf' with { type: 'file' }
import charterRegular from '../../assets/fonts/Charter-Regular.ttf' with { type: 'file' }
import geistMono400 from '../../assets/fonts/GeistMono-400.ttf' with { type: 'file' }
import geistMono500 from '../../assets/fonts/GeistMono-500.ttf' with { type: 'file' }
import geistMono700 from '../../assets/fonts/GeistMono-700.ttf' with { type: 'file' }
import geistMonoItalic400 from '../../assets/fonts/GeistMono-Italic-400.ttf' with { type: 'file' }
import geistMonoItalic700 from '../../assets/fonts/GeistMono-Italic-700.ttf' with { type: 'file' }
import inter400 from '../../assets/fonts/Inter-400.ttf' with { type: 'file' }
import inter500 from '../../assets/fonts/Inter-500.ttf' with { type: 'file' }
import inter600 from '../../assets/fonts/Inter-600.ttf' with { type: 'file' }
import inter700 from '../../assets/fonts/Inter-700.ttf' with { type: 'file' }
import interItalic400 from '../../assets/fonts/Inter-Italic-400.ttf' with { type: 'file' }
import interItalic500 from '../../assets/fonts/Inter-Italic-500.ttf' with { type: 'file' }
import interItalic600 from '../../assets/fonts/Inter-Italic-600.ttf' with { type: 'file' }
import interItalic700 from '../../assets/fonts/Inter-Italic-700.ttf' with { type: 'file' }
import jetBrainsMono400 from '../../assets/fonts/JetBrainsMono-400.ttf' with { type: 'file' }
import jetBrainsMono500 from '../../assets/fonts/JetBrainsMono-500.ttf' with { type: 'file' }
import jetBrainsMono700 from '../../assets/fonts/JetBrainsMono-700.ttf' with { type: 'file' }
import jetBrainsMonoItalic400 from '../../assets/fonts/JetBrainsMono-Italic-400.ttf' with { type: 'file' }
import jetBrainsMonoItalic700 from '../../assets/fonts/JetBrainsMono-Italic-700.ttf' with { type: 'file' }

export const BUNDLED_FONTS: Record<string, string> = {
  'Charter-Bold.ttf': charterBold,
  'Charter-BoldItalic.ttf': charterBoldItalic,
  'Charter-Italic.ttf': charterItalic,
  'Charter-Regular.ttf': charterRegular,
  'GeistMono-400.ttf': geistMono400,
  'GeistMono-500.ttf': geistMono500,
  'GeistMono-700.ttf': geistMono700,
  'GeistMono-Italic-400.ttf': geistMonoItalic400,
  'GeistMono-Italic-700.ttf': geistMonoItalic700,
  'Inter-400.ttf': inter400,
  'Inter-500.ttf': inter500,
  'Inter-600.ttf': inter600,
  'Inter-700.ttf': inter700,
  'Inter-Italic-400.ttf': interItalic400,
  'Inter-Italic-500.ttf': interItalic500,
  'Inter-Italic-600.ttf': interItalic600,
  'Inter-Italic-700.ttf': interItalic700,
  'JetBrainsMono-400.ttf': jetBrainsMono400,
  'JetBrainsMono-500.ttf': jetBrainsMono500,
  'JetBrainsMono-700.ttf': jetBrainsMono700,
  'JetBrainsMono-Italic-400.ttf': jetBrainsMonoItalic400,
  'JetBrainsMono-Italic-700.ttf': jetBrainsMonoItalic700,
}

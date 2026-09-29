// Embedded with `with { type: 'file' }` so a compiled binary carries them. Linux copies them
// to a real folder for fontconfig; macOS registers the .app's Resources copy instead.
import charterBold from '../../assets/fonts/Charter-Bold.ttf' with { type: 'file' }
import charterBoldItalic from '../../assets/fonts/Charter-BoldItalic.ttf' with { type: 'file' }
import charterItalic from '../../assets/fonts/Charter-Italic.ttf' with { type: 'file' }
import charterRegular from '../../assets/fonts/Charter-Regular.ttf' with { type: 'file' }
import geistMonoBold from '../../assets/fonts/GeistMono-Bold.ttf' with { type: 'file' }
import geistMonoBoldItalic from '../../assets/fonts/GeistMono-BoldItalic.ttf' with { type: 'file' }
import geistMonoItalic from '../../assets/fonts/GeistMono-Italic.ttf' with { type: 'file' }
import geistMonoMedium from '../../assets/fonts/GeistMono-Medium.ttf' with { type: 'file' }
import geistMonoRegular from '../../assets/fonts/GeistMono-Regular.ttf' with { type: 'file' }
import interBold from '../../assets/fonts/Inter-Bold.ttf' with { type: 'file' }
import interBoldItalic from '../../assets/fonts/Inter-BoldItalic.ttf' with { type: 'file' }
import interItalic from '../../assets/fonts/Inter-Italic.ttf' with { type: 'file' }
import interMedium from '../../assets/fonts/Inter-Medium.ttf' with { type: 'file' }
import interRegular from '../../assets/fonts/Inter-Regular.ttf' with { type: 'file' }
import interSemiBold from '../../assets/fonts/Inter-SemiBold.ttf' with { type: 'file' }
import jetBrainsMonoBold from '../../assets/fonts/JetBrainsMono-Bold.ttf' with { type: 'file' }
import jetBrainsMonoBoldItalic from '../../assets/fonts/JetBrainsMono-BoldItalic.ttf' with { type: 'file' }
import jetBrainsMonoItalic from '../../assets/fonts/JetBrainsMono-Italic.ttf' with { type: 'file' }
import jetBrainsMonoRegular from '../../assets/fonts/JetBrainsMono-Regular.ttf' with { type: 'file' }

export const BUNDLED_FONTS: Record<string, string> = {
  'Charter-Bold.ttf': charterBold,
  'Charter-BoldItalic.ttf': charterBoldItalic,
  'Charter-Italic.ttf': charterItalic,
  'Charter-Regular.ttf': charterRegular,
  'GeistMono-Bold.ttf': geistMonoBold,
  'GeistMono-BoldItalic.ttf': geistMonoBoldItalic,
  'GeistMono-Italic.ttf': geistMonoItalic,
  'GeistMono-Medium.ttf': geistMonoMedium,
  'GeistMono-Regular.ttf': geistMonoRegular,
  'Inter-Bold.ttf': interBold,
  'Inter-BoldItalic.ttf': interBoldItalic,
  'Inter-Italic.ttf': interItalic,
  'Inter-Medium.ttf': interMedium,
  'Inter-Regular.ttf': interRegular,
  'Inter-SemiBold.ttf': interSemiBold,
  'JetBrainsMono-Bold.ttf': jetBrainsMonoBold,
  'JetBrainsMono-BoldItalic.ttf': jetBrainsMonoBoldItalic,
  'JetBrainsMono-Italic.ttf': jetBrainsMonoItalic,
  'JetBrainsMono-Regular.ttf': jetBrainsMonoRegular,
}

import AppKit

/// Darkens HTML pages that have no dark appearance of their own, by inverting the page and
/// inverting its media back. The filter sits on `<body>`, so `<html>` keeps a real dark
/// background for the area WebKit paints around the page. Runs in Mdow's own script world,
/// so page scripts stay off.
enum HTMLDarkMode {
  static let background = NSColor(srgbRed: 0x1b / 255, green: 0x1b / 255, blue: 0x1b / 255, alpha: 1)

  static let script = """
    (() => {
      if (window.__mdowSetDark) return;
      const supportsDark = () => {
        const meta = document.querySelector('meta[name="color-scheme"]');
        if (meta && /dark/i.test(meta.content)) return true;
        if (/dark/.test(getComputedStyle(document.documentElement).colorScheme || '')) return true;
        for (const sheet of document.styleSheets) {
          try {
            for (const rule of sheet.cssRules) {
              if (rule.media && /prefers-color-scheme:\\s*dark/.test(rule.media.mediaText)) return true;
            }
          } catch (error) {}
        }
        return false;
      };
      // Checked once, before Mdow's own style is added.
      const native = window.__mdowNative ?? (window.__mdowNative = supportsDark());
      window.__mdowSetDark = (on) => {
        let style = document.getElementById('__mdow-dark');
        if (native || !on) {
          if (style) style.remove();
          return false;
        }
        if (style) return true;
        style = document.createElement('style');
        style.id = '__mdow-dark';
        style.textContent = `
          html { background: #1b1b1b !important; }
          body { filter: invert(1) hue-rotate(180deg); background: #e4e4e4; min-height: 100vh; }
          img, video, picture, canvas, iframe, object, embed, svg image,
          [style*="background-image"] { filter: invert(1) hue-rotate(180deg); }`;
        document.documentElement.appendChild(style);
        return true;
      };
    })();
    """
}

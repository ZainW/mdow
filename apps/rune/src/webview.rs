//! WebKit, for what Rune can't draw: HTML files shown as the page itself (in a `WKWebView` with
//! page scripts off, laid over the pane), and printing or exporting a document as a paginated
//! PDF with real text. Pages that have no dark mode are darkened in dark mode, as in the Swift
//! app, by a script in Mdow's own content world.

#[cfg(target_os = "macos")]
use crate::app::Cmd;
use std::path::{Path, PathBuf};

/// Where an HTML pane sits, in the window's points from its top-left.
pub type Rect = [f32; 4];

#[cfg(target_os = "macos")]
mod imp {
    use super::*;
    use block2::RcBlock;
    use objc2::rc::Retained;
    use objc2::runtime::{AnyObject, NSObject, ProtocolObject};
    use objc2::{
        AnyThread, DefinedClass, MainThreadMarker, MainThreadOnly, Message, define_class, msg_send,
    };
    use objc2_app_kit::{
        NSColor, NSPrintInfo, NSPrintOperation, NSPrintingPaginationMode, NSView, NSWindow,
    };
    use objc2_foundation::{NSError, NSObjectProtocol, NSPoint, NSRect, NSSize, NSString, NSURL};
    use objc2_web_kit::{
        WKContentWorld, WKFindConfiguration, WKFindResult, WKNavigation, WKNavigationAction,
        WKNavigationActionPolicy, WKNavigationDelegate, WKNavigationType, WKWebView,
        WKWebViewConfiguration, WKWebsiteDataStore,
    };
    use std::cell::{Cell, RefCell};
    use std::collections::HashMap;
    use std::ptr::NonNull;

    pub struct Ivars {
        window: String,
        pane: u64,
        /// Print this page once it has loaded: (window to attach the panel to, save-to path).
        print: RefCell<Option<(String, Option<PathBuf>, String)>>,
        dark: Cell<bool>,
    }

    define_class!(
        #[unsafe(super(NSObject))]
        #[thread_kind = MainThreadOnly]
        #[name = "MdowWebDelegate"]
        #[ivars = Ivars]
        struct Delegate;

        unsafe impl NSObjectProtocol for Delegate {}

        unsafe impl WKNavigationDelegate for Delegate {
            #[unsafe(method(webView:didFinishNavigation:))]
            fn did_finish(&self, web: &WKWebView, _navigation: Option<&WKNavigation>) {
                let iv = self.ivars();
                if let Some((window, save, title)) = iv.print.borrow_mut().take() {
                    run_print(web, &window, save, &title);
                    return;
                }
                apply_dark(web, iv.dark.get());
            }

            #[unsafe(method(webView:decidePolicyForNavigationAction:decisionHandler:))]
            fn decide(
                &self,
                web: &WKWebView,
                action: &WKNavigationAction,
                handler: &block2::DynBlock<dyn Fn(WKNavigationActionPolicy)>,
            ) {
                let iv = self.ivars();
                let link = unsafe { action.navigationType() } == WKNavigationType::LinkActivated;
                let url = unsafe { action.request() }.URL();
                let (Some(url), true) = (url, link) else {
                    handler.call((WKNavigationActionPolicy::Allow,));
                    return;
                };
                let text = url.absoluteString().map(|s| s.to_string()).unwrap_or_default();
                // Same-page anchors scroll in place; everything else goes through the app.
                let current = unsafe { web.URL() }
                    .and_then(|u| u.absoluteString())
                    .map(|s| s.to_string())
                    .unwrap_or_default();
                let page = current.split('#').next().unwrap_or("").to_string();
                if text.contains('#') && !page.is_empty() && text.starts_with(&page) {
                    handler.call((WKNavigationActionPolicy::Allow,));
                    return;
                }
                handler.call((WKNavigationActionPolicy::Cancel,));
                let new_tab = unsafe { action.modifierFlags() }
                    .contains(objc2_app_kit::NSEventModifierFlags::Command);
                crate::windows::send(&iv.window, Cmd::OpenUrl(iv.pane, text, new_tab));
            }
        }
    );

    impl Delegate {
        fn new(mtm: MainThreadMarker, window: &str, pane: u64) -> Retained<Self> {
            let this = Self::alloc(mtm).set_ivars(Ivars {
                window: window.to_string(),
                pane,
                print: RefCell::new(None),
                dark: Cell::new(false),
            });
            unsafe { msg_send![super(this), init] }
        }
    }

    struct Web {
        window: String,
        view: Retained<WKWebView>,
        _delegate: Retained<Delegate>,
        path: PathBuf,
        dark: bool,
        darken: bool,
        zoom: f64,
    }

    thread_local! {
        static HOSTS: RefCell<HashMap<String, Retained<NSView>>> = RefCell::new(HashMap::new());
        /// By window and pane: pane ids are only unique within a window.
        static WEBS: RefCell<HashMap<(String, u64), Web>> = RefCell::new(HashMap::new());
        /// Web views kept alive while they print.
        static PRINTING: RefCell<Vec<(Retained<WKWebView>, Retained<Delegate>)>> = const { RefCell::new(Vec::new()) };
        /// Find results by pane: true when the page has a match.
        static FOUND: RefCell<HashMap<(String, u64), bool>> = RefCell::new(HashMap::new());
    }

    const DARK_SCRIPT: &str = r#"(() => {
  if (window.__mdowSetDark) return;
  const supportsDark = () => {
    const meta = document.querySelector('meta[name="color-scheme"]');
    if (meta && /dark/i.test(meta.content)) return true;
    if (/dark/.test(getComputedStyle(document.documentElement).colorScheme || '')) return true;
    for (const sheet of document.styleSheets) {
      try {
        for (const rule of sheet.cssRules) {
          if (rule.media && /prefers-color-scheme:\s*dark/.test(rule.media.mediaText)) return true;
        }
      } catch (error) {}
    }
    return false;
  };
  const native = window.__mdowNative ?? (window.__mdowNative = supportsDark());
  window.__mdowSetDark = (on) => {
    let style = document.getElementById('__mdow-dark');
    if (native || !on) { if (style) style.remove(); return false; }
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
})();"#;

    fn apply_dark(web: &WKWebView, on: bool) {
        let Some(mtm) = MainThreadMarker::new() else { return };
        let script =
            NSString::from_str(&format!("{DARK_SCRIPT}\nwindow.__mdowSetDark({on}) ?? false"));
        let target: Retained<WKWebView> = web.retain();
        let done = RcBlock::new(move |result: *mut AnyObject, _err: *mut NSError| {
            // WebKit paints the overscroll area with the page's own colour, sampled before it
            // was darkened; a fixed colour stands in.
            let darkened = !result.is_null() && unsafe { msg_send![&*result, boolValue] };
            let color = darkened.then(|| {
                NSColor::colorWithSRGBRed_green_blue_alpha(
                    0x1b as f64 / 255.0,
                    0x1b as f64 / 255.0,
                    0x1b as f64 / 255.0,
                    1.0,
                )
            });
            unsafe { target.setUnderPageBackgroundColor(color.as_deref()) };
        });
        unsafe {
            web.evaluateJavaScript_inFrame_inContentWorld_completionHandler(
                &script,
                None,
                &WKContentWorld::defaultClientWorld(mtm),
                Some(&done),
            )
        };
    }

    fn file_url(path: &Path) -> Retained<NSURL> {
        NSURL::fileURLWithPath(&NSString::from_str(&path.to_string_lossy()))
    }

    fn make_web(mtm: MainThreadMarker, delegate: &Delegate, frame: NSRect) -> Retained<WKWebView> {
        unsafe {
            let config = WKWebViewConfiguration::new(mtm);
            // Pages are shown, never run. Mdow's own scripts run in a separate world.
            config.defaultWebpagePreferences().setAllowsContentJavaScript(false);
            config.setWebsiteDataStore(&WKWebsiteDataStore::nonPersistentDataStore(mtm));
            let web = WKWebView::initWithFrame_configuration(WKWebView::alloc(mtm), frame, &config);
            web.setNavigationDelegate(Some(ProtocolObject::from_ref(delegate)));
            web.setAllowsMagnification(true);
            web
        }
    }

    /// Remembers the window's content view, for laying web views over panes.
    pub fn attach(window: &str, handle: &rune::WindowHandle) {
        let view = handle.with_raw_handle(|h| match h.as_raw() {
            rune::raw_window_handle::RawWindowHandle::AppKit(a) => unsafe {
                Retained::retain(a.ns_view.as_ptr() as *mut NSView)
            },
            _ => None,
        });
        if let Ok(Some(view)) = view {
            HOSTS.with_borrow_mut(|h| h.insert(window.to_string(), view));
        }
    }

    fn to_ns(host: &NSView, [x, y, w, h]: Rect) -> NSRect {
        let y = if host.isFlipped() { y } else { host.bounds().size.height as f32 - y - h };
        NSRect::new(
            NSPoint::new(x as f64, y as f64),
            NSSize::new(w.max(1.0) as f64, h.max(1.0) as f64),
        )
    }

    /// Shows `path` over pane `pane`, at `rect`.
    pub fn place(
        window: &str,
        pane: u64,
        path: &Path,
        rect: Rect,
        dark: bool,
        darken: bool,
        zoom: f32,
    ) {
        let Some(mtm) = MainThreadMarker::new() else { return };
        let Some(host) = HOSTS.with_borrow(|h| h.get(window).cloned()) else { return };
        let frame = to_ns(&host, rect);
        WEBS.with_borrow_mut(|webs| {
            let key = (window.to_string(), pane);
            let stale = webs.get(&key).is_some_and(|w| w.path != path);
            if stale && let Some(old) = webs.remove(&key) {
                old.view.removeFromSuperview();
            }
            let web = webs.entry(key).or_insert_with(|| {
                let delegate = Delegate::new(mtm, window, pane);
                let view = make_web(mtm, &delegate, frame);
                // Round it with the document card under it.
                view.setWantsLayer(true);
                if let Some(layer) = view.layer() {
                    let _: () = unsafe { msg_send![&*layer, setCornerRadius: 11.0f64] };
                    let _: () = unsafe { msg_send![&*layer, setMasksToBounds: true] };
                }
                host.addSubview(&view);
                let dir = path.parent().unwrap_or(Path::new("/"));
                unsafe {
                    view.loadFileURL_allowingReadAccessToURL(&file_url(path), &file_url(dir))
                };
                Web {
                    window: window.to_string(),
                    view,
                    _delegate: delegate,
                    path: path.to_path_buf(),
                    dark: false,
                    darken: false,
                    zoom: 0.0,
                }
            });
            web.view.setFrame(frame);
            web.view.setHidden(false);
            let on = dark && darken;
            if web.dark != dark || web.darken != darken {
                web.dark = dark;
                web.darken = darken;
                web._delegate.ivars().dark.set(on);
                apply_dark(&web.view, on);
            }
            if (web.zoom - zoom as f64).abs() > 0.001 {
                web.zoom = zoom as f64;
                unsafe { web.view.setPageZoom(zoom as f64) };
            }
        });
    }

    /// Hides the web views of `window` other than `keep`, and all of them while `covered`
    /// (Quick Open or a sheet is up: Rune draws those under native views).
    pub fn sync(window: &str, keep: &[u64], covered: bool) {
        WEBS.with_borrow(|webs| {
            for ((_, pane), web) in webs.iter().filter(|(_, w)| w.window == window) {
                web.view.setHidden(covered || !keep.contains(pane));
            }
        });
    }

    /// Drops the web views of panes that are gone.
    pub fn retain_panes(window: &str, alive: &[u64]) {
        WEBS.with_borrow_mut(|webs| {
            webs.retain(|(_, pane), web| {
                let keep = web.window != window || alive.contains(pane);
                if !keep {
                    web.view.removeFromSuperview();
                }
                keep
            })
        });
    }

    pub fn reload(window: &str, pane: u64) {
        WEBS.with_borrow(|webs| {
            if let Some(w) = webs.get(&(window.to_string(), pane)) {
                unsafe { w.view.reload() };
            }
        });
    }

    /// Finds `query` in the page, selecting the next (or previous) match; the answer arrives
    /// in `found` once WebKit has looked.
    pub fn find(window: &str, pane: u64, query: &str, backwards: bool, waker: Option<rune::Waker>) {
        let Some(mtm) = MainThreadMarker::new() else { return };
        WEBS.with_borrow(|webs| {
            let Some(w) = webs.get(&(window.to_string(), pane)) else { return };
            let key = (window.to_string(), pane);
            unsafe {
                let config = WKFindConfiguration::new(mtm);
                config.setBackwards(backwards);
                config.setCaseSensitive(false);
                config.setWraps(true);
                let done = RcBlock::new(move |result: NonNull<WKFindResult>| {
                    let found = result.as_ref().matchFound();
                    FOUND.with_borrow_mut(|f| f.insert(key.clone(), found));
                    if let Some(w) = &waker {
                        w();
                    }
                });
                w.view.findString_withConfiguration_completionHandler(
                    &NSString::from_str(query),
                    Some(&config),
                    &done,
                );
            }
        });
    }

    pub fn found(window: &str, pane: u64) -> Option<bool> {
        FOUND.with_borrow_mut(|f| f.remove(&(window.to_string(), pane)))
    }

    fn window_of(window: &str) -> Option<Retained<NSWindow>> {
        HOSTS.with_borrow(|h| h.get(window).and_then(|v| v.window()))
    }

    fn run_print(web: &WKWebView, window: &str, save: Option<PathBuf>, title: &str) {
        let Some(ns_window) = window_of(window) else { return };
        unsafe {
            let info: Retained<NSPrintInfo> = msg_send![&*NSPrintInfo::sharedPrintInfo(), copy];
            info.setHorizontalPagination(NSPrintingPaginationMode::Fit);
            info.setVerticalPagination(NSPrintingPaginationMode::Automatic);
            info.setHorizontallyCentered(false);
            info.setVerticallyCentered(false);
            info.setTopMargin(54.0);
            info.setBottomMargin(54.0);
            info.setLeftMargin(54.0);
            info.setRightMargin(54.0);
            if let Some(path) = &save {
                info.setJobDisposition(objc2_app_kit::NSPrintSaveJob);
                let dict = info.dictionary();
                let _: () = msg_send![&*dict, setObject: &*file_url(path), forKey: objc2_app_kit::NSPrintJobSavingURL];
            }
            let width = info.paperSize().width - 108.0;
            let op: Retained<NSPrintOperation> = web.printOperationWithPrintInfo(&info);
            if let Some(view) = op.view() {
                view.setFrame(NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(width, 100.0)));
            }
            op.setJobTitle(Some(&NSString::from_str(title)));
            op.setShowsPrintPanel(save.is_none());
            op.setShowsProgressPanel(true);
            op.runOperationModalForWindow_delegate_didRunSelector_contextInfo(
                &ns_window,
                None,
                None,
                std::ptr::null_mut(),
            );
        }
    }

    /// Prints pane `pane`'s page if it shows HTML; else lays out `html` (a temporary file) in
    /// an offscreen web view and prints that once it loads. `save` exports a PDF instead.
    pub fn print(
        window: &str,
        pane: u64,
        html: Option<PathBuf>,
        save: Option<PathBuf>,
        title: &str,
    ) {
        let Some(mtm) = MainThreadMarker::new() else { return };
        if html.is_none() {
            let web =
                WEBS.with_borrow(|w| w.get(&(window.to_string(), pane)).map(|w| w.view.clone()));
            if let Some(web) = web {
                run_print(&web, window, save, title);
            }
            return;
        }
        let path = html.unwrap();
        let delegate = Delegate::new(mtm, window, pane);
        *delegate.ivars().print.borrow_mut() = Some((window.to_string(), save, title.to_string()));
        let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(612.0, 792.0));
        let web = make_web(mtm, &delegate, frame);
        // Images and diagrams anywhere on disk: the page is ours and runs no scripts.
        unsafe {
            web.loadFileURL_allowingReadAccessToURL(&file_url(&path), &file_url(Path::new("/")))
        };
        PRINTING.with_borrow_mut(|p| {
            if p.len() > 4 {
                p.remove(0);
            }
            p.push((web, delegate));
        });
    }

    /// Shows the system Share menu for `path` under the point `at` (window points).
    pub fn share(window: &str, path: &Path, at: [f32; 2]) {
        if MainThreadMarker::new().is_none() {
            return;
        }
        let Some(host) = HOSTS.with_borrow(|h| h.get(window).cloned()) else { return };
        unsafe {
            let url = file_url(path);
            let items = objc2_foundation::NSArray::from_retained_slice(&[Retained::into_super(
                Retained::into_super(url),
            )]);
            let picker = objc2_app_kit::NSSharingServicePicker::initWithItems(
                objc2_app_kit::NSSharingServicePicker::alloc(),
                &items,
            );
            let rect = to_ns(&host, [at[0], at[1], 1.0, 1.0]);
            picker.showRelativeToRect_ofView_preferredEdge(
                rect,
                &host,
                objc2_foundation::NSRectEdge::MinY,
            );
        }
    }

    pub fn page_setup() {
        let Some(mtm) = MainThreadMarker::new() else { return };
        let app = objc2_app_kit::NSApplication::sharedApplication(mtm);
        let _: () = unsafe { msg_send![&*app, runPageLayout: std::ptr::null::<AnyObject>()] };
    }

    #[allow(dead_code)]
    fn _assert_any_thread<T: AnyThread>() {}
}

#[cfg(target_os = "macos")]
pub use imp::{attach, find, found, page_setup, place, print, reload, retain_panes, share, sync};

#[cfg(not(target_os = "macos"))]
mod stub {
    use super::*;
    pub fn attach(_: &str, _: &rune::WindowHandle) {}
    pub fn place(_: &str, _: u64, _: &Path, _: Rect, _: bool, _: bool, _: f32) {}
    pub fn sync(_: &str, _: &[u64], _: bool) {}
    pub fn retain_panes(_: &str, _: &[u64]) {}
    pub fn reload(_: &str, _: u64) {}
    pub fn find(_: &str, _: u64, _: &str, _: bool, _: Option<rune::Waker>) {}
    pub fn found(_: &str, _: u64) -> Option<bool> {
        None
    }
    pub fn print(_: &str, _: u64, _: Option<PathBuf>, _: Option<PathBuf>, _: &str) {}
    pub fn share(_: &str, _: &Path, _: [f32; 2]) {}
    pub fn page_setup() {}
}

#[cfg(not(target_os = "macos"))]
pub use stub::*;

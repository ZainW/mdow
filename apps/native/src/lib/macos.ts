/**
 * macOS integration gpuix 0.10 does not expose yet, done through the Objective-C runtime
 * with bun:ffi: bundled font registration, an app-declared menu bar, and Finder
 * "Open With" / Dock drops (GPUI's delegate receives `application:openURLs:` but gpuix
 * never forwards it).
 *
 * Everything here runs on the main thread. On macOS gpuix pumps AppKit from the JS frame
 * loop, so AppKit invokes these callbacks synchronously inside `tick()`; handlers defer
 * their real work to a macrotask so React never commits mid-pump.
 */
import { CFunction, CString, dlopen, FFIType, JSCallback, ptr, type Pointer } from 'bun:ffi'

export interface MenuItemSpec {
  title: string
  run?: () => void
  /** Standard AppKit selector sent down the responder chain, e.g. `hide:`. */
  selector?: string
  key?: string
  modifiers?: ('cmd' | 'shift' | 'alt' | 'ctrl')[]
  separator?: boolean
  services?: boolean
}

export interface MenuSpec {
  title: string
  items: MenuItemSpec[]
  windowsMenu?: boolean
}

const MODIFIER_MASK = { shift: 1 << 17, ctrl: 1 << 18, alt: 1 << 19, cmd: 1 << 20 }
const OBJC = '/usr/lib/libobjc.A.dylib'

type Id = Pointer | null

let runtime: ReturnType<typeof loadRuntime> | null = null
// JSCallbacks must stay reachable for as long as AppKit may call them.
const retained: JSCallback[] = []

function loadRuntime() {
  const objc = dlopen(OBJC, {
    objc_getClass: { args: [FFIType.cstring], returns: FFIType.ptr },
    sel_registerName: { args: [FFIType.cstring], returns: FFIType.ptr },
    object_getClass: { args: [FFIType.ptr], returns: FFIType.ptr },
    objc_allocateClassPair: {
      args: [FFIType.ptr, FFIType.cstring, FFIType.u64],
      returns: FFIType.ptr,
    },
    objc_registerClassPair: { args: [FFIType.ptr], returns: FFIType.void },
    class_addMethod: {
      args: [FFIType.ptr, FFIType.ptr, FFIType.ptr, FFIType.cstring],
      returns: FFIType.bool,
    },
    class_replaceMethod: {
      args: [FFIType.ptr, FFIType.ptr, FFIType.ptr, FFIType.cstring],
      returns: FFIType.ptr,
    },
    class_getInstanceMethod: { args: [FFIType.ptr, FFIType.ptr], returns: FFIType.ptr },
    method_getImplementation: { args: [FFIType.ptr], returns: FFIType.ptr },
    objc_msgSend: { args: [], returns: FFIType.ptr },
  })
  // bun:ffi exposes each symbol's address as `.ptr`; its types omit it.
  const msgSend = (objc.symbols.objc_msgSend as unknown as { ptr: Pointer }).ptr
  const fn = (args: FFIType[], returns: FFIType) => CFunction({ ptr: msgSend, args, returns })
  const P = FFIType.ptr
  return {
    objc,
    send: fn([P, P], P),
    sendI: fn([P, P], FFIType.i64),
    sendP: fn([P, P, P], P),
    sendPP: fn([P, P, P, P], P),
    sendPPP: fn([P, P, P, P, P], P),
    sendIdxP: fn([P, P, FFIType.i64], P),
    sendSetI: fn([P, P, FFIType.i64], FFIType.void),
    sendSetP: fn([P, P, P], FFIType.void),
    sendSetB: fn([P, P, FFIType.bool], FFIType.void),
  }
}

function rt() {
  runtime ??= loadRuntime()
  return runtime
}

const cstr = (value: string) => Buffer.from(`${value}\0`, 'utf8')
const sel = (name: string) => rt().objc.symbols.sel_registerName(cstr(name))
const cls = (name: string) => rt().objc.symbols.objc_getClass(cstr(name))

function nsString(value: string): Id {
  return rt().sendP(cls('NSString'), sel('stringWithUTF8String:'), ptr(cstr(value)))
}

function jsString(object: Id): string | null {
  if (!object) return null
  const utf8 = rt().send(object, sel('UTF8String'))
  return utf8 ? new CString(utf8).toString() : null
}

const defer = (work: () => void) => setTimeout(work, 0)

// ---------------------------------------------------------------------------
// Fonts
// ---------------------------------------------------------------------------

/** Register font files for this process only, so GPUI resolves them by family name. */
export function registerFonts(paths: string[]) {
  const cf = dlopen('/System/Library/Frameworks/CoreFoundation.framework/CoreFoundation', {
    CFURLCreateFromFileSystemRepresentation: {
      args: [FFIType.ptr, FFIType.ptr, FFIType.i64, FFIType.bool],
      returns: FFIType.ptr,
    },
  })
  const ct = dlopen('/System/Library/Frameworks/CoreText.framework/CoreText', {
    CTFontManagerRegisterFontsForURL: {
      args: [FFIType.ptr, FFIType.u32, FFIType.ptr],
      returns: FFIType.bool,
    },
  })
  const kCTFontManagerScopeProcess = 1
  for (const path of paths) {
    const bytes = Buffer.from(path, 'utf8')
    const url = cf.symbols.CFURLCreateFromFileSystemRepresentation(
      null,
      ptr(bytes),
      bytes.length,
      false,
    )
    if (url) ct.symbols.CTFontManagerRegisterFontsForURL(url, kCTFontManagerScopeProcess, null)
  }
}

// ---------------------------------------------------------------------------
// Menu bar
// ---------------------------------------------------------------------------

let menuTarget: Id = null
let menuActions: (() => void)[] = []

function ensureMenuTarget(): Id {
  if (menuTarget) return menuTarget
  const { objc } = rt()
  const superclass = cls('NSObject')
  let klass = cls('MdowMenuTarget')
  if (!klass) {
    klass = objc.symbols.objc_allocateClassPair(superclass, cstr('MdowMenuTarget'), 0)
    const callback = new JSCallback(
      (_self: Id, _cmd: Id, sender: Id) => {
        if (!sender) return
        const tag = Number(rt().sendI(sender, sel('tag')))
        const action = menuActions[tag]
        if (action) defer(action)
      },
      { args: [FFIType.ptr, FFIType.ptr, FFIType.ptr], returns: FFIType.void },
    )
    retained.push(callback)
    objc.symbols.class_addMethod(klass, sel('mdowCommand:'), callback.ptr, cstr('v@:@'))
    objc.symbols.objc_registerClassPair(klass)
  }
  menuTarget = rt().send(rt().send(klass, sel('alloc')), sel('init'))
  return menuTarget
}

/**
 * Replace gpuix's minimal menu bar. Items with `run` call it; items with a
 * `selector` keep AppKit's standard behavior (Hide, Minimize, Services…).
 */
export function installMenuBar(menus: MenuSpec[]) {
  const r = rt()
  const target = ensureMenuTarget()
  menuActions = []
  const app = r.send(cls('NSApplication'), sel('sharedApplication'))

  const makeMenu = (title: string) =>
    r.sendP(r.send(cls('NSMenu'), sel('alloc')), sel('initWithTitle:'), nsString(title))

  const mainMenu = makeMenu('')
  for (const spec of menus) {
    const submenu = makeMenu(spec.title)
    for (const item of spec.items) {
      if (item.separator) {
        r.sendSetP(submenu, sel('addItem:'), r.send(cls('NSMenuItem'), sel('separatorItem')))
        continue
      }
      const action = item.run ? sel('mdowCommand:') : item.selector ? sel(item.selector) : null
      const menuItem = r.sendPPP(
        r.send(cls('NSMenuItem'), sel('alloc')),
        sel('initWithTitle:action:keyEquivalent:'),
        nsString(item.title),
        action,
        nsString(item.key ?? ''),
      )
      if (item.key && item.modifiers) {
        const mask = item.modifiers.reduce((sum, mod) => sum | MODIFIER_MASK[mod], 0)
        r.sendSetI(menuItem, sel('setKeyEquivalentModifierMask:'), mask)
      }
      if (item.run) {
        r.sendSetP(menuItem, sel('setTarget:'), target)
        r.sendSetI(menuItem, sel('setTag:'), menuActions.length)
        menuActions.push(item.run)
      }
      if (item.services) {
        const services = makeMenu('Services')
        r.sendSetP(menuItem, sel('setSubmenu:'), services)
        r.sendSetP(app, sel('setServicesMenu:'), services)
      }
      r.sendSetP(submenu, sel('addItem:'), menuItem)
    }
    const holder = r.sendPPP(
      r.send(cls('NSMenuItem'), sel('alloc')),
      sel('initWithTitle:action:keyEquivalent:'),
      nsString(spec.title),
      null,
      nsString(''),
    )
    r.sendSetP(holder, sel('setSubmenu:'), submenu)
    r.sendSetP(mainMenu, sel('addItem:'), holder)
    if (spec.windowsMenu) r.sendSetP(app, sel('setWindowsMenu:'), submenu)
  }
  r.sendSetP(app, sel('setMainMenu:'), mainMenu)
}

// ---------------------------------------------------------------------------
// Finder "Open With", Dock drops, `open -a`
// ---------------------------------------------------------------------------

/**
 * Route `application:openURLs:` to `onOpen`: Finder "Open With", double-clicks, Dock drops and
 * `open -a`. GPUI's delegate receives these but gpuix never forwards them.
 *
 * Call before `render()`. A cold launch delivers its open event while GPUI is still starting,
 * so the handler must be on GPUI's delegate the moment it exists. This wraps
 * `-[NSApplication setDelegate:]` to patch the delegate as it is set, without touching `NSApp`
 * early (GPUI may install its own NSApplication subclass).
 */
export function handleOpenUrls(onOpen: (paths: string[]) => void) {
  const r = rt()
  const { objc } = r
  const openUrls = new JSCallback(
    (_self: Id, _cmd: Id, _app: Id, urls: Id) => {
      if (!urls) return
      const paths: string[] = []
      const count = Number(r.sendI(urls, sel('count')))
      for (let index = 0; index < count; index++) {
        const url = r.sendIdxP(urls, sel('objectAtIndex:'), index)
        if (!url || !Number(r.sendI(url, sel('isFileURL')))) continue
        const path = jsString(r.send(url, sel('path')))
        if (path) paths.push(path)
      }
      if (paths.length > 0) defer(() => onOpen(paths))
    },
    { args: [FFIType.ptr, FFIType.ptr, FFIType.ptr, FFIType.ptr], returns: FFIType.void },
  )
  retained.push(openUrls)
  const patch = (delegate: Id) => {
    if (!delegate) return
    objc.symbols.class_replaceMethod(
      objc.symbols.object_getClass(delegate),
      sel('application:openURLs:'),
      openUrls.ptr,
      cstr('v@:@@'),
    )
  }

  const application = cls('NSApplication')
  const setDelegate = sel('setDelegate:')
  const original = objc.symbols.method_getImplementation(
    objc.symbols.class_getInstanceMethod(application, setDelegate),
  )
  if (!original) return
  const callOriginal = CFunction({
    ptr: original,
    args: [FFIType.ptr, FFIType.ptr, FFIType.ptr],
    returns: FFIType.void,
  })
  const wrapped = new JSCallback(
    (self: Id, cmd: Id, delegate: Id) => {
      callOriginal(self, cmd, delegate)
      patch(delegate)
    },
    { args: [FFIType.ptr, FFIType.ptr, FFIType.ptr], returns: FFIType.void },
  )
  retained.push(wrapped)
  objc.symbols.class_replaceMethod(application, setDelegate, wrapped.ptr, cstr('v@:@'))
}

/** Bring the app forward, e.g. after Finder hands it a file while another app is active. */
export function activateApp() {
  const r = rt()
  const app = r.send(cls('NSApplication'), sel('sharedApplication'))
  r.sendSetB(app, sel('activateIgnoringOtherApps:'), true)
}

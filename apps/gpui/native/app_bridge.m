// AppKit calls GPUI does not expose: the standard About panel and Bring All to Front.
#import <AppKit/AppKit.h>

void mdow_show_about_panel(void) {
  dispatch_async(dispatch_get_main_queue(), ^{
    [NSApp orderFrontStandardAboutPanel:nil];
    [NSApp activateIgnoringOtherApps:YES];
  });
}

void mdow_arrange_in_front(void) {
  dispatch_async(dispatch_get_main_queue(), ^{
    [NSApp arrangeInFront:nil];
  });
}

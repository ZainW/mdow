// Integration host: uses the production bridge with an isolated bundle and feed.
#import <AppKit/AppKit.h>
#import "sparkle_bridge.h"

static NSString *mode;
static void event(int32_t kind, const char *version, int32_t percent, int32_t flags) {
  printf("event=%d percent=%d manual=%d\n", kind, percent, flags);
  fflush(stdout);
  if (kind == MDOW_SPARKLE_AVAILABLE) {
    dispatch_async(dispatch_get_main_queue(), ^{ mdow_sparkle_download(); });
  } else if (kind == MDOW_SPARKLE_READY) {
    if ([mode isEqualToString:@"install"]) {
      dispatch_async(dispatch_get_main_queue(), ^{ mdow_sparkle_install(); });
    } else {
      exit([mode isEqualToString:@"download"] ? 0 : 2);
    }
  } else if (kind == MDOW_SPARKLE_UP_TO_DATE) {
    exit([mode isEqualToString:@"current"] ? 0 : 3);
  } else if (kind == MDOW_SPARKLE_FAILED) {
    exit([mode isEqualToString:@"failure"] ? 0 : 4);
  }
}

int main(void) {
  @autoreleasepool {
    NSBundle *bundle = NSBundle.mainBundle;
    NSString *marker = [bundle objectForInfoDictionaryKey:@"MdowTestMarker"];
    if ([[bundle objectForInfoDictionaryKey:@"CFBundleVersion"] isEqualToString:@"2"]) {
      [@"installed-and-relaunched" writeToFile:marker atomically:YES
                                    encoding:NSUTF8StringEncoding error:NULL];
      return 0;
    }
    mode = NSProcessInfo.processInfo.environment[@"MDOW_TEST_MODE"];
    [NSApplication sharedApplication];
    [NSApp setActivationPolicy:NSApplicationActivationPolicyAccessory];
    mdow_sparkle_set_event_callback(event);
    dispatch_async(dispatch_get_main_queue(), ^{
      if (!mdow_sparkle_start()) exit(5);
      mdow_sparkle_check(1);
    });
    dispatch_after(dispatch_time(DISPATCH_TIME_NOW, 45 * NSEC_PER_SEC),
                   dispatch_get_main_queue(), ^{ exit(6); });
    [NSApp run];
  }
  return 0;
}

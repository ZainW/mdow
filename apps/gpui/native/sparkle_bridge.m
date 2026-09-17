#import "sparkle_bridge.h"

#import <Foundation/Foundation.h>
#import <Sparkle/Sparkle.h>

static mdow_sparkle_event_cb gEventCallback;

static void emit_event(int32_t kind, NSString *version, int32_t percent, int32_t flags) {
  if (gEventCallback == NULL) {
    return;
  }
  const char *version_utf8 = version != nil ? version.UTF8String : NULL;
  gEventCallback(kind, version_utf8, percent, flags);
}

@interface MdowSparkleHost : NSObject <SPUUserDriver, SPUUpdaterDelegate>
@property (nonatomic, strong) SPUUpdater *updater;
@property (nonatomic, copy) void (^foundReply)(SPUUserUpdateChoice);
@property (nonatomic, copy) void (^readyReply)(SPUUserUpdateChoice);
@property (nonatomic, copy) void (^errorAck)(void);
@property (nonatomic, copy) NSString *version;
@property (nonatomic) BOOL userInitiated;
@property (nonatomic) uint64_t expectedLength;
@property (nonatomic) uint64_t receivedLength;
@end

@implementation MdowSparkleHost

static MdowSparkleHost *gHost;

+ (instancetype)shared {
  return gHost;
}

- (void)clearPendingRepliesWithChoice:(SPUUserUpdateChoice)choice {
  if (self.foundReply) {
    self.foundReply(choice);
    self.foundReply = nil;
  }
  if (self.readyReply) {
    self.readyReply(choice);
    self.readyReply = nil;
  }
  if (self.errorAck) {
    self.errorAck();
    self.errorAck = nil;
  }
}

- (int32_t)flags {
  return self.userInitiated ? MDOW_SPARKLE_FLAG_MANUAL : 0;
}

- (nullable NSString *)feedURLStringForUpdater:(SPUUpdater *)updater {
  (void)updater;
  NSString *override = [[NSProcessInfo processInfo] environment][@"MDOW_SPARKLE_FEED_URL"];
  if (override.length > 0) {
    return override;
  }
  return nil;
}

- (BOOL)updater:(SPUUpdater *)updater
    shouldDownloadReleaseNotesForUpdate:(SUAppcastItem *)updateItem {
  (void)updater;
  (void)updateItem;
  return NO;
}

- (BOOL)updaterShouldPromptForPermissionToCheckForUpdates:(SPUUpdater *)updater {
  (void)updater;
  return NO;
}

- (void)showUpdatePermissionRequest:(SPUUpdatePermissionRequest *)request
                                  reply:(void (^)(SUUpdatePermissionResponse *))reply {
  (void)request;
  reply([[SUUpdatePermissionResponse alloc] initWithAutomaticUpdateChecks:YES
                                                           sendSystemProfile:NO]);
}

- (void)showUserInitiatedUpdateCheckWithCancellation:(void (^)(void))cancellation {
  (void)cancellation;
  self.userInitiated = YES;
  emit_event(MDOW_SPARKLE_CHECKING, self.version, -1, self.flags);
}

- (void)showUpdateFoundWithAppcastItem:(SUAppcastItem *)appcastItem
                                  state:(SPUUserUpdateState *)state
                                  reply:(void (^)(SPUUserUpdateChoice))reply {
  self.userInitiated = state.userInitiated;
  self.version = appcastItem.displayVersionString ?: appcastItem.versionString;
  self.foundReply = reply;
  self.readyReply = nil;

  if (appcastItem.informationOnlyUpdate) {
    emit_event(MDOW_SPARKLE_AVAILABLE, self.version, -1, self.flags);
    return;
  }

  switch (state.stage) {
  case SPUUserUpdateStageInstalling:
    emit_event(MDOW_SPARKLE_READY, self.version, 100, self.flags);
    self.readyReply = reply;
    self.foundReply = nil;
    break;
  case SPUUserUpdateStageDownloaded:
    emit_event(MDOW_SPARKLE_DOWNLOADING, self.version, 100, self.flags);
    reply(SPUUserUpdateChoiceInstall);
    self.foundReply = nil;
    break;
  case SPUUserUpdateStageNotDownloaded:
  default:
    emit_event(MDOW_SPARKLE_AVAILABLE, self.version, -1, self.flags);
    break;
  }
}

- (void)showUpdateReleaseNotesWithDownloadData:(SPUDownloadData *)downloadData {
  (void)downloadData;
}

- (void)showUpdateReleaseNotesFailedToDownloadWithError:(NSError *)error {
  (void)error;
}

- (void)showUpdateNotFoundWithError:(NSError *)error acknowledgement:(void (^)(void))acknowledgement {
  (void)error;
  emit_event(MDOW_SPARKLE_UP_TO_DATE, self.version, -1, self.flags);
  acknowledgement();
}

- (void)showUpdaterError:(NSError *)error acknowledgement:(void (^)(void))acknowledgement {
  NSLog(@"Mdow Native update check failed: %@", error);
  emit_event(MDOW_SPARKLE_FAILED, self.version, -1, self.flags);
  acknowledgement();
}

- (void)showDownloadInitiatedWithCancellation:(void (^)(void))cancellation {
  (void)cancellation;
  self.expectedLength = 0;
  self.receivedLength = 0;
  emit_event(MDOW_SPARKLE_DOWNLOADING, self.version, 0, self.flags);
}

- (void)showDownloadDidReceiveExpectedContentLength:(uint64_t)expectedContentLength {
  self.expectedLength = expectedContentLength;
}

- (void)showDownloadDidReceiveDataOfLength:(uint64_t)length {
  self.receivedLength += length;
  int32_t percent = 0;
  if (self.expectedLength > 0) {
    double ratio = (double)self.receivedLength / (double)self.expectedLength;
    if (ratio > 1.0) {
      ratio = 1.0;
    }
    percent = (int32_t)(ratio * 100.0);
  }
  emit_event(MDOW_SPARKLE_DOWNLOADING, self.version, percent, self.flags);
}

- (void)showDownloadDidStartExtractingUpdate {
  emit_event(MDOW_SPARKLE_DOWNLOADING, self.version, 100, self.flags);
}

- (void)showExtractionReceivedProgress:(double)progress {
  int32_t percent = (int32_t)(progress * 100.0);
  if (percent < 0) {
    percent = 0;
  }
  if (percent > 100) {
    percent = 100;
  }
  emit_event(MDOW_SPARKLE_DOWNLOADING, self.version, percent, self.flags);
}

- (void)showReadyToInstallAndRelaunch:(void (^)(SPUUserUpdateChoice))reply {
  self.readyReply = reply;
  emit_event(MDOW_SPARKLE_READY, self.version, 100, self.flags);
}

- (void)showInstallingUpdateWithApplicationTerminated:(BOOL)applicationTerminated
                             retryTerminatingApplication:(void (^)(void))retryTerminatingApplication {
  (void)applicationTerminated;
  (void)retryTerminatingApplication;
}

- (void)showUpdateInstalledAndRelaunched:(BOOL)relaunched acknowledgement:(void (^)(void))acknowledgement {
  (void)relaunched;
  acknowledgement();
}

- (void)dismissUpdateInstallation {
  self.foundReply = nil;
  self.readyReply = nil;
  self.errorAck = nil;
}

@end

static BOOL sparkle_configured(void) {
  NSBundle *bundle = [NSBundle mainBundle];
  NSString *bundle_path = bundle.bundlePath;
  if (![bundle_path hasSuffix:@".app"]) {
    return NO;
  }
  NSString *feed = [bundle objectForInfoDictionaryKey:@"SUFeedURL"];
  NSString *key = [bundle objectForInfoDictionaryKey:@"SUPublicEDKey"];
  return feed.length > 0 && key.length > 0;
}

void mdow_sparkle_set_event_callback(mdow_sparkle_event_cb callback) {
  gEventCallback = callback;
}

int32_t mdow_sparkle_is_enabled(void) {
  return sparkle_configured() ? 1 : 0;
}

int32_t mdow_sparkle_start(void) {
  if (gHost.updater != nil) {
    return 1;
  }
  if (!sparkle_configured()) {
    return 0;
  }

  MdowSparkleHost *host = [[MdowSparkleHost alloc] init];
  NSBundle *bundle = [NSBundle mainBundle];
  SPUUpdater *updater = [[SPUUpdater alloc] initWithHostBundle:bundle
                                             applicationBundle:bundle
                                                    userDriver:host
                                                      delegate:host];
  NSError *error = nil;
  if (![updater startUpdater:&error]) {
    NSLog(@"Mdow Native Sparkle failed to start: %@", error);
    emit_event(MDOW_SPARKLE_FAILED, nil, -1, 0);
    return 0;
  }
  host.updater = updater;
  gHost = host;

  if (updater.automaticallyChecksForUpdates) {
    [updater checkForUpdatesInBackground];
  }
  return 1;
}

void mdow_sparkle_check(int32_t user_initiated) {
  MdowSparkleHost *host = [MdowSparkleHost shared];
  if (host.updater == nil) {
    return;
  }
  host.userInitiated = user_initiated != 0;
  if (user_initiated) {
    [host.updater checkForUpdates];
  } else if (host.updater.automaticallyChecksForUpdates) {
    [host.updater checkForUpdatesInBackground];
  }
}

void mdow_sparkle_download(void) {
  MdowSparkleHost *host = [MdowSparkleHost shared];
  if (host.foundReply) {
    emit_event(MDOW_SPARKLE_DOWNLOADING, host.version, 0, host.flags);
    host.foundReply(SPUUserUpdateChoiceInstall);
    host.foundReply = nil;
  }
}

void mdow_sparkle_install(void) {
  MdowSparkleHost *host = [MdowSparkleHost shared];
  if (host.readyReply) {
    host.readyReply(SPUUserUpdateChoiceInstall);
    host.readyReply = nil;
  }
}

void mdow_sparkle_dismiss_choice(void) {
  [[MdowSparkleHost shared] clearPendingRepliesWithChoice:SPUUserUpdateChoiceDismiss];
}

int32_t mdow_sparkle_can_check(void) {
  MdowSparkleHost *host = [MdowSparkleHost shared];
  return host.updater.canCheckForUpdates ? 1 : 0;
}

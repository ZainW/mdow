#ifndef MDOW_SPARKLE_BRIDGE_H
#define MDOW_SPARKLE_BRIDGE_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

enum MdowSparkleEventKind {
  MDOW_SPARKLE_IDLE = 0,
  MDOW_SPARKLE_CHECKING = 1,
  MDOW_SPARKLE_AVAILABLE = 2,
  MDOW_SPARKLE_DOWNLOADING = 3,
  MDOW_SPARKLE_READY = 4,
  MDOW_SPARKLE_UP_TO_DATE = 5,
  MDOW_SPARKLE_FAILED = 6,
};

enum MdowSparkleEventFlags {
  MDOW_SPARKLE_FLAG_MANUAL = 1 << 0,
};

typedef void (*mdow_sparkle_event_cb)(
    int32_t kind,
    const char *version,
    int32_t percent,
    int32_t flags);

void mdow_sparkle_set_event_callback(mdow_sparkle_event_cb callback);
int32_t mdow_sparkle_start(void);
void mdow_sparkle_check(int32_t user_initiated);
void mdow_sparkle_download(void);
void mdow_sparkle_install(void);
void mdow_sparkle_dismiss_choice(void);
int32_t mdow_sparkle_can_check(void);
int32_t mdow_sparkle_is_enabled(void);

#ifdef __cplusplus
}
#endif

#endif

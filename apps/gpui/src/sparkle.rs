//! Sparkle update feed helpers and UI state. Network-free and cfg-safe on every OS.
//!
//! macOS talks to Sparkle 2 through `native/sparkle_bridge.m`. Linux compiles this
//! module without Sparkle and never offers install.

pub const NATIVE_APPCAST_FILENAME: &str = "appcast-native-mac.xml";
pub const GITHUB_REPO: &str = "ZainW/mdow";
pub const DEFAULT_CHECK_INTERVAL_SECS: u64 = 86_400;
pub const LAUNCH_CHECK_DELAY_SECS: u64 = 2;

pub fn latest_appcast_url(repo: &str) -> String {
    format!("https://github.com/{repo}/releases/latest/download/{NATIVE_APPCAST_FILENAME}")
}

pub fn default_feed_url() -> String {
    latest_appcast_url(GITHUB_REPO)
}

pub fn enclosure_url(repo: &str, tag: &str, version: &str) -> String {
    let tag = tag.trim();
    format!(
        "https://github.com/{repo}/releases/download/{tag}/MdowNative-{version}-arm64-mac-beta.zip"
    )
}

pub fn enclosure_url_for_version(repo: &str, version: &str) -> String {
    enclosure_url(repo, &format!("v{version}"), version)
}

pub fn is_native_feed_url(url: &str) -> bool {
    url.contains(NATIVE_APPCAST_FILENAME) && !url.contains("latest-mac.yml")
}

/// Numeric dotted compare (`1.9.1` > `1.9.0`). Pre-release suffixes sort below the
/// matching numeric prefix so `1.9.0-beta` < `1.9.0`.
pub fn version_cmp(left: &str, right: &str) -> std::cmp::Ordering {
    let (left_parts, left_pre) = split_pre(left);
    let (right_parts, right_pre) = split_pre(right);
    let len = left_parts.len().max(right_parts.len());
    for index in 0..len {
        let l = left_parts.get(index).copied().unwrap_or(0);
        let r = right_parts.get(index).copied().unwrap_or(0);
        match l.cmp(&r) {
            std::cmp::Ordering::Equal => {}
            other => return other,
        }
    }
    match (left_pre, right_pre) {
        (None, None) => std::cmp::Ordering::Equal,
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (Some(left_pre), Some(right_pre)) => left_pre.cmp(right_pre),
    }
}

pub fn version_is_newer(candidate: &str, current: &str) -> bool {
    version_cmp(candidate, current) == std::cmp::Ordering::Greater
}

fn split_pre(raw: &str) -> (Vec<u64>, Option<&str>) {
    let trimmed = raw.trim().trim_start_matches('v');
    let (numeric, pre) = match trimmed.split_once('-') {
        Some((numeric, rest)) => (numeric, Some(rest)),
        None => (trimmed, None),
    };
    let parts = numeric
        .split('.')
        .filter_map(|part| part.parse::<u64>().ok())
        .collect();
    (parts, pre)
}

pub fn native_appcast_xml(
    repo: &str,
    version: &str,
    build: &str,
    enclosure: &str,
    length: u64,
    ed_signature: &str,
    pub_date: &str,
) -> String {
    format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<rss version="2.0" xmlns:sparkle="http://www.andymatuschak.org/xml-namespaces/sparkle">
  <channel>
    <title>Mdow Native</title>
    <link>{feed}</link>
    <description>Mdow Native updates</description>
    <language>en</language>
    <item>
      <title>Mdow Native {version}</title>
      <pubDate>{pub_date}</pubDate>
      <enclosure url="{enclosure}" sparkle:version="{build}" sparkle:shortVersionString="{version}" sparkle:edSignature="{ed_signature}" length="{length}" type="application/octet-stream"/>
    </item>
  </channel>
</rss>
"#,
        feed = latest_appcast_url(repo),
    )
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum UpdateUi {
    #[default]
    Idle,
    Checking {
        manual: bool,
    },
    Available {
        version: String,
    },
    Downloading {
        version: String,
        percent: u8,
    },
    Ready {
        version: String,
    },
    UpToDate {
        manual: bool,
    },
    Failed {
        manual: bool,
    },
}

impl UpdateUi {
    pub fn banner_copy(&self) -> Option<String> {
        match self {
            Self::Idle => None,
            Self::Checking { manual: true } => Some("Checking for updates…".into()),
            Self::Checking { manual: false } => None,
            Self::Available { version } => Some(format!("Mdow Native {version} is available")),
            Self::Downloading { percent, .. } => Some(format!("Downloading update… {percent}%")),
            Self::Ready { .. } => Some("Update ready. Restart to apply.".into()),
            Self::UpToDate { manual: true } => Some("You're on the latest version".into()),
            Self::UpToDate { manual: false } => None,
            Self::Failed { manual: true } => {
                Some("Couldn't check for updates. Try again later.".into())
            }
            Self::Failed { manual: false } => None,
        }
    }

    pub fn shows_banner(&self) -> bool {
        self.banner_copy().is_some()
    }

    pub fn can_download(&self) -> bool {
        matches!(self, Self::Available { .. })
    }

    pub fn can_install(&self) -> bool {
        matches!(self, Self::Ready { .. })
    }

    pub fn action_label(&self) -> Option<&'static str> {
        if self.can_download() {
            Some("Download")
        } else if self.can_install() {
            Some("Restart")
        } else {
            None
        }
    }

    pub fn apply(&self, event: UpdateEvent) -> Self {
        match event {
            UpdateEvent::Checking { manual } => Self::Checking { manual },
            UpdateEvent::Available { version } => Self::Available { version },
            UpdateEvent::Downloading { version, percent } => Self::Downloading {
                version,
                percent: percent.min(100),
            },
            UpdateEvent::Ready { version } => Self::Ready { version },
            UpdateEvent::UpToDate { manual } => Self::UpToDate { manual },
            UpdateEvent::Failed { manual } => Self::Failed { manual },
            UpdateEvent::Idle => Self::Idle,
        }
    }

    pub fn resets_dismissed(&self) -> bool {
        matches!(
            self,
            Self::Available { .. }
                | Self::Downloading { .. }
                | Self::Ready { .. }
                | Self::UpToDate { manual: true }
                | Self::Failed { manual: true }
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateEvent {
    Idle,
    Checking { manual: bool },
    Available { version: String },
    Downloading { version: String, percent: u8 },
    Ready { version: String },
    UpToDate { manual: bool },
    Failed { manual: bool },
}

impl UpdateEvent {
    pub fn from_bridge(kind: i32, version: Option<&str>, percent: i32, flags: i32) -> Option<Self> {
        let manual = flags & 1 != 0;
        let version = version.unwrap_or("").to_owned();
        match kind {
            0 => Some(Self::Idle),
            1 => Some(Self::Checking { manual }),
            2 => Some(Self::Available { version }),
            3 => Some(Self::Downloading {
                version,
                percent: percent.clamp(0, 100) as u8,
            }),
            4 => Some(Self::Ready { version }),
            5 => Some(Self::UpToDate { manual }),
            6 => Some(Self::Failed { manual }),
            _ => None,
        }
    }
}

pub fn is_supported() -> bool {
    cfg!(target_os = "macos")
}

pub fn start() -> bool {
    #[cfg(target_os = "macos")]
    {
        return macos::start();
    }
    #[cfg(not(target_os = "macos"))]
    false
}

pub fn check(manual: bool) {
    #[cfg(target_os = "macos")]
    macos::check(manual);
    #[cfg(not(target_os = "macos"))]
    let _ = manual;
}

pub fn download() {
    #[cfg(target_os = "macos")]
    macos::download();
}

pub fn install() {
    #[cfg(target_os = "macos")]
    macos::install();
}

pub fn dismiss_choice() {
    #[cfg(target_os = "macos")]
    macos::dismiss_choice();
}

pub fn current_ui() -> UpdateUi {
    #[cfg(target_os = "macos")]
    {
        return macos::snapshot();
    }
    #[cfg(not(target_os = "macos"))]
    UpdateUi::Idle
}

#[cfg(target_os = "macos")]
mod macos {
    use super::{UpdateEvent, UpdateUi};
    use std::ffi::CStr;
    use std::os::raw::c_char;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicBool, Ordering};

    unsafe extern "C" {
        fn mdow_sparkle_set_event_callback(
            callback: Option<unsafe extern "C" fn(i32, *const c_char, i32, i32)>,
        );
        fn mdow_sparkle_start() -> i32;
        fn mdow_sparkle_check(user_initiated: i32);
        fn mdow_sparkle_download();
        fn mdow_sparkle_install();
        fn mdow_sparkle_dismiss_choice();
        fn mdow_sparkle_is_enabled() -> i32;
    }

    static STARTED: AtomicBool = AtomicBool::new(false);
    static STATE: Mutex<UpdateUi> = Mutex::new(UpdateUi::Idle);

    unsafe extern "C" fn on_event(kind: i32, version: *const c_char, percent: i32, flags: i32) {
        let version = if version.is_null() {
            None
        } else {
            unsafe { CStr::from_ptr(version) }.to_str().ok()
        };
        let Some(event) = UpdateEvent::from_bridge(kind, version, percent, flags) else {
            return;
        };
        if let Ok(mut state) = STATE.lock() {
            *state = state.apply(event);
        }
    }

    pub fn start() -> bool {
        if STARTED.load(Ordering::SeqCst) {
            return unsafe { mdow_sparkle_is_enabled() != 0 };
        }
        unsafe {
            mdow_sparkle_set_event_callback(Some(on_event));
            let started = mdow_sparkle_start() != 0;
            STARTED.store(true, Ordering::SeqCst);
            started
        }
    }

    pub fn check(manual: bool) {
        unsafe { mdow_sparkle_check(i32::from(manual)) }
    }

    pub fn download() {
        unsafe { mdow_sparkle_download() }
    }

    pub fn install() {
        unsafe { mdow_sparkle_install() }
    }

    pub fn dismiss_choice() {
        unsafe { mdow_sparkle_dismiss_choice() }
    }

    pub fn snapshot() -> UpdateUi {
        STATE.lock().map(|state| state.clone()).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_feed_is_github_latest_appcast_not_electron_yml() {
        let url = default_feed_url();
        assert_eq!(
            url,
            "https://github.com/ZainW/mdow/releases/latest/download/appcast-native-mac.xml"
        );
        assert!(is_native_feed_url(&url));
        assert!(!url.contains("latest-mac.yml"));
        assert!(!url.contains("latest-linux.yml"));
        assert_eq!(DEFAULT_CHECK_INTERVAL_SECS, 86_400);
        assert_eq!(LAUNCH_CHECK_DELAY_SECS, 2);
    }

    #[test]
    fn enclosure_url_targets_the_native_zip_on_the_version_tag() {
        assert_eq!(
            enclosure_url_for_version("ZainW/mdow", "1.9.1"),
            "https://github.com/ZainW/mdow/releases/download/v1.9.1/MdowNative-1.9.1-arm64-mac-beta.zip"
        );
        assert!(!enclosure_url_for_version("ZainW/mdow", "1.9.1").contains("latest-mac.yml"));
    }

    #[test]
    fn version_compare_orders_dotted_numeric_and_prerelease() {
        assert!(version_is_newer("1.9.1", "1.9.0"));
        assert!(version_is_newer("1.10.0", "1.9.9"));
        assert!(!version_is_newer("1.9.0", "1.9.0"));
        assert!(!version_is_newer("1.8.9", "1.9.0"));
        assert!(version_is_newer("1.9.0", "1.9.0-beta"));
        assert_eq!(version_cmp("1.9.0", "1.9.0"), std::cmp::Ordering::Equal);
    }

    #[test]
    fn appcast_xml_names_native_enclosure_and_eddsa() {
        let xml = native_appcast_xml(
            "ZainW/mdow",
            "1.9.1",
            "42",
            &enclosure_url_for_version("ZainW/mdow", "1.9.1"),
            12,
            "abc+def/ghi=",
            "Wed, 09 Sep 2026 00:00:00 +0000",
        );
        assert!(xml.contains("appcast-native-mac.xml"));
        assert!(xml.contains("MdowNative-1.9.1-arm64-mac-beta.zip"));
        assert!(xml.contains("sparkle:edSignature=\"abc+def/ghi=\""));
        assert!(xml.contains("sparkle:version=\"42\""));
        assert!(xml.contains("sparkle:shortVersionString=\"1.9.1\""));
        assert!(!xml.contains("latest-mac.yml"));
    }

    #[test]
    fn update_ui_does_not_offer_install_until_ready() {
        let available = UpdateUi::Idle.apply(UpdateEvent::Available {
            version: "1.9.1".into(),
        });
        assert!(available.can_download());
        assert!(!available.can_install());
        assert_eq!(available.action_label(), Some("Download"));
        assert_eq!(
            available.banner_copy().as_deref(),
            Some("Mdow Native 1.9.1 is available")
        );

        let downloading = available.apply(UpdateEvent::Downloading {
            version: "1.9.1".into(),
            percent: 40,
        });
        assert!(!downloading.can_download());
        assert!(!downloading.can_install());
        assert_eq!(downloading.action_label(), None);

        let ready = downloading.apply(UpdateEvent::Ready {
            version: "1.9.1".into(),
        });
        assert!(!ready.can_download());
        assert!(ready.can_install());
        assert_eq!(ready.action_label(), Some("Restart"));
        assert_eq!(
            ready.banner_copy().as_deref(),
            Some("Update ready. Restart to apply.")
        );
    }

    #[test]
    fn background_up_to_date_and_failed_stay_quiet() {
        let quiet_ok = UpdateUi::Idle.apply(UpdateEvent::UpToDate { manual: false });
        assert!(!quiet_ok.shows_banner());
        let quiet_fail = UpdateUi::Idle.apply(UpdateEvent::Failed { manual: false });
        assert!(!quiet_fail.shows_banner());
        let manual_ok = UpdateUi::Idle.apply(UpdateEvent::UpToDate { manual: true });
        assert_eq!(
            manual_ok.banner_copy().as_deref(),
            Some("You're on the latest version")
        );
        assert!(manual_ok.resets_dismissed());
    }

    #[test]
    fn linux_never_claims_sparkle_support() {
        assert_eq!(is_supported(), cfg!(target_os = "macos"));
        #[cfg(not(target_os = "macos"))]
        {
            assert!(!start());
            assert_eq!(current_ui(), UpdateUi::Idle);
        }
    }

    #[test]
    fn bridge_kind_mapping_matches_objc_enum() {
        let event = UpdateEvent::from_bridge(4, Some("1.9.1"), 100, 1).unwrap();
        assert_eq!(
            event,
            UpdateEvent::Ready {
                version: "1.9.1".into()
            }
        );
        assert!(UpdateEvent::from_bridge(99, None, 0, 0).is_none());
    }
}

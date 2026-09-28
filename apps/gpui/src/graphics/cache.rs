//! Process-wide cache of rendered graphics, bounded by the bytes their images hold.

use super::Graphic;
use std::{
    collections::HashMap,
    sync::{Arc, LazyLock, Mutex, MutexGuard},
};

/// Decoded image bytes the cache keeps before evicting the least recently used graphics.
const BYTE_BUDGET: usize = 256 * 1024 * 1024;

/// Everything that changes a rendered graphic's pixels.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum GraphicKey {
    Math {
        tex: String,
        display: bool,
        /// Straight RGBA.
        color: [u8; 4],
        /// Font size in hundredths of a logical pixel.
        font_size: u32,
    },
    Diagram {
        source: String,
        dark: bool,
        /// Raster scale in hundredths.
        scale: u32,
    },
}

#[derive(Debug, Clone)]
pub enum GraphicState {
    /// Rendering on a background thread.
    Pending,
    Ready(Graphic),
    /// The source could not be rendered; the reader shows it as text instead.
    Failed(Arc<str>),
}

struct Entry {
    state: GraphicState,
    last_used: u64,
    bytes: usize,
}

#[derive(Default)]
struct Entries {
    map: HashMap<GraphicKey, Entry>,
    clock: u64,
    bytes: usize,
}

#[derive(Default)]
pub struct GraphicCache {
    entries: Mutex<Entries>,
    budget: Option<usize>,
}

static GLOBAL: LazyLock<GraphicCache> = LazyLock::new(GraphicCache::default);

impl GraphicCache {
    pub fn global() -> &'static Self {
        &GLOBAL
    }

    #[cfg(test)]
    fn with_budget(budget: usize) -> Self {
        Self {
            entries: Mutex::default(),
            budget: Some(budget),
        }
    }

    fn lock(&self) -> MutexGuard<'_, Entries> {
        self.entries
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// The cached state for `key`, marking it recently used.
    pub fn get(&self, key: &GraphicKey) -> Option<GraphicState> {
        let mut entries = self.lock();
        entries.clock += 1;
        let clock = entries.clock;
        let entry = entries.map.get_mut(key)?;
        entry.last_used = clock;
        Some(entry.state.clone())
    }

    /// The cached state for `key`, or `None` after marking it pending when nobody has started
    /// rendering it yet. A `None` return obliges the caller to render and [`Self::insert`].
    pub fn get_or_begin(&self, key: &GraphicKey) -> Option<GraphicState> {
        if let Some(state) = self.get(key) {
            return Some(state);
        }
        self.insert(key.clone(), GraphicState::Pending);
        None
    }

    /// The cached state for `key`, rendering it synchronously on a miss.
    pub fn get_or_render(
        &self,
        key: &GraphicKey,
        render: impl FnOnce() -> Result<Graphic, String>,
    ) -> GraphicState {
        match self.get(key) {
            Some(GraphicState::Pending) | None => {
                let state = match render() {
                    Ok(graphic) => GraphicState::Ready(graphic),
                    Err(error) => GraphicState::Failed(error.into()),
                };
                self.insert(key.clone(), state.clone());
                state
            }
            Some(state) => state,
        }
    }

    pub fn insert(&self, key: GraphicKey, state: GraphicState) {
        let bytes = match &state {
            GraphicState::Ready(graphic) => graphic.byte_size(),
            GraphicState::Pending | GraphicState::Failed(_) => 0,
        };
        let budget = self.budget.unwrap_or(BYTE_BUDGET);
        let mut entries = self.lock();
        entries.clock += 1;
        let last_used = entries.clock;
        if let Some(previous) = entries.map.insert(
            key.clone(),
            Entry {
                state,
                last_used,
                bytes,
            },
        ) {
            entries.bytes -= previous.bytes;
        }
        entries.bytes += bytes;

        while entries.bytes > budget {
            let Some(oldest) = entries
                .map
                .iter()
                .filter(|(candidate, entry)| **candidate != key && entry.bytes > 0)
                .min_by_key(|(_, entry)| entry.last_used)
                .map(|(candidate, _)| candidate.clone())
            else {
                break;
            };
            if let Some(evicted) = entries.map.remove(&oldest) {
                entries.bytes -= evicted.bytes;
            }
        }
    }

    #[cfg(test)]
    fn contains(&self, key: &GraphicKey) -> bool {
        self.lock().map.contains_key(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graphics::rasterize_svg;

    fn key(name: &str) -> GraphicKey {
        GraphicKey::Diagram {
            source: name.into(),
            dark: false,
            scale: 100,
        }
    }

    fn graphic() -> Graphic {
        // 10x10 logical pixels rasterize to 20x20 device pixels: 1600 bytes.
        rasterize_svg(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"/>"#,
            None,
            None,
            1.0,
        )
        .unwrap()
    }

    #[test]
    fn begins_each_key_once_and_serves_the_result() {
        let cache = GraphicCache::default();

        assert!(cache.get_or_begin(&key("a")).is_none());
        assert!(matches!(
            cache.get_or_begin(&key("a")),
            Some(GraphicState::Pending)
        ));
        cache.insert(key("a"), GraphicState::Failed("nope".into()));
        assert!(matches!(
            cache.get(&key("a")),
            Some(GraphicState::Failed(message)) if &*message == "nope"
        ));
    }

    #[test]
    fn renders_synchronously_once_and_remembers_failures() {
        let cache = GraphicCache::default();
        let mut calls = 0;
        for _ in 0..2 {
            cache.get_or_render(&key("bad"), || {
                calls += 1;
                Err("broken".into())
            });
        }
        assert_eq!(calls, 1);
    }

    #[test]
    fn evicts_the_least_recently_used_graphics_over_budget() {
        let cache = GraphicCache::with_budget(3200);
        cache.insert(key("a"), GraphicState::Ready(graphic()));
        cache.insert(key("b"), GraphicState::Ready(graphic()));
        cache.get(&key("a"));
        cache.insert(key("c"), GraphicState::Ready(graphic()));

        assert!(cache.contains(&key("a")));
        assert!(!cache.contains(&key("b")));
        assert!(cache.contains(&key("c")));
    }
}

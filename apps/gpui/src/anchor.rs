//! Reload diffs and scroll anchors for the virtualized reader.
//!
//! Mirrors Electron's section signatures and `scroll-anchor.ts`: a live reload keeps every block
//! whose signature did not change (so the list keeps its measured height instead of
//! re-estimating it), and a reading position is "this block, this many pixels above the
//! viewport top" rather than a raw pixel offset, so content inserted above does not move what
//! the reader is looking at.

use std::ops::Range;

/// How far a reload diff looks ahead for the next matching block before treating a run as
/// replaced. Local edits resync within a few blocks; wholesale rewrites fall back to one splice.
const RESYNC_WINDOW: usize = 64;
/// How far a restored anchor searches for its block's signature around the expected index.
const ANCHOR_SEARCH: usize = 4096;

/// One replaced run: old items `old` became `new_len` items starting at `new_start`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Splice {
    pub old: Range<usize>,
    pub new_start: usize,
    pub new_len: usize,
}

/// The changed runs between two block lists, in ascending order, as list splices.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BlockDiff {
    pub splices: Vec<Splice>,
}

impl BlockDiff {
    pub fn compute(old: &[u64], new: &[u64]) -> Self {
        let prefix = old
            .iter()
            .zip(new)
            .take_while(|(old, new)| old == new)
            .count();
        let suffix = old[prefix..]
            .iter()
            .rev()
            .zip(new[prefix..].iter().rev())
            .take_while(|(old, new)| old == new)
            .count();
        let (old_end, new_end) = (old.len() - suffix, new.len() - suffix);
        let mut splices = Vec::new();
        let (mut i, mut j) = (prefix, prefix);
        let mut open: Option<(usize, usize)> = None;
        let close = |open: &mut Option<(usize, usize)>, i: usize, j: usize, out: &mut Vec<_>| {
            if let Some((old_start, new_start)) = open.take() {
                out.push(Splice {
                    old: old_start..i,
                    new_start,
                    new_len: j - new_start,
                });
            }
        };
        while i < old_end || j < new_end {
            if i < old_end && j < new_end && old[i] == new[j] {
                close(&mut open, i, j, &mut splices);
                i += 1;
                j += 1;
                continue;
            }
            open.get_or_insert((i, j));
            if i >= old_end {
                j = new_end;
                continue;
            }
            if j >= new_end {
                i = old_end;
                continue;
            }
            let inserted = (j + 1..new_end.min(j + RESYNC_WINDOW)).find(|&k| new[k] == old[i]);
            let removed = (i + 1..old_end.min(i + RESYNC_WINDOW)).find(|&l| old[l] == new[j]);
            match (inserted, removed) {
                (Some(k), Some(l)) if k - j <= l - i => j = k,
                (_, Some(l)) => i = l,
                (Some(k), None) => j = k,
                (None, None) => {
                    i += 1;
                    j += 1;
                }
            }
        }
        close(&mut open, i, j, &mut splices);
        Self { splices }
    }

    pub fn is_empty(&self) -> bool {
        self.splices.is_empty()
    }

    /// Where an unchanged old item sits in the new list; `None` when it was replaced.
    pub fn map_index(&self, old_index: usize) -> Option<usize> {
        let mut shift = 0_isize;
        for splice in &self.splices {
            if old_index < splice.old.start {
                break;
            }
            if old_index < splice.old.end {
                return None;
            }
            shift += splice.new_len as isize - splice.old.len() as isize;
        }
        Some(old_index.saturating_add_signed(shift))
    }

    /// The new index for `old_index`: its own position when unchanged, otherwise the start of
    /// the run that replaced it.
    pub fn map_index_or_run(&self, old_index: usize) -> usize {
        self.map_index(old_index).unwrap_or_else(|| {
            self.splices
                .iter()
                .find(|splice| splice.old.contains(&old_index))
                .map_or(old_index, |splice| splice.new_start)
        })
    }
}

/// A reading position: the top visible block (by index and content signature) and how far the
/// viewport top sits below that block's top, in pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScrollAnchor {
    pub block: usize,
    pub signature: u64,
    pub offset: f32,
}

impl ScrollAnchor {
    pub fn capture(block: usize, offset: f32, signatures: &[u64]) -> Option<Self> {
        let signature = *signatures.get(block)?;
        Some(Self {
            block,
            signature,
            offset: offset.max(0.0),
        })
    }

    /// Whether this is the very top of the document (nothing to restore).
    pub fn is_top(&self) -> bool {
        self.block == 0 && self.offset < 0.5
    }

    /// The block and offset to scroll to in `signatures`. `diff` maps the anchor through a
    /// reload; without it (a relaunch) the block's signature is searched near its old index.
    /// A block that changed keeps its offset at the index where it now sits.
    pub fn resolve(&self, signatures: &[u64], diff: Option<&BlockDiff>) -> Option<(usize, f32)> {
        if signatures.is_empty() {
            return None;
        }
        let expected = diff.map_or(self.block, |diff| diff.map_index_or_run(self.block));
        if signatures.get(expected) == Some(&self.signature) {
            return Some((expected, self.offset));
        }
        if let Some(found) = nearest_signature(signatures, expected, self.signature) {
            return Some((found, self.offset));
        }
        let clamped = expected.min(signatures.len() - 1);
        Some((
            clamped,
            if clamped == expected {
                self.offset
            } else {
                0.0
            },
        ))
    }
}

fn nearest_signature(signatures: &[u64], around: usize, signature: u64) -> Option<usize> {
    let around = around.min(signatures.len());
    for distance in 1..=ANCHOR_SEARCH {
        let after = around + distance;
        let before = around.checked_sub(distance);
        if after >= signatures.len() && before.is_none() {
            break;
        }
        if let Some(before) = before
            && signatures[before] == signature
        {
            return Some(before);
        }
        if signatures.get(after) == Some(&signature) {
            return Some(after);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn apply(old: &[u64], new: &[u64], diff: &BlockDiff) -> Vec<Option<u64>> {
        // Unchanged items keep their value; spliced-in items are fresh (None).
        let mut items = old.iter().copied().map(Some).collect::<Vec<_>>();
        for splice in diff.splices.iter().rev() {
            items.splice(
                splice.old.clone(),
                std::iter::repeat_n(None, splice.new_len),
            );
        }
        assert_eq!(items.len(), new.len());
        for (item, expected) in items.iter().zip(new) {
            if let Some(kept) = item {
                assert_eq!(kept, expected);
            }
        }
        items
    }

    #[test]
    fn identical_lists_need_no_splices() {
        let blocks = [1, 2, 3];
        assert!(BlockDiff::compute(&blocks, &blocks).is_empty());
    }

    #[test]
    fn insertion_above_keeps_every_other_block() {
        let old = [1, 2, 3, 4, 5];
        let new = [1, 9, 2, 3, 4, 5];
        let diff = BlockDiff::compute(&old, &new);

        assert_eq!(
            diff.splices,
            vec![Splice {
                old: 1..1,
                new_start: 1,
                new_len: 1
            }]
        );
        assert_eq!(
            apply(&old, &new, &diff)
                .iter()
                .filter(|i| i.is_none())
                .count(),
            1
        );
        assert_eq!(diff.map_index(3), Some(4));
        assert_eq!(diff.map_index(0), Some(0));
    }

    #[test]
    fn scattered_edits_become_separate_small_splices() {
        let old = (0..100).collect::<Vec<u64>>();
        let mut new = old.clone();
        new[10] = 1000;
        new.remove(50);
        new.insert(80, 2000);
        let diff = BlockDiff::compute(&old, &new);

        assert_eq!(diff.splices.len(), 3);
        let fresh = apply(&old, &new, &diff)
            .iter()
            .filter(|i| i.is_none())
            .count();
        assert_eq!(fresh, 2);
        assert_eq!(diff.map_index(10), None);
        assert_eq!(diff.map_index_or_run(10), 10);
        assert_eq!(diff.map_index(60), Some(59));
        assert_eq!(diff.map_index(90), Some(90));
    }

    #[test]
    fn wholesale_rewrite_is_one_replacement() {
        let old = (0..50).collect::<Vec<u64>>();
        let new = (100..140).collect::<Vec<u64>>();
        let diff = BlockDiff::compute(&old, &new);

        assert_eq!(diff.splices.len(), 1);
        apply(&old, &new, &diff);
    }

    #[test]
    fn anchor_follows_its_block_when_content_is_inserted_above() {
        let old = [10, 20, 30, 40];
        let new = [10, 11, 12, 20, 30, 40];
        let anchor = ScrollAnchor::capture(2, 37.0, &old).unwrap();

        let diff = BlockDiff::compute(&old, &new);
        assert_eq!(anchor.resolve(&new, Some(&diff)), Some((4, 37.0)));
        // Without a diff (relaunch), the signature is found near the saved index.
        assert_eq!(anchor.resolve(&new, None), Some((4, 37.0)));
    }

    #[test]
    fn anchor_on_an_edited_block_stays_at_its_index() {
        let old = [10, 20, 30];
        let new = [10, 21, 30];
        let anchor = ScrollAnchor::capture(1, 12.0, &old).unwrap();

        let diff = BlockDiff::compute(&old, &new);
        assert_eq!(anchor.resolve(&new, Some(&diff)), Some((1, 12.0)));
        assert_eq!(anchor.resolve(&[10], None), Some((0, 0.0)));
        assert_eq!(anchor.resolve(&[], None), None);
    }
}

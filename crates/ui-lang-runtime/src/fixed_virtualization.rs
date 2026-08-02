//! Private fixed-height virtualization machinery shared by product widgets.
//!
//! This module deliberately owns no widget roles, input bindings, messages, or
//! public API. A list and a future tree may share row windows, stable keyed
//! identity, and scroll synchronization while defining different semantics.

use std::collections::HashMap;
use std::hash::Hash;
use std::ops::Range;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct FixedRows {
    row_height: f32,
    overscan: usize,
}

impl FixedRows {
    pub(crate) const fn new(row_height: f32, overscan: usize) -> Self {
        Self {
            row_height,
            overscan,
        }
    }

    pub(crate) const fn row_height(self) -> f32 {
        self.row_height
    }

    pub(crate) fn rows_per_page(self, viewport_height: f32) -> usize {
        (viewport_height / self.row_height).floor().max(1.0) as usize
    }

    pub(crate) fn total_height(self, item_count: usize) -> f32 {
        ((item_count as f64) * f64::from(self.row_height)).min(f64::from(f32::MAX)) as f32
    }

    fn max_offset(self, item_count: usize, viewport_height: f32) -> f32 {
        (self.total_height(item_count) - viewport_height).max(0.0)
    }

    fn visible_range(self, item_count: usize, offset: f32, viewport_height: f32) -> Range<usize> {
        if item_count == 0 || viewport_height == 0.0 {
            return 0..0;
        }
        let offset = offset.clamp(0.0, self.max_offset(item_count, viewport_height));
        let first = (offset / self.row_height).floor() as usize;
        let end = ((offset + viewport_height) / self.row_height).ceil() as usize;
        first..end.min(item_count)
    }

    fn mounted_range(self, item_count: usize, offset: f32, viewport_height: f32) -> Range<usize> {
        let visible = self.visible_range(item_count, offset, viewport_height);
        if visible.is_empty() {
            return visible;
        }
        visible.start.saturating_sub(self.overscan)
            ..visible.end.saturating_add(self.overscan).min(item_count)
    }

    pub(crate) fn window(
        self,
        item_count: usize,
        offset: f32,
        viewport_height: f32,
    ) -> FixedRowWindow {
        let offset = offset.clamp(0.0, self.max_offset(item_count, viewport_height));
        let visible = self.visible_range(item_count, offset, viewport_height);
        let mounted = self.mounted_range(item_count, offset, viewport_height);
        let total_height = self.total_height(item_count);
        let top_spacer =
            (mounted.start as f64 * f64::from(self.row_height)).min(f64::from(f32::MAX)) as f32;
        let bottom_spacer =
            (total_height - (mounted.end as f64 * f64::from(self.row_height)) as f32).max(0.0);
        FixedRowWindow {
            offset,
            visible,
            mounted,
            total_height,
            top_spacer,
            bottom_spacer,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct FixedRowWindow {
    pub(crate) offset: f32,
    pub(crate) visible: Range<usize>,
    pub(crate) mounted: Range<usize>,
    pub(crate) total_height: f32,
    pub(crate) top_spacer: f32,
    pub(crate) bottom_spacer: f32,
}

#[derive(Debug)]
pub(crate) struct KeyedRows<Key> {
    local_ids: Arc<HashMap<Key, u32>>,
    next_local_id: u32,
}

impl<Key> KeyedRows<Key> {
    pub(crate) fn new(first_local_id: u32) -> Self {
        Self {
            local_ids: Arc::new(HashMap::new()),
            next_local_id: first_local_id,
        }
    }

    pub(crate) fn snapshot(&self) -> Self {
        Self {
            local_ids: Arc::clone(&self.local_ids),
            next_local_id: self.next_local_id,
        }
    }

    pub(crate) fn local_id(&self, key: &Key) -> Option<u32>
    where
        Key: Eq + Hash,
    {
        self.local_ids.get(key).copied()
    }

    pub(crate) fn local_ids(&self) -> &HashMap<Key, u32> {
        &self.local_ids
    }

    pub(crate) const fn next_local_id(&self) -> u32 {
        self.next_local_id
    }

    #[cfg(test)]
    pub(crate) fn shares_ids_with(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.local_ids, &other.local_ids)
    }
}

impl<Key> KeyedRows<Key>
where
    Key: Clone + Eq + Hash,
{
    /// Atomically replaces the key set and returns the retained key's index.
    pub(crate) fn reconcile<T>(
        &mut self,
        items: &[T],
        key: impl Fn(&T) -> Key,
        retained: Option<&Key>,
        exhausted_message: &'static str,
    ) -> Result<Option<usize>, Key> {
        let mut local_ids = HashMap::with_capacity(items.len());
        let mut retained_index = None;
        let mut next_local_id = self.next_local_id;
        for (index, item) in items.iter().enumerate() {
            let item_key = key(item);
            if local_ids.contains_key(&item_key) {
                return Err(item_key);
            }
            if retained == Some(&item_key) {
                retained_index = Some(index);
            }
            let local_id = self.local_ids.get(&item_key).copied().unwrap_or_else(|| {
                let local_id = next_local_id;
                next_local_id = next_local_id
                    .checked_add(1)
                    .unwrap_or_else(|| panic!("{exhausted_message}"));
                local_id
            });
            local_ids.insert(item_key, local_id);
        }
        self.local_ids = Arc::new(local_ids);
        self.next_local_id = next_local_id;
        Ok(retained_index)
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct FixedRowScroll {
    offset: f32,
    viewport_height: f32,
    revision: u64,
}

impl FixedRowScroll {
    pub(crate) const fn offset(self) -> f32 {
        self.offset
    }

    pub(crate) const fn viewport_height(self) -> f32 {
        self.viewport_height
    }

    pub(crate) const fn revision(self) -> u64 {
        self.revision
    }

    pub(crate) fn window(self, item_count: usize, rows: FixedRows) -> FixedRowWindow {
        rows.window(item_count, self.offset, self.viewport_height)
    }

    pub(crate) fn visible_range(self, item_count: usize, rows: FixedRows) -> Range<usize> {
        rows.visible_range(item_count, self.offset, self.viewport_height)
    }

    pub(crate) fn mounted_range(self, item_count: usize, rows: FixedRows) -> Range<usize> {
        rows.mounted_range(item_count, self.offset, self.viewport_height)
    }

    pub(crate) fn reconcile(&mut self, item_count: usize, rows: FixedRows) -> bool {
        self.set_offset(self.offset, item_count, rows, true)
    }

    pub(crate) fn set_viewport_height(
        &mut self,
        height: f32,
        item_count: usize,
        rows: FixedRows,
    ) -> bool {
        self.viewport_height = if height.is_finite() {
            height.max(0.0)
        } else {
            0.0
        };
        self.set_offset(self.offset, item_count, rows, true)
    }

    pub(crate) fn set_native_offset(
        &mut self,
        offset: f32,
        item_count: usize,
        rows: FixedRows,
    ) -> bool {
        self.set_offset(offset, item_count, rows, false)
    }

    pub(crate) fn reveal(&mut self, index: usize, item_count: usize, rows: FixedRows) -> bool {
        let top = index as f64 * f64::from(rows.row_height);
        let bottom = top + f64::from(rows.row_height);
        let offset = f64::from(self.offset);
        let viewport_bottom = offset + f64::from(self.viewport_height);
        if top < offset {
            self.set_offset(top as f32, item_count, rows, true)
        } else if bottom > viewport_bottom {
            self.set_offset(
                (bottom - f64::from(self.viewport_height)) as f32,
                item_count,
                rows,
                true,
            )
        } else {
            false
        }
    }

    pub(crate) fn scroll_to_item(
        &mut self,
        index: usize,
        item_count: usize,
        rows: FixedRows,
    ) -> bool {
        if index >= item_count {
            return false;
        }
        let offset = (index as f64 * f64::from(rows.row_height))
            .min(f64::from(rows.max_offset(item_count, self.viewport_height)))
            as f32;
        self.set_offset(offset, item_count, rows, true)
    }

    fn set_offset(
        &mut self,
        offset: f32,
        item_count: usize,
        rows: FixedRows,
        synchronize_native: bool,
    ) -> bool {
        let previous = self.offset;
        self.offset = if offset.is_finite() {
            offset.clamp(0.0, rows.max_offset(item_count, self.viewport_height))
        } else {
            0.0
        };
        let changed = self.offset != previous;
        if changed && synchronize_native {
            self.revision = self.revision.wrapping_add(1);
        }
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reference_visible(
        rows: FixedRows,
        item_count: usize,
        offset: f32,
        viewport_height: f32,
    ) -> Range<usize> {
        if item_count == 0 || viewport_height == 0.0 {
            return 0..0;
        }
        let total =
            ((item_count as f64) * f64::from(rows.row_height)).min(f64::from(f32::MAX)) as f32;
        let max_offset = (total - viewport_height).max(0.0);
        let offset = offset.clamp(0.0, max_offset);
        let first = (offset / rows.row_height).floor() as usize;
        let end = ((offset + viewport_height) / rows.row_height).ceil() as usize;
        first..end.min(item_count)
    }

    #[test]
    fn fixed_windows_match_the_previous_range_contract() {
        for row_height in [0.5, 1.0, 20.0, 31.25] {
            for overscan in [0, 1, 2, 17] {
                let rows = FixedRows::new(row_height, overscan);
                for item_count in [0, 1, 2, 10, 100_000, usize::MAX] {
                    for viewport_height in [0.0, 0.25, row_height, row_height * 3.5, f32::MAX] {
                        for offset in [0.0, 0.25, row_height, 1_000_000.0, f32::MAX] {
                            let window = rows.window(item_count, offset, viewport_height);
                            let visible =
                                reference_visible(rows, item_count, offset, viewport_height);
                            let mounted = if visible.is_empty() {
                                visible.clone()
                            } else {
                                visible.start.saturating_sub(overscan)
                                    ..visible.end.saturating_add(overscan).min(item_count)
                            };
                            assert_eq!(window.visible, visible);
                            assert_eq!(window.mounted, mounted);
                            assert!(window.top_spacer.is_finite());
                            assert!(window.bottom_spacer.is_finite());
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn keyed_reconciliation_is_atomic_and_retains_local_identity() {
        let mut keys = KeyedRows::new(2);
        assert_eq!(
            keys.reconcile(&[10, 20, 30], |key| *key, Some(&20), "exhausted"),
            Ok(Some(1))
        );
        let id_10 = keys.local_id(&10).unwrap();
        let id_20 = keys.local_id(&20).unwrap();
        let before = keys.snapshot();

        assert_eq!(
            keys.reconcile(&[30, 20, 10], |key| *key, Some(&20), "exhausted"),
            Ok(Some(1))
        );
        assert_eq!(keys.local_id(&10), Some(id_10));
        assert_eq!(keys.local_id(&20), Some(id_20));
        assert!(!keys.shares_ids_with(&before));

        let stable = keys.snapshot();
        assert_eq!(
            keys.reconcile(&[40, 40], |key| *key, None, "exhausted"),
            Err(40)
        );
        assert!(keys.shares_ids_with(&stable));
        assert_eq!(keys.next_local_id(), stable.next_local_id());
    }

    #[test]
    fn scroll_revision_changes_only_for_programmatic_native_sync() {
        let rows = FixedRows::new(20.0, 2);
        let mut scroll = FixedRowScroll::default();
        assert!(!scroll.set_viewport_height(100.0, 100, rows));
        assert_eq!(scroll.revision(), 0);
        assert!(scroll.set_native_offset(200.0, 100, rows));
        assert_eq!(scroll.offset(), 200.0);
        assert_eq!(scroll.revision(), 0);
        assert!(scroll.reveal(0, 100, rows));
        assert_eq!(scroll.offset(), 0.0);
        assert_eq!(scroll.revision(), 1);
        assert!(scroll.scroll_to_item(99, 100, rows));
        assert_eq!(scroll.offset(), 1_900.0);
        assert_eq!(scroll.revision(), 2);
        assert!(scroll.set_viewport_height(200.0, 10, rows));
        assert_eq!(scroll.offset(), 0.0);
        assert_eq!(scroll.revision(), 3);
    }
}

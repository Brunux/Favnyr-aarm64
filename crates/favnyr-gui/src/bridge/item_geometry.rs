//! Item geometry queries independent of Slint models and filesystem state.
//!
//! List bands remain the source of truth for variable-height items. Adapters
//! supply those bands directly from a slice or model, without copying the
//! collection or maintaining a second geometry cache.

use std::ops::Range;

/// A half-open vertical band in logical content coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ItemBand {
    pub top: f32,
    pub height: f32,
}

/// A half-open item rectangle in logical content coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ItemRect {
    pub left: f32,
    pub top: f32,
    pub width: f32,
    pub height: f32,
}

impl ItemRect {
    pub fn band(self) -> ItemBand {
        ItemBand {
            top: self.top,
            height: self.height,
        }
    }

    pub fn contains(self, x: f32, y: f32) -> bool {
        x >= self.left && x < self.left + self.width && y >= self.top && y < self.top + self.height
    }
}

/// Constant-size cells in row-major order. Resizing replaces these metrics,
/// without storing or rewriting a rectangle for every item in the folder.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct GridLayout {
    pub columns: usize,
    pub cell_width: f32,
    pub cell_height: f32,
    pub gap: f32,
    pub left: f32,
    pub top: f32,
}

impl GridLayout {
    /// `left` reserves the same selection gutter as the list. The right inset
    /// leaves room for the scrollbar even when the folder does not need it yet.
    pub fn fit(width: f32, image_size: f32, font_size: f32, spacing: f32, left: f32) -> Self {
        let width = finite_or(width, 1.0).max(1.0);
        let left = finite_or(left, 0.0).clamp(0.0, (width - 1.0).max(0.0));
        let gap = 8.0 + finite_or(spacing, 0.0).max(0.0);
        let available = (width - left - 12.0).max(1.0);
        let preferred = (finite_or(image_size, 88.0).max(1.0) + 16.0).max(72.0);
        let columns = ((available + gap) / (preferred + gap)).floor().max(1.0) as usize;
        let cell_width = preferred.min(available);
        let caption_height = finite_or(font_size, 13.0).max(1.0) * 2.8;
        Self {
            columns,
            cell_width,
            cell_height: cell_width + caption_height + 8.0,
            gap,
            left,
            top: gap * 0.5,
        }
    }

    pub fn rect(self, index: usize) -> ItemRect {
        ItemRect {
            left: self.left + (index % self.columns) as f32 * (self.cell_width + self.gap),
            top: self.top + (index / self.columns) as f32 * (self.cell_height + self.gap),
            width: self.cell_width,
            height: self.cell_height,
        }
    }

    /// Shared vertical queries also accept equal bands for items in one row,
    /// and empty space between successive rows.
    pub fn bands(self, count: usize) -> ItemBands<impl Fn(usize) -> ItemBand> {
        ItemBands::new(count, move |index| self.rect(index).band())
    }

    pub fn index_at(self, count: usize, x: f32, y: f32) -> Option<usize> {
        if !x.is_finite() || !y.is_finite() || x < self.left || y < self.top {
            return None;
        }
        let column = ((x - self.left) / (self.cell_width + self.gap)).floor() as usize;
        let row = ((y - self.top) / (self.cell_height + self.gap)).floor() as usize;
        if column >= self.columns {
            return None;
        }
        let index = row.checked_mul(self.columns)?.checked_add(column)?;
        (index < count && self.rect(index).contains(x, y)).then_some(index)
    }

    /// Inclusive leading edges, exclusive trailing edges, matching list
    /// rubber-band selection. Only the intersecting columns enter each span.
    pub fn selection(self, count: usize, x1: f32, y1: f32, x2: f32, y2: f32) -> GridSelection {
        let rows = self.bands(count).selection_range(y1, y2);
        let low = x1.min(x2);
        let high = x1.max(x2);
        let stride = self.cell_width + self.gap;
        let first = (((low - self.left - self.cell_width) / stride).floor() + 1.0)
            .max(0.0)
            .min(self.columns as f32) as usize;
        let end = (((high - self.left) / stride).floor() + 1.0)
            .max(0.0)
            .min(self.columns as f32) as usize;
        GridSelection {
            indices: rows,
            columns: self.columns,
            first,
            end: end.max(first),
        }
    }

    /// The middle third of a folder/application remains its file drop target.
    /// Blank row tails and cell gaps instead designate an insertion position.
    pub fn insertion_gap(self, count: usize, x: f32, y: f32, keep_center: bool) -> Option<usize> {
        if count == 0 || !x.is_finite() || !y.is_finite() || x < self.left || y < 0.0 {
            return None;
        }
        if let Some(index) = self.index_at(count, x, y) {
            let fraction = (x - self.rect(index).left) / self.cell_width;
            if keep_center && (0.33..=0.66).contains(&fraction) {
                return None;
            }
            return Some(index + usize::from(fraction >= 0.5));
        }
        let row = ((y - self.top).max(0.0) / (self.cell_height + self.gap)).floor() as usize;
        let column = (((x - self.left) / (self.cell_width + self.gap)).floor() as usize)
            .min(self.columns - 1);
        let index = row.saturating_mul(self.columns).saturating_add(column);
        let rect = self.rect(index.min(count - 1));
        if y < self.top {
            return Some(0);
        }
        if y >= rect.top + rect.height {
            return Some(
                row.saturating_add(1)
                    .saturating_mul(self.columns)
                    .min(count),
            );
        }
        Some(
            index
                .saturating_add(usize::from(x >= rect.left + rect.width))
                .min(count),
        )
    }

    /// Vertical movement keeps the column when possible. At the outer rows,
    /// it stays put; the short final row clamps to its last existing item.
    pub fn move_cursor(self, count: usize, index: usize, direction: i32) -> usize {
        if count == 0 {
            return 0;
        }
        let index = index.min(count - 1);
        match direction {
            0 if index >= self.columns => index - self.columns,
            1 if index / self.columns < (count - 1) / self.columns => {
                (index + self.columns).min(count - 1)
            }
            2 => 0,
            3 => count - 1,
            4 => index.saturating_sub(1),
            5 => (index + 1).min(count - 1),
            _ => index,
        }
    }

    /// A grid pointer identifies a column as well as a row. If it is in a gap,
    /// use a visible item instead of jumping to another part of the folder.
    pub fn capture_anchor(
        self,
        count: usize,
        top: f32,
        height: f32,
        pointer: (f32, f32),
        selected: Option<usize>,
    ) -> Option<ItemAnchor> {
        let bands = self.bands(count);
        let pointer_y = top.max(0.0) + pointer.1.clamp(0.0, (height - 0.5).max(0.0));
        let pointed = self.index_at(count, pointer.0, pointer_y);
        if let Some(index) = selected.filter(|&index| index < count) {
            let band = self.rect(index).band();
            let low = band.top.max(top);
            let high = band.bottom().min(top + height);
            if high > low {
                return bands.anchor_on_item(index, (low + high) * 0.5, top);
            }
        }
        if let Some(index) = pointed {
            return bands.anchor_on_item(index, pointer_y, top);
        }
        let visible = bands.visible_range(top, top + height.max(1.0));
        (visible.start < visible.end).then(|| {
            let band = self.rect(visible.start).band();
            bands
                .anchor_on_item(visible.start, band.top.max(top), top)
                .unwrap()
        })
    }
}

fn finite_or(value: f32, fallback: f32) -> f32 {
    if value.is_finite() { value } else { fallback }
}

/// Rectangular membership without allocating a set of every selected index.
pub(crate) struct GridSelection {
    indices: Range<usize>,
    columns: usize,
    first: usize,
    end: usize,
}

impl GridSelection {
    pub fn contains(&self, index: usize) -> bool {
        self.indices.contains(&index) && (self.first..self.end).contains(&(index % self.columns))
    }

    pub fn first(&self) -> Option<usize> {
        let index = self.indices.start + self.first;
        (self.first < self.end && self.indices.contains(&index)).then_some(index)
    }
}

impl ItemBand {
    fn bottom(self) -> f32 {
        self.top + self.height
    }
}

/// Semantic point kept at the same viewport position during a relayout.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ItemAnchor {
    pub index: usize,
    pub fraction: f32,
    pub viewport_y: f32,
}

/// Queries ordered, positive-height bands. Bands may repeat within a grid row;
/// successive rows may be separated by gaps.
/// The accessor must return the same geometry throughout each query.
pub(crate) struct ItemBands<F> {
    count: usize,
    band_at: F,
}

impl<F: Fn(usize) -> ItemBand> ItemBands<F> {
    pub fn new(count: usize, band_at: F) -> Self {
        Self { count, band_at }
    }

    fn band(&self, index: usize) -> Option<ItemBand> {
        (index < self.count).then(|| (self.band_at)(index))
    }

    fn partition_point(&self, predicate: impl Fn(ItemBand) -> bool) -> usize {
        let mut left = 0;
        let mut right = self.count;
        while left < right {
            let mid = left + (right - left) / 2;
            if predicate((self.band_at)(mid)) {
                left = mid + 1;
            } else {
                right = mid;
            }
        }
        left
    }

    pub fn index_at(&self, y: f32) -> Option<usize> {
        if y < 0.0 {
            return None;
        }
        let index = self.partition_point(|band| band.bottom() <= y);
        self.band(index)
            .filter(|band| y >= band.top && y < band.bottom())
            .map(|_| index)
    }

    /// Items intersecting a half-open visible span. An item beginning exactly
    /// at the viewport's bottom is not visible yet.
    pub fn visible_range(&self, low: f32, high: f32) -> Range<usize> {
        if high <= low {
            return 0..0;
        }
        let first = self.partition_point(|band| band.bottom() <= low);
        let end = self.partition_point(|band| band.top < high);
        first..end.max(first)
    }

    /// Selection includes the item touched by the rectangle's lower edge.
    /// Unlike a viewport, a zero-height band can therefore select an item.
    /// Reversed drag directions have exactly the same result.
    pub fn selection_range(&self, y1: f32, y2: f32) -> Range<usize> {
        let low = y1.min(y2);
        let high = y1.max(y2);
        let first = self.partition_point(|band| band.bottom() <= low);
        let end = self.partition_point(|band| band.top <= high);
        first..end.max(first)
    }

    /// One viewport of overscan before and after the visible content.
    pub fn render_range(&self, top: f32, height: f32) -> Range<usize> {
        let height = height.max(1.0);
        self.visible_range((top - height).max(0.0), top.max(0.0) + height * 2.0)
    }

    pub fn content_height(&self) -> f32 {
        self.count
            .checked_sub(1)
            .and_then(|index| self.band(index))
            .map_or(0.0, ItemBand::bottom)
    }

    fn anchor_on_item(&self, index: usize, content_y: f32, top: f32) -> Option<ItemAnchor> {
        let band = self.band(index)?;
        let fraction = if band.height > 0.0 {
            ((content_y - band.top) / band.height).clamp(0.0, 1.0)
        } else {
            0.0
        };
        Some(ItemAnchor {
            index,
            fraction,
            viewport_y: content_y - top,
        })
    }

    /// Prefer a single visible selection, then the pointer, then the viewport
    /// center. An off-screen selection must not pull the viewport away.
    pub fn capture_anchor(
        &self,
        top: f32,
        height: f32,
        pointer_y: f32,
        single_selection: Option<usize>,
    ) -> Option<ItemAnchor> {
        let top = top.max(0.0);
        let height = height.max(1.0);
        if let Some(index) = single_selection
            && let Some(band) = self.band(index)
        {
            let visible_top = band.top.max(top);
            let visible_bottom = band.bottom().min(top + height);
            if visible_bottom > visible_top {
                return self.anchor_on_item(index, (visible_top + visible_bottom) * 0.5, top);
            }
        }

        // The bottom edge may receive a pointer event but lies just outside
        // the half-open viewport. Keep that event inside the visible area.
        let pointer_y = top + pointer_y.clamp(0.0, (height - 0.5).max(0.0));
        if let Some(index) = self.index_at(pointer_y) {
            return self.anchor_on_item(index, pointer_y, top);
        }

        let center_y = top + height * 0.5;
        self.index_at(center_y)
            .and_then(|index| self.anchor_on_item(index, center_y, top))
    }

    pub fn restore_top(
        &self,
        anchor: Option<ItemAnchor>,
        fallback_top: f32,
        viewport_height: f32,
    ) -> f32 {
        let max_top = (self.content_height() - viewport_height.max(1.0)).max(0.0);
        let desired = anchor
            .and_then(|anchor| {
                self.band(anchor.index)
                    .map(|band| band.top + band.height * anchor.fraction - anchor.viewport_y)
            })
            .unwrap_or(fallback_top);
        desired.clamp(0.0, max_top)
    }
}

/// Only entering and leaving items need rendering-state notifications.
pub(crate) fn changed_indices(old: Range<usize>, new: Range<usize>) -> Vec<usize> {
    old.clone()
        .filter(|index| !new.contains(index))
        .chain(new.clone().filter(|index| !old.contains(index)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid() -> GridLayout {
        GridLayout {
            columns: 3,
            cell_width: 80.0,
            cell_height: 120.0,
            gap: 10.0,
            left: 30.0,
            top: 5.0,
        }
    }

    #[test]
    fn grid_fit_is_bounded_even_in_a_narrow_or_uninitialized_view() {
        for width in [0.0, 1.0, 24.0, 80.0, 400.0, 1_200.0] {
            for spacing in [0.0, 24.0] {
                for image in [48.0, 88.0, 256.0] {
                    let grid = GridLayout::fit(width, image, 28.0, spacing, 30.0);
                    assert!(grid.columns >= 1);
                    assert!(grid.cell_width > 0.0 && grid.cell_height > grid.cell_width);
                    let last = grid.rect(grid.columns - 1);
                    assert!(last.left + last.width <= width.max(1.0));
                    assert_eq!(grid.gap, spacing + 8.0);
                }
            }
        }
        let invalid = GridLayout::fit(f32::NAN, f32::NAN, f32::NAN, f32::NAN, f32::NAN);
        assert!(invalid.cell_height.is_finite());
        assert_eq!(invalid.columns, 1);
    }

    #[test]
    fn grid_hit_test_matches_rectangles_and_rejects_empty_cell_slots() {
        let layout = grid();
        let count = 8;
        for x in [
            -1.0, 29.0, 30.0, 109.9, 110.0, 119.9, 120.0, 199.9, 200.0, 210.0, 290.0, 500.0,
        ] {
            for y in [
                -1.0, 0.0, 5.0, 124.9, 125.0, 134.9, 135.0, 264.9, 265.0, 385.0, 500.0,
            ] {
                assert_eq!(
                    layout.index_at(count, x, y),
                    (0..count).find(|&index| layout.rect(index).contains(x, y)),
                    "point ({x}, {y})"
                );
            }
        }
        assert_eq!(layout.index_at(count, 250.0, 300.0), None);
        assert_eq!(layout.index_at(count, f32::NAN, 10.0), None);
        assert_eq!(layout.index_at(0, 30.0, 5.0), None);
    }

    #[test]
    fn grid_rubber_band_only_selects_intersecting_columns_in_either_direction() {
        let layout = grid();
        let count = 8;
        let points = [-10.0, 0.0, 30.0, 110.0, 115.0, 120.0, 200.0, 280.0, 500.0];
        for x1 in points {
            for x2 in points {
                for (y1, y2) in [
                    (5.0, 265.0),
                    (265.0, 5.0),
                    (125.0, 134.0),
                    (125.0, 135.0),
                    (300.0, 300.0),
                ] {
                    let selection = layout.selection(count, x1, y1, x2, y2);
                    let expected: Vec<_> = (0..count)
                        .filter(|&index| {
                            let rect = layout.rect(index);
                            rect.left + rect.width > x1.min(x2)
                                && rect.left <= x1.max(x2)
                                && rect.top + rect.height > y1.min(y2)
                                && rect.top <= y1.max(y2)
                        })
                        .collect();
                    assert_eq!(
                        (0..count)
                            .filter(|&index| selection.contains(index))
                            .collect::<Vec<_>>(),
                        expected
                    );
                    assert_eq!(selection.first(), expected.first().copied());
                }
            }
        }
    }

    #[test]
    fn grid_virtualization_matches_visible_rectangles_with_gaps_and_partial_rows() {
        let layout = grid();
        for count in [0, 1, 3, 8, 1_001] {
            let bands = layout.bands(count);
            for top in [
                -10.0, 0.0, 5.0, 125.0, 130.0, 135.0, 200.0, 2_340.0, 50_000.0,
            ] {
                for height in [0.0, 1.0, 10.0, 120.0, 500.0] {
                    let expected: Vec<_> = (0..count)
                        .filter(|&index| {
                            let band = layout.rect(index).band();
                            height > 0.0 && band.bottom() > top && band.top < top + height
                        })
                        .collect();
                    assert_eq!(
                        bands.visible_range(top, top + height).collect::<Vec<_>>(),
                        expected
                    );
                }
            }
        }
        let range = layout.bands(1_000_000).render_range(30_000_000.0, 600.0);
        assert!(range.len() <= 48);
    }

    #[test]
    fn grid_manual_insertion_preserves_drop_centers_and_row_boundaries() {
        let layout = grid();
        assert_eq!(layout.insertion_gap(8, 32.0, 10.0, true), Some(0));
        assert_eq!(layout.insertion_gap(8, 70.0, 10.0, true), None);
        assert_eq!(layout.insertion_gap(8, 70.0, 10.0, false), Some(1));
        assert_eq!(layout.insertion_gap(8, 108.0, 10.0, true), Some(1));
        assert_eq!(layout.insertion_gap(8, 115.0, 10.0, false), Some(1));
        assert_eq!(layout.insertion_gap(8, 300.0, 10.0, false), Some(3));
        assert_eq!(layout.insertion_gap(8, 150.0, 130.0, false), Some(3));
        assert_eq!(layout.insertion_gap(8, 35.0, 140.0, false), Some(3));
        assert_eq!(layout.insertion_gap(8, 250.0, 300.0, false), Some(8));
        assert_eq!(layout.insertion_gap(8, 35.0, 500.0, false), Some(8));
        assert_eq!(layout.insertion_gap(8, 250.0, 0.0, false), Some(0));
        assert_eq!(layout.insertion_gap(8, 20.0, 10.0, false), None);
        assert_eq!(layout.insertion_gap(0, 35.0, 10.0, false), None);
    }

    #[test]
    fn grid_keyboard_respects_outer_rows_and_the_incomplete_final_row() {
        let layout = grid();
        assert_eq!(layout.move_cursor(8, 2, 0), 2);
        assert_eq!(layout.move_cursor(8, 5, 1), 7);
        assert_eq!(layout.move_cursor(8, 6, 1), 6);
        assert_eq!(layout.move_cursor(8, 7, 0), 4);
        assert_eq!(layout.move_cursor(8, 3, 4), 2);
        assert_eq!(layout.move_cursor(8, 2, 5), 3);
        assert_eq!(layout.move_cursor(8, 0, 4), 0);
        assert_eq!(layout.move_cursor(8, 7, 5), 7);
        assert_eq!(layout.move_cursor(8, 4, 2), 0);
        assert_eq!(layout.move_cursor(8, 4, 3), 7);
    }

    #[test]
    fn grid_zoom_and_reflow_anchor_the_pointed_column_or_visible_selection() {
        let old = grid();
        let count = 300;
        let top = 520.0;
        let pointer = (250.0, 20.0);
        let anchor = old
            .capture_anchor(count, top, 400.0, pointer, None)
            .unwrap();
        assert_eq!(anchor.index, 14);
        let selected = old
            .capture_anchor(count, top, 400.0, pointer, Some(17))
            .unwrap();
        assert_eq!(selected.index, 17);
        assert_eq!(
            old.capture_anchor(count, top, 400.0, pointer, Some(0)),
            Some(anchor)
        );
        let resized = GridLayout {
            columns: 2,
            cell_width: 100.0,
            cell_height: 140.0,
            ..old
        };
        for anchor in [anchor, selected] {
            let restored = resized.bands(count).restore_top(Some(anchor), top, 400.0);
            let rect = resized.rect(anchor.index);
            assert!(
                (rect.top + rect.height * anchor.fraction - restored - anchor.viewport_y).abs()
                    < 0.01
            );
        }
        let gap_anchor = old
            .capture_anchor(count, 125.0, 300.0, (115.0, 0.0), None)
            .unwrap();
        assert_eq!(gap_anchor.index, 3);
        assert_eq!(old.capture_anchor(0, 0.0, 400.0, pointer, None), None);
    }

    fn bands(heights: &[f32]) -> Vec<ItemBand> {
        let mut top = 0.0;
        heights
            .iter()
            .map(|&height| {
                let band = ItemBand { top, height };
                top += height;
                band
            })
            .collect()
    }

    #[test]
    fn viewport_and_selection_keep_distinct_edge_rules() {
        let items = bands(&[76.0, 28.0, 76.0]);
        let geometry = ItemBands::new(items.len(), |index| items[index]);

        assert_eq!(geometry.index_at(-1.0), None);
        assert_eq!(geometry.index_at(75.5), Some(0));
        assert_eq!(geometry.index_at(76.0), Some(1));
        assert_eq!(geometry.index_at(104.0), Some(2));
        assert_eq!(geometry.index_at(180.0), None);
        assert_eq!(geometry.visible_range(0.0, 76.0), 0..1);
        assert_eq!(geometry.visible_range(76.0, 76.0), 0..0);
        assert_eq!(geometry.selection_range(0.0, 76.0), 0..2);
        assert_eq!(geometry.selection_range(76.0, 76.0), 1..2);
        assert_eq!(geometry.selection_range(104.0, 76.0), 1..3);
        assert_eq!(geometry.selection_range(76.0, 104.0), 1..3);
        assert_eq!(geometry.selection_range(-20.0, -1.0), 0..0);
        assert_eq!(geometry.selection_range(180.0, 240.0), 3..3);
    }

    #[test]
    fn queries_match_linear_intersections_across_variable_bands() {
        let heights: Vec<_> = (0..4_000)
            .map(|index| if index % 3 == 0 { 28.0 } else { 76.0 } + 3.0)
            .collect();
        let items = bands(&heights);
        let geometry = ItemBands::new(items.len(), |index| items[index]);
        let mut points = vec![
            -10.0,
            -0.5,
            geometry.content_height(),
            geometry.content_height() + 1.0,
        ];
        for band in items.iter().step_by(37) {
            points.extend([band.top, band.top + band.height * 0.5, band.bottom()]);
        }

        for y in points {
            assert_eq!(
                geometry.index_at(y),
                items
                    .iter()
                    .position(|band| band.top <= y && y < band.bottom()),
                "point {y}"
            );
            for span in [0.0, 0.5, 28.0, 76.0, 720.0] {
                let visible: Vec<_> = items
                    .iter()
                    .enumerate()
                    .filter(|(_, band)| span > 0.0 && band.bottom() > y && band.top < y + span)
                    .map(|(index, _)| index)
                    .collect();
                assert_eq!(
                    geometry.visible_range(y, y + span).collect::<Vec<_>>(),
                    visible
                );
                let selected: Vec<_> = items
                    .iter()
                    .enumerate()
                    .filter(|(_, band)| band.bottom() > y && band.top <= y + span)
                    .map(|(index, _)| index)
                    .collect();
                assert_eq!(
                    geometry.selection_range(y + span, y).collect::<Vec<_>>(),
                    selected
                );
            }
        }
    }

    #[test]
    fn distant_scroll_does_not_walk_the_whole_collection() {
        let reads = std::cell::Cell::new(0);
        let geometry = ItemBands::new(1_000_000, |index| {
            reads.set(reads.get() + 1);
            ItemBand {
                top: index as f32 * 32.0,
                height: 32.0,
            }
        });
        let top = 850_000.0 * 32.0;
        assert_eq!(geometry.index_at(top + 16.0), Some(850_000));
        assert!(reads.replace(0) <= 22);
        assert_eq!(geometry.visible_range(top, top + 640.0), 850_000..850_020);
        assert!(reads.replace(0) <= 42);
        assert_eq!(geometry.render_range(top, 640.0), 849_980..850_040);
        assert!(reads.get() <= 42);
    }

    #[test]
    fn render_window_handles_empty_short_and_zero_height_viewports() {
        let empty = ItemBands::new(0, |_| panic!("empty geometry has no bands"));
        assert_eq!(empty.index_at(0.0), None);
        assert_eq!(empty.visible_range(0.0, 900.0), 0..0);
        assert_eq!(empty.selection_range(-10.0, 900.0), 0..0);
        assert_eq!(empty.render_range(0.0, 0.0), 0..0);
        assert_eq!(empty.capture_anchor(0.0, 0.0, 0.0, None), None);
        assert_eq!(empty.restore_top(None, 100.0, 0.0), 0.0);

        let items = bands(&[28.0, 76.0, 28.0]);
        let geometry = ItemBands::new(items.len(), |index| items[index]);
        assert_eq!(geometry.render_range(0.0, 900.0), 0..3);
        assert_eq!(geometry.render_range(0.0, 0.0), 0..1);
        assert_eq!(geometry.restore_top(None, 900.0, 900.0), 0.0);
    }

    #[test]
    fn anchor_keeps_visible_selection_and_pointer_positions() {
        let items = bands(&[28.0; 100]);
        let geometry = ItemBands::new(items.len(), |index| items[index]);
        let top = 560.0;
        let selected = geometry.capture_anchor(top, 420.0, 10.0, Some(22)).unwrap();
        assert_eq!(selected.index, 22);
        assert_eq!(selected.viewport_y, 70.0);
        let pointer = geometry.capture_anchor(top, 420.0, 95.0, None).unwrap();
        assert_eq!(pointer.index, 23);
        assert_eq!(pointer.viewport_y, 95.0);
        assert_eq!(
            geometry.capture_anchor(top, 420.0, 95.0, Some(2)),
            Some(pointer)
        );

        let enlarged = bands(&[76.0; 100]);
        let enlarged_geometry = ItemBands::new(enlarged.len(), |index| enlarged[index]);
        for anchor in [selected, pointer] {
            let new_top = enlarged_geometry.restore_top(Some(anchor), top, 420.0);
            let band = enlarged[anchor.index];
            let screen_y = band.top + band.height * anchor.fraction - new_top;
            assert!((screen_y - anchor.viewport_y).abs() < 0.01);
        }
    }

    #[test]
    fn anchor_handles_partial_visibility_blank_space_and_removed_item() {
        let items = bands(&[28.0; 10]);
        let geometry = ItemBands::new(items.len(), |index| items[index]);
        let partial = geometry.capture_anchor(10.0, 100.0, 95.0, Some(0)).unwrap();
        assert_eq!(partial.index, 0);
        assert_eq!(partial.viewport_y, 9.0);
        let center = geometry.capture_anchor(0.0, 500.0, 450.0, None).unwrap();
        assert_eq!(center.index, 8);
        assert_eq!(center.viewport_y, 250.0);
        let bottom = geometry.capture_anchor(0.0, 28.0, 28.0, None).unwrap();
        assert_eq!(bottom.index, 0);
        assert_eq!(bottom.viewport_y, 27.5);
        let removed = ItemAnchor {
            index: 20,
            fraction: 0.5,
            viewport_y: 10.0,
        };
        assert_eq!(geometry.restore_top(Some(removed), 50.0, 100.0), 50.0);
        assert_eq!(geometry.restore_top(Some(removed), 500.0, 100.0), 180.0);
    }

    #[test]
    fn render_notifications_only_cover_entering_and_leaving_items() {
        assert!(changed_indices(10..20, 10..20).is_empty());
        assert_eq!(
            changed_indices(10..20, 15..25),
            vec![10, 11, 12, 13, 14, 20, 21, 22, 23, 24]
        );
        assert_eq!(changed_indices(10..20, 12..18), vec![10, 11, 18, 19]);
        assert_eq!(changed_indices(0..2, 100..102), vec![0, 1, 100, 101]);
        assert_eq!(changed_indices(10..12, 0..0), vec![10, 11]);
    }
}

//! Pure arithmetic behind a virtualized rule list (`ScrollArea::show_viewport`).
//!
//! The list has one variable-height row (the expanded accordion row); everything else has a fixed
//! height. [`build_offsets`] turns that into cumulative row tops, [`visible_range`] picks the rows
//! intersecting the viewport with a binary search, and [`scroll_offset_for`] centres a row. No
//! GUI-toolkit types.

use std::ops::Range;

/// Cumulative row tops: `len == n + 1`, `offsets[n]` is the total height. `expanded` replaces the
/// height of one row `(index, height)`.
pub fn build_offsets(n: usize, row_h: f32, expanded: Option<(usize, f32)>) -> Vec<f32> {
    let mut out = Vec::with_capacity(n + 1);
    let mut y = 0.0f32;
    out.push(y);
    for i in 0..n {
        y += match expanded {
            Some((idx, h)) if idx == i => h,
            _ => row_h,
        };
        out.push(y);
    }
    out
}

/// Rows intersecting `[top, bottom)`, partially visible rows included at both edges.
pub fn visible_range(offsets: &[f32], top: f32, bottom: f32) -> Range<usize> {
    let n = offsets.len().saturating_sub(1);
    if n == 0 {
        return 0..0;
    }
    // First row whose bottom edge is below `top`.
    let first = offsets[1..].partition_point(|&b| b <= top).min(n);
    // First row whose top edge is at or past `bottom`.
    let end = offsets[..n].partition_point(|&t| t < bottom).max(first);
    first..end
}

/// Scroll offset that centres row `idx`, clamped to `[0, max(total - viewport_h, 0)]`.
pub fn scroll_offset_for(offsets: &[f32], idx: usize, viewport_h: f32) -> f32 {
    let n = offsets.len().saturating_sub(1);
    if n == 0 {
        return 0.0;
    }
    let idx = idx.min(n - 1);
    let total = offsets[n];
    let centre = offsets[idx] + (offsets[idx + 1] - offsets[idx]) / 2.0;
    let max = (total - viewport_h).max(0.0);
    (centre - viewport_h / 2.0).clamp(0.0, max)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offsets_fixed_expanded_and_empty() {
        assert_eq!(build_offsets(3, 20.0, None), vec![0.0, 20.0, 40.0, 60.0]);
        assert_eq!(
            build_offsets(3, 20.0, Some((1, 100.0))),
            vec![0.0, 20.0, 120.0, 140.0]
        );
        assert_eq!(build_offsets(0, 20.0, None), vec![0.0]);
    }

    #[test]
    fn range_at_top_of_511_rows() {
        let o = build_offsets(511, 22.0, None);
        assert_eq!(visible_range(&o, 0.0, 600.0), 0..28);
    }

    #[test]
    fn range_in_the_middle_and_past_the_end() {
        let o = build_offsets(511, 22.0, None);
        let r = visible_range(&o, 1000.0, 1600.0);
        assert_eq!(r.start, 45);
        assert_eq!(visible_range(&o, 11000.0, 99999.0).end, 511);
    }

    #[test]
    fn partial_rows_at_both_edges_are_included() {
        let o = build_offsets(10, 20.0, None);
        // [25, 45) touches rows 1 (20..40) and 2 (40..60).
        assert_eq!(visible_range(&o, 25.0, 45.0), 1..3);
        // Exactly on a boundary: row 1 ends at 40, so [40, 60) is row 2 only.
        assert_eq!(visible_range(&o, 40.0, 60.0), 2..3);
    }

    #[test]
    fn empty_list_has_empty_range() {
        let o = build_offsets(0, 20.0, None);
        assert_eq!(visible_range(&o, 0.0, 600.0), 0..0);
    }

    #[test]
    fn range_accounts_for_an_expanded_row() {
        let o = build_offsets(5, 20.0, Some((1, 200.0)));
        // Row 1 spans 20..220; viewport 100..140 sits inside it.
        assert_eq!(visible_range(&o, 100.0, 140.0), 1..2);
    }

    #[test]
    fn scroll_centres_and_clamps() {
        let o = build_offsets(100, 20.0, None);
        // Row 50 spans 1000..1020, centre 1010; viewport 200 -> 910.
        assert_eq!(scroll_offset_for(&o, 50, 200.0), 910.0);
        assert_eq!(scroll_offset_for(&o, 0, 200.0), 0.0);
        assert_eq!(scroll_offset_for(&o, 99, 200.0), 1800.0);
        let small = build_offsets(3, 20.0, None);
        assert_eq!(scroll_offset_for(&small, 2, 200.0), 0.0);
    }

    #[test]
    fn uses_binary_search() {
        let src = include_str!("virtual_list.rs");
        assert!(src.contains("partition_point"));
    }
}

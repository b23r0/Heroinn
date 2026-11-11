use std::cmp;
use std::iter::Peekable;

use glutin::Rect;

use heroinn_shell_terminal::term::{LineDamageBounds, TermDamageIterator};

use crate::display::SizeInfo;

/// Iterator which converts `heroinn_shell_terminal` damage information into renderer damaged rects.
pub struct RenderDamageIterator<'a> {
    damaged_lines: Peekable<TermDamageIterator<'a>>,
    size_info: SizeInfo<u32>,
}

impl<'a> RenderDamageIterator<'a> {
    pub fn new(damaged_lines: TermDamageIterator<'a>, size_info: SizeInfo<u32>) -> Self {
        Self { damaged_lines: damaged_lines.peekable(), size_info }
    }

    #[inline]
    fn rect_for_line(&self, line_damage: LineDamageBounds) -> Rect {
        let size_info = &self.size_info;
        let y_top = size_info.height() - size_info.padding_y();
        let x = size_info.padding_x() + line_damage.left as u32 * size_info.cell_width();
        let y = y_top - (line_damage.line + 1) as u32 * size_info.cell_height();
        let width = (line_damage.right - line_damage.left + 1) as u32 * size_info.cell_width();
        Rect { x, y, height: size_info.cell_height(), width }
    }

    // Make sure to damage near cells to include wide chars.
    #[inline]
    fn overdamage(&self, mut rect: Rect) -> Rect {
        let size_info = &self.size_info;
        rect.x = rect.x.saturating_sub(size_info.cell_width());
        rect.width = cmp::min(size_info.width() - rect.x, rect.width + 2 * size_info.cell_width());
        rect.y = rect.y.saturating_sub(size_info.cell_height() / 2);
        rect.height = cmp::min(size_info.height() - rect.y, rect.height + size_info.cell_height());

        rect
    }
}

impl<'a> Iterator for RenderDamageIterator<'a> {
    type Item = Rect;

    fn next(&mut self) -> Option<Rect> {
        let line = self.damaged_lines.next()?;
        let mut total_damage_rect = self.overdamage(self.rect_for_line(line));

        // Merge rectangles which overlap with each other.
        while let Some(line) = self.damaged_lines.peek().copied() {
            let next_rect = self.overdamage(self.rect_for_line(line));
            if !rects_overlap(total_damage_rect, next_rect) {
                break;
            }

            total_damage_rect = merge_rects(total_damage_rect, next_rect);
            let _ = self.damaged_lines.next();
        }

        Some(total_damage_rect)
    }
}

/// Check if two given [`glutin::Rect`] overlap.
fn rects_overlap(lhs: Rect, rhs: Rect) -> bool {
    !(
        // `lhs` is left of `rhs`.
        lhs.x + lhs.width < rhs.x
        // `lhs` is right of `rhs`.
        || rhs.x + rhs.width < lhs.x
        // `lhs` is below `rhs`.
        || lhs.y + lhs.height < rhs.y
        // `lhs` is above `rhs`.
        || rhs.y + rhs.height < lhs.y
    )
}

/// Merge two [`glutin::Rect`] by producing the smallest rectangle that contains both.
#[inline]
fn merge_rects(lhs: Rect, rhs: Rect) -> Rect {
    let left_x = cmp::min(lhs.x, rhs.x);
    let right_x = cmp::max(lhs.x + lhs.width, rhs.x + rhs.width);
    let y_top = cmp::max(lhs.y + lhs.height, rhs.y + rhs.height);
    let y_bottom = cmp::min(lhs.y, rhs.y);
    Rect { x: left_x, y: y_bottom, width: right_x - left_x, height: y_top - y_bottom }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlap_detection_includes_touching_edges_and_rejects_gaps() {
        let base = Rect { x: 10, y: 10, width: 10, height: 10 };
        assert!(rects_overlap(base, Rect { x: 19, y: 19, width: 1, height: 1 }));
        assert!(rects_overlap(base, Rect { x: 20, y: 10, width: 5, height: 5 }));
        assert!(rects_overlap(base, Rect { x: 10, y: 20, width: 5, height: 5 }));
        assert!(!rects_overlap(base, Rect { x: 21, y: 10, width: 5, height: 5 }));
        assert!(!rects_overlap(base, Rect { x: 10, y: 21, width: 5, height: 5 }));
        assert!(!rects_overlap(base, Rect { x: 0, y: 0, width: 9, height: 10 }));
        assert!(!rects_overlap(base, Rect { x: 0, y: 0, width: 10, height: 9 }));
    }

    #[test]
    fn merge_returns_the_smallest_containing_rectangle() {
        let merged = merge_rects(
            Rect { x: 10, y: 20, width: 30, height: 40 },
            Rect { x: 5, y: 10, width: 10, height: 15 },
        );
        assert_eq!(merged, Rect { x: 5, y: 10, width: 35, height: 50 });
    }
}

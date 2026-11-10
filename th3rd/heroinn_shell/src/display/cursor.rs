//! Convert a cursor into an iterator of rects.

use alacritty_terminal::ansi::CursorShape;
use alacritty_terminal::term::color::Rgb;

use crate::display::content::RenderableCursor;
use crate::display::SizeInfo;
use crate::renderer::rects::RenderRect;

/// Trait for conversion into the iterator.
pub trait IntoRects {
    /// Consume the cursor for an iterator of rects.
    fn rects(self, size_info: &SizeInfo, thickness: f32) -> CursorRects;
}

impl IntoRects for RenderableCursor {
    fn rects(self, size_info: &SizeInfo, thickness: f32) -> CursorRects {
        let point = self.point();
        let x = point.column.0 as f32 * size_info.cell_width() + size_info.padding_x();
        let y = point.line as f32 * size_info.cell_height() + size_info.padding_y();

        let mut width = size_info.cell_width();
        let height = size_info.cell_height();

        let thickness = (thickness * width).round().max(1.);

        if self.is_wide() {
            width *= 2.;
        }

        match self.shape() {
            CursorShape::Beam => beam(x, y, height, thickness, self.color()),
            CursorShape::Underline => underline(x, y, width, height, thickness, self.color()),
            CursorShape::HollowBlock => hollow(x, y, width, height, thickness, self.color()),
            _ => CursorRects::default(),
        }
    }
}

/// Cursor rect iterator.
#[derive(Default)]
pub struct CursorRects {
    rects: [Option<RenderRect>; 4],
    index: usize,
}

impl From<RenderRect> for CursorRects {
    fn from(rect: RenderRect) -> Self {
        Self { rects: [Some(rect), None, None, None], index: 0 }
    }
}

impl Iterator for CursorRects {
    type Item = RenderRect;

    fn next(&mut self) -> Option<Self::Item> {
        let rect = self.rects.get_mut(self.index)?;
        self.index += 1;
        rect.take()
    }
}

/// Create an iterator yielding a single beam rect.
fn beam(x: f32, y: f32, height: f32, thickness: f32, color: Rgb) -> CursorRects {
    RenderRect::new(x, y, thickness, height, color, 1.).into()
}

/// Create an iterator yielding a single underline rect.
fn underline(x: f32, y: f32, width: f32, height: f32, thickness: f32, color: Rgb) -> CursorRects {
    let y = y + height - thickness;
    RenderRect::new(x, y, width, thickness, color, 1.).into()
}

/// Create an iterator yielding a rect for each side of the hollow block cursor.
fn hollow(x: f32, y: f32, width: f32, height: f32, thickness: f32, color: Rgb) -> CursorRects {
    let top_line = RenderRect::new(x, y, width, thickness, color, 1.);

    let vertical_y = y + thickness;
    let vertical_height = height - 2. * thickness;
    let left_line = RenderRect::new(x, vertical_y, thickness, vertical_height, color, 1.);

    let bottom_y = y + height - thickness;
    let bottom_line = RenderRect::new(x, bottom_y, width, thickness, color, 1.);

    let right_x = x + width - thickness;
    let right_line = RenderRect::new(right_x, vertical_y, thickness, vertical_height, color, 1.);

    CursorRects {
        rects: [Some(top_line), Some(bottom_line), Some(left_line), Some(right_line)],
        index: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const COLOR: Rgb = Rgb { r: 1, g: 2, b: 3 };

    fn coords(rect: RenderRect) -> (f32, f32, f32, f32, f32) {
        (rect.x, rect.y, rect.width, rect.height, rect.alpha)
    }

    #[test]
    fn beam_is_a_single_full_height_rect() {
        let mut rects = beam(10.0, 20.0, 30.0, 2.0, COLOR);
        let rect = rects.next().unwrap();
        assert_eq!(coords(rect), (10.0, 20.0, 2.0, 30.0, 1.0));
        assert_eq!(rect.color, COLOR);
        assert!(rects.next().is_none());
    }

    #[test]
    fn underline_is_anchored_to_the_cell_bottom() {
        let rect = underline(10.0, 20.0, 30.0, 40.0, 2.0, COLOR).next().unwrap();
        assert_eq!(coords(rect), (10.0, 58.0, 30.0, 2.0, 1.0));
        assert_eq!(rect.color, COLOR);
    }

    #[test]
    fn hollow_cursor_emits_four_non_overlapping_edges() {
        let rects = hollow(10.0, 20.0, 40.0, 30.0, 2.0, COLOR).collect::<Vec<_>>();
        assert_eq!(rects.len(), 4);
        assert_eq!(coords(rects[0]), (10.0, 20.0, 40.0, 2.0, 1.0));
        assert_eq!(coords(rects[1]), (10.0, 48.0, 40.0, 2.0, 1.0));
        assert_eq!(coords(rects[2]), (10.0, 22.0, 2.0, 26.0, 1.0));
        assert_eq!(coords(rects[3]), (48.0, 22.0, 2.0, 26.0, 1.0));
        assert!(rects.iter().all(|rect| rect.color == COLOR));
    }

    #[test]
    fn cursor_rect_iterator_is_empty_by_default_and_single_rect_from_one() {
        let mut empty = CursorRects::default();
        assert!(empty.next().is_none());
        let rect = RenderRect::new(1.0, 2.0, 3.0, 4.0, COLOR, 0.5);
        let mut single = CursorRects::from(rect);
        assert_eq!(coords(single.next().unwrap()), coords(rect));
        assert!(single.next().is_none());
    }
}

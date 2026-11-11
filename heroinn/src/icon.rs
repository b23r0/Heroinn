//! Runtime and executable icon derived from `image/heroinn-mark.svg`.

use eframe::egui;

pub(crate) const RUNTIME_ICON_PNG: &[u8] = include_bytes!("../resources/heroinn-128.png");

pub(crate) fn runtime_icon() -> egui::IconData {
    let image = image::load_from_memory(RUNTIME_ICON_PNG)
        .expect("embedded Heroinn runtime icon must decode")
        .to_rgba8();
    let (width, height) = image.dimensions();

    egui::IconData {
        rgba: image.into_raw(),
        width,
        height,
    }
}

#[cfg(test)]
mod tests {
    use super::{RUNTIME_ICON_PNG, runtime_icon};

    #[test]
    fn runtime_icon_is_transparent_multi_pixel_asset() {
        let image = image::load_from_memory(RUNTIME_ICON_PNG)
            .expect("decode embedded PNG")
            .to_rgba8();
        assert_eq!(image.dimensions(), (128, 128));
        assert!(image.pixels().any(|pixel| pixel[3] == 0));
        assert!(image.pixels().any(|pixel| pixel[3] != 0));
    }

    #[test]
    fn egui_icon_has_matching_rgba_buffer() {
        let icon = runtime_icon();
        assert_eq!(icon.width, 128);
        assert_eq!(icon.height, 128);
        assert_eq!(
            icon.rgba.len(),
            (icon.width as usize) * (icon.height as usize) * 4
        );
    }
}

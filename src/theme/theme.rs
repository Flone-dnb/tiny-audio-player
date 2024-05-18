use iced::{theme::Palette, Color};

pub fn dark_orange_palette() -> Palette {
    Palette {
        background: Color::from_rgb8(30, 30, 30),
        text: Color::from_rgb8(75, 75, 75),
        primary: Color::from_rgb8(160, 90, 26),
        success: Color::from_rgb8(41, 245, 177),
        danger: Color::from_rgb8(119, 53, 24),
    }
}

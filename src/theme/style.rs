use iced::{theme::Palette, Color};

pub fn dark_orange_palette() -> Palette {
    Palette {
        background: Color::from_rgb8(30, 30, 30),
        text: Color::from_rgb8(75, 75, 75),
        primary: get_primary_color(),
        success: Color::from_rgb8(41, 245, 177),
        danger: Color::from_rgb8(119, 53, 24),
    }
}

pub fn get_primary_color() -> Color {
    Color::from_rgb8(155, 65, 0)
}

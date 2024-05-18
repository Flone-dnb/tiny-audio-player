#![deny(warnings)]
#![windows_subsystem = "windows"] // don't show a console when opening the app on windows

use app::application::ApplicationState;
use iced::{
    window::{self, Position},
    Application, Size,
};

mod app;
mod audio;
mod layouts;
mod misc;
mod theme;

fn main() -> iced::Result {
    // Prepare initial window size.
    let window_size = Size {
        width: 700,
        height: 400,
    };

    // Prepare window settings.
    let window_settings = window::Settings {
        size: Size::new(window_size.width as f32, window_size.height as f32),
        position: Position::Centered,
        ..window::Settings::default()
    };

    // Run app.
    ApplicationState::run(iced::Settings {
        antialiasing: true,
        window: window_settings,
        ..iced::Settings::default()
    })
}

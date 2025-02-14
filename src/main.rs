#![windows_subsystem = "windows"] // don't show a console when opening the app on windows

use app::application::ApplicationState;
use iced::Font;

mod app;
mod audio;
mod layouts;
mod misc;
mod theme;
mod widgets;

fn main() -> iced::Result {
    iced::application(
        ApplicationState::title,
        ApplicationState::update,
        ApplicationState::view,
    )
    .subscription(ApplicationState::subscription)
    .theme(ApplicationState::theme)
    .window_size((700.0, 400.0))
    .antialiasing(true)
    .centered()
    .run_with(ApplicationState::new)
}

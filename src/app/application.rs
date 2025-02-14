use super::process_message_listener::ProcessMessageListener;
use crate::layouts::main_layout::*;
use iced::{event, window, Element, Event, Renderer, Subscription, Task, Theme};
use std::path::PathBuf;
use std::time::Instant;

/// Send refresh UI messages every N seconds.
const APP_VISUAL_UPDATE_INTERVAL_SEC: u64 = 1;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Layout {
    Main,
}

#[derive(Debug, Clone)]
pub enum ApplicationMessage {
    MainLayoutMessage(MainLayoutMessage),
    OsEvent(Event),
    VisualUpdate(Instant),
}

pub struct ApplicationState {
    current_layout: Layout,

    main_layout: MainLayout,

    process_message_listener: ProcessMessageListener,
}

impl ApplicationState {
    pub fn new() -> (Self, Task<ApplicationMessage>) {
        let listener = ProcessMessageListener::new();
        if listener.is_none() {
            // Exit.
            std::process::exit(0);
        }

        (
            Self {
                current_layout: Layout::Main,
                main_layout: MainLayout::new(),
                process_message_listener: listener.unwrap(),
            },
            Task::none(),
        )
    }

    pub fn theme(&self) -> Theme {
        iced::theme::Theme::Custom(
            iced::theme::Custom::new(
                "Dark Orange".to_string(),
                crate::theme::style::dark_orange_palette(),
            )
            .into(),
        )
    }

    pub fn title(&self) -> String {
        format!("Tiny Audio Player v{}", env!("CARGO_PKG_VERSION"))
    }

    pub fn view(&self) -> Element<ApplicationMessage, Theme, Renderer> {
        match self.current_layout {
            Layout::Main => self
                .main_layout
                .view()
                .map(ApplicationMessage::MainLayoutMessage),
        }
    }

    pub fn update(&mut self, message: ApplicationMessage) -> Task<ApplicationMessage> {
        match message {
            ApplicationMessage::MainLayoutMessage(message) => self.main_layout.update(message),
            ApplicationMessage::OsEvent(os_event) => match os_event {
                Event::Window(event) => {
                    if let window::Event::FileHovered(_) = event {
                        return Task::none();
                    }

                    if let window::Event::FileDropped(path) = event {
                        return self
                            .main_layout
                            .update(MainLayoutMessage::FileDropped(path));
                    }

                    Task::none()
                }
                _ => Task::none(),
            },
            ApplicationMessage::VisualUpdate(_) => {
                let paths = self.process_message_listener.process_messages();
                for path in paths {
                    self.main_layout
                        .try_importing_track_from_path(PathBuf::from(path).as_path());
                }
                Task::none()
            }
        }
    }

    pub fn subscription(&self) -> Subscription<ApplicationMessage> {
        let tick = iced::time::every(std::time::Duration::from_secs(
            APP_VISUAL_UPDATE_INTERVAL_SEC,
        ))
        .map(ApplicationMessage::VisualUpdate);

        Subscription::batch(vec![tick, event::listen().map(ApplicationMessage::OsEvent)])
    }
}

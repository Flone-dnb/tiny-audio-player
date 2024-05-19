use iced::{
    event, executor, window, Application, Command, Element, Event, Renderer, Subscription, Theme,
};

use crate::layouts::main_layout::*;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Layout {
    Main,
}

#[derive(Debug, Clone)]
pub enum ApplicationMessage {
    MainLayoutMessage(MainLayoutMessage),
    OsEvent(Event),
}

pub struct ApplicationState {
    current_layout: Layout,

    main_layout: MainLayout,
}

impl Application for ApplicationState {
    type Message = ApplicationMessage;
    type Theme = Theme;
    type Executor = executor::Default;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<ApplicationMessage>) {
        (
            Self {
                current_layout: Layout::Main,
                main_layout: MainLayout::new(),
            },
            Command::none(),
        )
    }

    fn theme(&self) -> Theme {
        iced::theme::Theme::Custom(
            iced::theme::Custom::new(
                "Dark Orange".to_string(),
                crate::theme::style::dark_orange_palette(),
            )
            .into(),
        )
    }

    fn title(&self) -> String {
        format!("Tiny Audio Player v{}", env!("CARGO_PKG_VERSION"))
    }

    fn view(&self) -> Element<ApplicationMessage, Theme, Renderer> {
        match self.current_layout {
            Layout::Main => self
                .main_layout
                .view()
                .map(ApplicationMessage::MainLayoutMessage),
        }
    }

    fn update(&mut self, message: ApplicationMessage) -> Command<ApplicationMessage> {
        match message {
            ApplicationMessage::MainLayoutMessage(message) => self.main_layout.update(message),
            ApplicationMessage::OsEvent(os_event) => match os_event {
                Event::Window(_, event) => {
                    if let window::Event::FileHovered(_) = event {
                        return Command::none();
                    }

                    if let window::Event::FileDropped(path) = event {
                        return self
                            .main_layout
                            .update(MainLayoutMessage::FileDropped(path));
                    }

                    Command::none()
                }
                _ => Command::none(),
            },
        }
    }

    fn subscription(&self) -> Subscription<ApplicationMessage> {
        event::listen().map(ApplicationMessage::OsEvent)
    }
}

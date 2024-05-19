use iced::advanced::graphics::core::event;
use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer;
use iced::advanced::widget::{self, Widget};
use iced::{mouse, Element, Event, Shadow};
use iced::{Border, Color, Length, Rectangle, Size};
use std::rc::Rc;

use crate::theme;

pub struct TrackPosSlider<Message> {
    audio_data: Rc<Vec<i8>>,
    current_pos_portion: f64,
    on_clicked: Option<Box<dyn FnMut(f32) -> Message>>,
}

impl<Message> TrackPosSlider<Message> {
    pub fn new(audio_data: Rc<Vec<i8>>, current_pos_portion: f64) -> Self {
        Self {
            audio_data,
            current_pos_portion,
            on_clicked: None,
        }
    }

    #[must_use]
    pub fn on_clicked<CB: 'static + Fn(f32) -> Message>(mut self, callback: CB) -> Self {
        self.on_clicked = Some(Box::new(callback));
        self
    }
}

impl<'a, Message, Theme, Renderer> Widget<Message, Theme, Renderer> for TrackPosSlider<Message>
where
    Renderer: renderer::Renderer,
{
    fn size(&self) -> Size<Length> {
        Size {
            width: Length::Fill,
            height: Length::Fill,
        }
    }

    fn layout(
        &self,
        _tree: &mut widget::Tree,
        _renderer: &Renderer,
        _limits: &layout::Limits,
    ) -> layout::Node {
        layout::Node::new(_limits.max())
    }

    fn on_event(
        &mut self,
        _state: &mut widget::Tree,
        event: iced::Event,
        layout: Layout<'_>,
        cursor: iced::advanced::mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn iced::advanced::Clipboard,
        shell: &mut iced::advanced::Shell<'_, Message>,
        _viewport: &Rectangle,
    ) -> iced::advanced::graphics::core::event::Status {
        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if let Some(on_clicked) = self.on_clicked.as_mut() {
                    if let Some(relative_pos) = cursor.position_in(layout.bounds()) {
                        shell.publish(on_clicked(relative_pos.x / layout.bounds().width));
                    }
                }
            }
            _ => {}
        }

        return event::Status::Ignored;
    }

    fn draw(
        &self,
        _state: &widget::Tree,
        renderer: &mut Renderer,
        _theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        let layout_bounds = layout.bounds();
        let step_width = layout_bounds.width / self.audio_data.len() as f32;

        // Draw wave.
        for (i, sample) in self.audio_data.iter().enumerate() {
            let portion = sample.abs() as f32 / i8::MAX as f32;
            let sample_height = layout_bounds.height * portion;

            // Draw a quad that represents this "sample".
            renderer.fill_quad(
                renderer::Quad {
                    bounds: Rectangle {
                        x: layout_bounds.x + step_width * i as f32,
                        y: layout_bounds.y + layout_bounds.height - sample_height,
                        width: step_width,
                        height: sample_height,
                    },
                    border: Border {
                        radius: 0.0.into(),
                        width: 0.0,
                        color: Color::from_rgb(0.0, 0.0, 0.0),
                    },
                    shadow: Shadow::default(),
                },
                theme::theme::get_primary_color(),
            );
        }

        // Draw current position quad.
        renderer.fill_quad(
            renderer::Quad {
                bounds: Rectangle {
                    x: layout_bounds.x,
                    y: layout_bounds.y,
                    width: layout_bounds.width * self.current_pos_portion as f32,
                    height: layout_bounds.height,
                },
                border: Border {
                    radius: 0.0.into(),
                    width: 0.0,
                    color: Color::from_rgb(0.0, 0.0, 0.0),
                },
                shadow: Shadow::default(),
            },
            Color {
                a: 0.5,
                ..Color::BLACK
            },
        );
    }
}

impl<'a, Message> From<TrackPosSlider<Message>> for Element<'a, Message>
where
    Message: 'a + Clone,
{
    fn from(slider: TrackPosSlider<Message>) -> Self {
        Self::new(slider)
    }
}

use crate::{
    app::application::ApplicationMessage,
    audio::audio_player::AudioPlayer,
    misc::{
        config_manger::{ConfigManager, TracklistConfig, TRACKLIST_EXTENSION},
        settings::*,
    },
    widgets::track_pos_slider::TrackPosSlider,
};
use iced::widget::svg;
use iced::{
    alignment::{Horizontal, Vertical},
    widget::{container, Button, Column, Container, MouseArea, Row, Scrollable, Slider, Text},
    Background, Border, Color, Command, Element, Length, Renderer, Theme,
};
use native_dialog::{FileDialog, MessageDialog, MessageType};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

// Layout customization.
const TITLE_BLOCK_PORTION: u16 = 7;
const PLAYBACK_RATE_BLOCK_PORTION: u16 = 4;
const VOLUME_BLOCK_PORTION: u16 = 4;
const TRACK_POS_HEIGHT_PORTION: u16 = 2;
const TRACKLIST_HEIGHT_PORTION: u16 = 7;
const WIDGET_BACKGROUND_DARK_ALPHA: f32 = 0.4;

#[derive(Debug, Clone)]
pub enum MainLayoutMessage {
    VolumeChanged(f64),
    PlaybackRateChanged(f64),
    PlayTrackFromStart(usize),
    DeleteTrack(usize),
    ChangeTrackPos(f32),
    MoveTrackUp(usize),
    MoveTrackDown(usize),
    PlayPauseCurrentTrack,
    OpenTracklist,
    SaveTracklist,
    FileDropped(PathBuf),
}

pub struct MainLayout {
    audio_player: Arc<Mutex<AudioPlayer>>,
}

impl MainLayout {
    pub fn new() -> Self {
        Self {
            audio_player: AudioPlayer::new(),
        }
    }

    pub fn view(&self) -> Element<MainLayoutMessage, Theme, Renderer> {
        let audio_player = self.audio_player.lock().unwrap();

        // Prepare top block.
        let top_block = Row::new()
            .push(
                Column::new()
                    .push(
                        Text::new({
                            match audio_player.get_current_track_index() {
                                None => "".to_string(),
                                Some(index) => audio_player.get_tracklist()[index].name.clone(),
                            }
                        })
                        .size(TEXT_SIZE),
                    )
                    .spacing(VERTICAL_ELEMENT_SPACING)
                    .push(
                        Text::new(format!(
                            "Time: {}:{} / {}:{}",
                            audio_player.get_current_sound_position() as usize / 60,
                            audio_player.get_current_sound_position() as usize % 60,
                            audio_player.get_current_sound_duration() as usize / 60,
                            audio_player.get_current_sound_duration() as usize % 60
                        ))
                        .size(TEXT_SIZE),
                    )
                    .width(Length::FillPortion(TITLE_BLOCK_PORTION)),
            )
            .spacing(HORIZONTAL_ELEMENT_SPACING)
            .push(
                Column::new()
                    .push(
                        Text::new(format!(
                            "Playback Rate: x{:.2}",
                            audio_player.get_playback_rate()
                        ))
                        .size(TEXT_SIZE)
                        .vertical_alignment(Vertical::Center),
                    )
                    .spacing(VERTICAL_ELEMENT_SPACING)
                    .push(
                        Slider::new(
                            0.4..=1.4,
                            audio_player.get_playback_rate(),
                            MainLayoutMessage::PlaybackRateChanged,
                        )
                        .step(0.01),
                    )
                    .width(Length::FillPortion(PLAYBACK_RATE_BLOCK_PORTION)),
            )
            .spacing(HORIZONTAL_ELEMENT_SPACING)
            .push(
                Column::new()
                    .push(
                        Text::new(format!("Volume: {:.0}%", audio_player.get_volume() * 100.0))
                            .size(TEXT_SIZE)
                            .vertical_alignment(Vertical::Center),
                    )
                    .spacing(VERTICAL_ELEMENT_SPACING)
                    .push(
                        Slider::new(
                            0.0..=1.25,
                            audio_player.get_volume(),
                            MainLayoutMessage::VolumeChanged,
                        )
                        .step(0.01),
                    )
                    .width(Length::FillPortion(VOLUME_BLOCK_PORTION)),
            );

        // Prepare track position block.
        let track_pos_block = Container::new(
            TrackPosSlider::new(self.audio_player.clone())
                .on_clicked(MainLayoutMessage::ChangeTrackPos),
        )
        .padding(1)
        .style(container::Appearance {
            background: Some(Background::Color(Color {
                a: WIDGET_BACKGROUND_DARK_ALPHA,
                ..Color::BLACK
            })),
            border: Border {
                color: crate::theme::style::get_primary_color(),
                width: 1.0,
                radius: BORDER_RADIUS.into(),
            },
            ..container::Appearance::default()
        })
        .width(Length::Fill)
        .height(Length::FillPortion(TRACK_POS_HEIGHT_PORTION));

        let play_pause_svg_handle =
            svg::Handle::from_path(format!("{}/res/play-pause.svg", env!("CARGO_MANIFEST_DIR")));

        // Prepare block above tracklist.
        let above_tracklist_block = Column::new()
            .push(
                Row::new()
                    .push(
                        Button::new(
                            Text::new("Save Tracklist")
                                .horizontal_alignment(Horizontal::Center)
                                .size(TEXT_SIZE),
                        )
                        .height(Length::FillPortion(1))
                        .width(Length::FillPortion(5))
                        .on_press(MainLayoutMessage::SaveTracklist),
                    )
                    .spacing(HORIZONTAL_ELEMENT_SPACING / 4)
                    .push(
                        Button::new(
                            svg(play_pause_svg_handle)
                                .width(Length::FillPortion(1))
                                .height(Length::FillPortion(1))
                                .content_fit(iced::ContentFit::ScaleDown),
                        )
                        .on_press(MainLayoutMessage::PlayPauseCurrentTrack),
                    )
                    .spacing(HORIZONTAL_ELEMENT_SPACING / 4)
                    .push(
                        Button::new(
                            Text::new("Open Tracklist")
                                .horizontal_alignment(Horizontal::Center)
                                .size(TEXT_SIZE),
                        )
                        .height(Length::FillPortion(1))
                        .width(Length::FillPortion(5))
                        .on_press(MainLayoutMessage::OpenTracklist),
                    ),
            )
            .height(Length::Fixed(29.0));

        // Prepare tracklist.
        let mut tracklist_column = Column::new();
        for (id, track) in audio_player.get_tracklist().iter().enumerate() {
            tracklist_column = tracklist_column
                .push(
                    Row::new()
                        .push(
                            Button::new(Text::new("<").size(TEXT_SIZE))
                                .on_press(MainLayoutMessage::MoveTrackUp(id)),
                        )
                        .spacing(HORIZONTAL_ELEMENT_SPACING / 4)
                        .push(
                            MouseArea::new(
                                Button::new(Text::new(track.name.clone()).size(TEXT_SIZE))
                                    .width(Length::Fill)
                                    .on_press(MainLayoutMessage::PlayTrackFromStart(id)),
                            )
                            .on_right_press(MainLayoutMessage::DeleteTrack(id)),
                        )
                        .spacing(HORIZONTAL_ELEMENT_SPACING / 4)
                        .push(
                            Button::new(Text::new(">").size(TEXT_SIZE))
                                .on_press(MainLayoutMessage::MoveTrackDown(id)),
                        ),
                )
                .spacing(VERTICAL_ELEMENT_SPACING);
        }
        let tracklist_block =
            Container::new(Scrollable::new(tracklist_column.padding(10)).height(Length::Fill))
                .style(container::Appearance {
                    background: Some(Background::Color(Color {
                        a: WIDGET_BACKGROUND_DARK_ALPHA,
                        ..Color::BLACK
                    })),
                    border: Border {
                        color: crate::theme::style::get_primary_color(),
                        width: 1.0,
                        radius: BORDER_RADIUS.into(),
                    },
                    ..container::Appearance::default()
                })
                .width(Length::Fill)
                .height(Length::FillPortion(TRACKLIST_HEIGHT_PORTION));

        // Construct the final layout.
        Column::new()
            .push(top_block)
            .push(track_pos_block)
            .push(above_tracklist_block)
            .push(tracklist_block)
            .spacing(VERTICAL_ELEMENT_SPACING)
            .padding(10)
            .into()
    }

    pub fn update(&mut self, message: MainLayoutMessage) -> Command<ApplicationMessage> {
        match message {
            MainLayoutMessage::VolumeChanged(new_volume) => {
                let mut audio_player = self.audio_player.lock().unwrap();
                audio_player.set_volume(new_volume);
            }
            MainLayoutMessage::PlaybackRateChanged(new_rate) => {
                let mut audio_player = self.audio_player.lock().unwrap();
                audio_player.set_playback_rate(new_rate)
            }
            MainLayoutMessage::PlayTrackFromStart(track_index) => {
                let mut audio_player = self.audio_player.lock().unwrap();
                audio_player.play_track(track_index);
            }
            MainLayoutMessage::PlayPauseCurrentTrack => {
                let mut audio_player = self.audio_player.lock().unwrap();
                if audio_player.get_current_track_index().is_some() {
                    audio_player.pause_resume();
                } else {
                    audio_player.play_track(0);
                }
            }
            MainLayoutMessage::DeleteTrack(track_index) => {
                let mut audio_player = self.audio_player.lock().unwrap();
                audio_player.remove_track(track_index);
            }
            MainLayoutMessage::ChangeTrackPos(portion) => {
                let mut audio_player = self.audio_player.lock().unwrap();

                let position = portion as f64 * audio_player.get_current_sound_duration();
                audio_player.set_current_sound_pos(position);
            }
            MainLayoutMessage::MoveTrackUp(track_index) => {
                let mut audio_player = self.audio_player.lock().unwrap();
                audio_player.move_track_up(track_index);
            }
            MainLayoutMessage::MoveTrackDown(track_index) => {
                let mut audio_player = self.audio_player.lock().unwrap();
                audio_player.move_track_down(track_index);
            }
            MainLayoutMessage::FileDropped(path) => {
                self.try_importing_track_from_path(path.as_path())
            }
            MainLayoutMessage::OpenTracklist => {
                // Ask for path.
                let path = FileDialog::new()
                    .add_filter("Tracklist", &[TRACKLIST_EXTENSION])
                    .show_open_single_file()
                    .unwrap();
                if let Some(path) = path {
                    let path = path.as_path().display().to_string();

                    // Load sound paths.
                    let config = ConfigManager::load_tracklist(&path);

                    self.clear_tracklist();

                    // Import paths.
                    for path in config.paths {
                        self.try_importing_track_from_path(PathBuf::from(path.as_str()).as_path())
                    }
                }
            }
            MainLayoutMessage::SaveTracklist => {
                let audio_player = self.audio_player.lock().unwrap();

                // Make sure the tracklist is not empty.
                if audio_player.get_tracklist().is_empty() {
                    MessageDialog::new()
                        .set_type(MessageType::Info)
                        .set_title("Info")
                        .set_text("Tracklist is empty - there is nothing to save!")
                        .show_alert()
                        .unwrap();
                    return Command::none();
                }

                // Ask for path.
                let path = FileDialog::new()
                    .add_filter("Tracklist", &[TRACKLIST_EXTENSION])
                    .show_save_single_file()
                    .unwrap();
                if let Some(path) = path {
                    let mut config = TracklistConfig::new();
                    config.paths = Vec::with_capacity(audio_player.get_tracklist().len());
                    for track_info in audio_player.get_tracklist() {
                        config.paths.push(track_info.path.clone());
                    }
                    ConfigManager::save_tracklist(&path.as_path().display().to_string(), config);
                }
            }
        }

        Command::none()
    }

    fn clear_tracklist(&mut self) {
        let mut audio_player = self.audio_player.lock().unwrap();
        audio_player.clear_tracklist();
    }

    pub fn try_importing_track_from_path(&mut self, path: &Path) {
        let mut audio_player = self.audio_player.lock().unwrap();
        audio_player.add_track(path);
    }
}

use iced::{
    alignment::{Horizontal, Vertical},
    widget::{container, Button, Column, Container, MouseArea, Row, Scrollable, Slider, Text},
    Background, Border, Color, Command, Element, Length, Renderer, Theme,
};
use native_dialog::{FileDialog, MessageDialog, MessageType};
use std::path::{Path, PathBuf};

use crate::{
    app::application::ApplicationMessage,
    audio::audio_player::AudioPlayer,
    misc::{
        config_manger::{ConfigManager, TracklistConfig, TRACKLIST_EXTENSION},
        settings::*,
    },
    widgets::track_pos_slider::TrackPosSlider,
};

// Layout customization.
const TITLE_BLOCK_PORTION: u16 = 7;
const PLAYBACK_RATE_BLOCK_PORTION: u16 = 4;
const VOLUME_BLOCK_PORTION: u16 = 4;
const TRACK_POS_HEIGHT_PORTION: u16 = 2;
const TRACKLIST_HEIGHT_PORTION: u16 = 7;
const WIDGET_BACKGROUND_DARK_ALPHA: f32 = 0.4;

struct TrackInfo {
    name: String,
    path: String,
}

#[derive(Debug, Clone)]
pub enum MainLayoutMessage {
    VolumeChanged(f64),
    PlaybackRateChanged(f64),
    PlayPauseTrack(usize),
    PlayTrackFromStart(usize),
    DeleteTrack(usize),
    ChangeTrackPos(f32),
    OpenTracklist,
    SaveTracklist,
    RedrawTrackPos,
    AddMusic,
    FileDropped(PathBuf),
}

pub struct MainLayout {
    current_track_index: Option<usize>,
    tracklist: Vec<TrackInfo>,
    audio_player: AudioPlayer,
}

impl MainLayout {
    pub fn new() -> Self {
        Self {
            current_track_index: None,
            tracklist: Vec::new(),
            audio_player: AudioPlayer::new(),
        }
    }

    pub fn view(&self) -> Element<MainLayoutMessage, Theme, Renderer> {
        // Prepare top block.
        let top_block = Row::new()
            .push(
                Column::new()
                    .push(
                        Text::new({
                            match self.current_track_index {
                                None => "",
                                Some(index) => &self.tracklist[index].name,
                            }
                        })
                        .size(TEXT_SIZE),
                    )
                    .spacing(VERTICAL_ELEMENT_SPACING)
                    .push(
                        Text::new(format!(
                            "Time: {}:{} / {}:{}",
                            self.audio_player.get_current_sound_position() as usize / 60,
                            self.audio_player.get_current_sound_position() as usize % 60,
                            self.audio_player.get_current_sound_duration() as usize / 60,
                            self.audio_player.get_current_sound_duration() as usize % 60
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
                            self.audio_player.get_playback_rate()
                        ))
                        .size(TEXT_SIZE)
                        .vertical_alignment(Vertical::Center),
                    )
                    .spacing(VERTICAL_ELEMENT_SPACING)
                    .push(
                        Slider::new(
                            0.4..=1.4,
                            self.audio_player.get_playback_rate(),
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
                        Text::new(format!(
                            "Volume: {:.0}%",
                            self.audio_player.get_volume() * 100.0
                        ))
                        .size(TEXT_SIZE)
                        .vertical_alignment(Vertical::Center),
                    )
                    .spacing(VERTICAL_ELEMENT_SPACING)
                    .push(
                        Slider::new(
                            0.0..=1.25,
                            self.audio_player.get_volume(),
                            MainLayoutMessage::VolumeChanged,
                        )
                        .step(0.01),
                    )
                    .width(Length::FillPortion(VOLUME_BLOCK_PORTION)),
            );

        // Prepare tracklist.
        let mut tracklist_column = Column::new();
        for (id, track) in self.tracklist.iter().enumerate() {
            tracklist_column = tracklist_column
                .push(
                    MouseArea::new(
                        Button::new(Text::new(track.name.as_str()).size(TEXT_SIZE))
                            .width(Length::Fill)
                            .on_press(MainLayoutMessage::PlayPauseTrack(id)),
                    )
                    .on_right_press(MainLayoutMessage::DeleteTrack(id))
                    .on_middle_press(MainLayoutMessage::PlayTrackFromStart(id)),
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

        // Prepare track position block.
        let track_pos_block = Container::new(
            TrackPosSlider::new(
                self.audio_player.get_current_sound_wave(),
                self.audio_player.get_current_sound_position()
                    / self.audio_player.get_current_sound_duration(),
            )
            .on_clicked(MainLayoutMessage::ChangeTrackPos)
            .on_redraw(|| MainLayoutMessage::RedrawTrackPos),
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

        // Prepare bottom block.
        let bottom_block = Column::new().push(
            Row::new()
                .push(
                    Button::new(
                        Text::new("Save Tracklist")
                            .horizontal_alignment(Horizontal::Center)
                            .size(TEXT_SIZE),
                    )
                    .width(Length::Fill)
                    .on_press(MainLayoutMessage::SaveTracklist),
                )
                .spacing(HORIZONTAL_ELEMENT_SPACING)
                .push(
                    Button::new(
                        Text::new("Open Tracklist")
                            .horizontal_alignment(Horizontal::Center)
                            .size(TEXT_SIZE),
                    )
                    .width(Length::Fill)
                    .on_press(MainLayoutMessage::OpenTracklist),
                ),
        );

        // Remove this block when Iced adds support for drag and drop on Linux.
        let temp_add_track_block = Button::new(
            Text::new("Drag and drop files here or click to add music...")
                .horizontal_alignment(Horizontal::Center)
                .size(TEXT_SIZE),
        )
        .width(Length::Fill)
        .on_press(MainLayoutMessage::AddMusic);

        // Construct the final layout.
        Column::new()
            .push(top_block)
            .push(track_pos_block)
            .push(tracklist_block)
            .push(bottom_block)
            .push(temp_add_track_block)
            .spacing(VERTICAL_ELEMENT_SPACING)
            .padding(10)
            .into()
    }

    pub fn update(&mut self, message: MainLayoutMessage) -> Command<ApplicationMessage> {
        match message {
            MainLayoutMessage::VolumeChanged(new_volume) => {
                self.audio_player.set_volume(new_volume)
            }
            MainLayoutMessage::PlaybackRateChanged(new_rate) => {
                self.audio_player.set_playback_rate(new_rate)
            }
            MainLayoutMessage::PlayTrackFromStart(track_index) => {
                if let Some(current_index) = self.current_track_index {
                    if current_index == track_index {
                        // Just restart.
                        self.audio_player.stop();
                        self.audio_player.play(&self.tracklist[current_index].path);
                        return Command::none();
                    }
                }

                // Play a new one.
                self.current_track_index = Some(track_index);
                self.audio_player
                    .play(self.tracklist[track_index].path.as_str());
            }
            MainLayoutMessage::PlayPauseTrack(track_index) => {
                if let Some(current_index) = self.current_track_index {
                    if current_index == track_index {
                        self.audio_player.pause_resume();

                        return Command::none();
                    }
                }

                self.current_track_index = Some(track_index);
                self.audio_player
                    .play(self.tracklist[track_index].path.as_str());
            }
            MainLayoutMessage::DeleteTrack(track_index) => {
                // Clear current index if this is the track being played.
                if let Some(current_index) = self.current_track_index {
                    if current_index == track_index {
                        self.current_track_index = None;
                        self.audio_player.stop();
                    }
                }

                // Remove from list.
                self.tracklist.remove(track_index);

                // Update current index (if deleted not the current track).
                if let Some(index) = self.current_track_index {
                    if index >= self.tracklist.len() {
                        self.current_track_index = Some(index - 1);
                    }
                }
            }
            MainLayoutMessage::ChangeTrackPos(portion) => self.audio_player.set_current_sound_pos(
                portion as f64 * self.audio_player.get_current_sound_duration(),
            ),
            MainLayoutMessage::RedrawTrackPos => {
                if let Some(mut track_index) = self.current_track_index {
                    if self.audio_player.get_current_sound_position() + 0.01
                        >= self.audio_player.get_current_sound_duration()
                    {
                        // Switch to the next track.
                        if track_index + 1 == self.tracklist.len() {
                            track_index = 0;
                        } else {
                            track_index += 1;
                        }

                        self.current_track_index = Some(track_index);

                        // Play it.
                        self.audio_player
                            .play(self.tracklist[track_index].path.as_str());
                    }
                }
            }
            MainLayoutMessage::AddMusic => {
                let paths = FileDialog::new().show_open_multiple_file().unwrap();
                for path in paths {
                    self.try_importing_track_from_path(path.as_path())
                }
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
                // Make sure the tracklist is not empty.
                if self.tracklist.is_empty() {
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
                    config.paths = Vec::with_capacity(self.tracklist.len());
                    for track_info in &self.tracklist {
                        config.paths.push(track_info.path.clone());
                    }
                    ConfigManager::save_tracklist(&path.as_path().display().to_string(), config);
                }
            }
        }

        Command::none()
    }

    fn clear_tracklist(&mut self) {
        self.audio_player.stop();
        self.current_track_index = None;
        self.tracklist.clear();
    }

    fn try_importing_track_from_path(&mut self, path: &Path) {
        // Make sure it's a file.
        if !path.is_file() {
            return;
        }

        // Get file extension.
        let file_extension = match path.extension() {
            None => return,
            Some(ext) => ext,
        };

        // Make sure it has a correct extension.
        if !AudioPlayer::is_format_supported(file_extension.to_str().unwrap()) {
            return;
        }

        self.tracklist.push(TrackInfo {
            name: path.file_stem().unwrap().to_str().unwrap().to_string(),
            path: path.display().to_string(),
        });
    }
}

use kira::{
    manager::{backend::DefaultBackend, AudioManager, AudioManagerSettings},
    sound::{
        streaming::{StreamingSoundData, StreamingSoundHandle},
        PlaybackState,
    },
    tween::Tween,
    Volume,
};
use native_dialog::MessageDialog;

pub struct AudioPlayer {
    audio_manager: AudioManager,
    current_sound: Option<StreamingSoundHandle<kira::sound::FromFileError>>,
    playback_rate: f64,
    volume: f64,
}

impl AudioPlayer {
    pub fn is_format_supported(extension: &str) -> bool {
        return extension == "mp3"
            || extension == "wav"
            || extension == "ogg"
            || extension == "flac";
    }

    pub fn new() -> Self {
        let audio_manager =
            match AudioManager::<DefaultBackend>::new(AudioManagerSettings::default()) {
                Err(msg) => {
                    MessageDialog::new()
                        .set_title("Critical error")
                        .set_text(&format!("failed to create audio manager, error: {}", msg))
                        .show_alert()
                        .unwrap();
                    panic!();
                }
                Ok(manager) => manager,
            };

        Self {
            audio_manager,
            current_sound: None,
            playback_rate: 1.0,
            volume: 1.0,
        }
    }

    pub fn play(&mut self, path: &str) {
        // Stop any sound if we are playing.
        if let Some(handle) = self.current_sound.as_mut() {
            handle.stop(Tween::default());
        }

        // Create sound data.
        let sound_data = match StreamingSoundData::from_file(path) {
            Err(msg) => {
                MessageDialog::new()
                    .set_title("Critical error")
                    .set_text(&format!("failed to create sound data, error: {}", msg))
                    .show_alert()
                    .unwrap();
                panic!();
            }
            Ok(data) => data,
        };

        // Play sound.
        self.current_sound = match self.audio_manager.play(sound_data) {
            Ok(handle) => Some(handle),
            Err(msg) => {
                MessageDialog::new()
                    .set_title("Critical error")
                    .set_text(&format!("failed to play sound data, error: {}", msg))
                    .show_alert()
                    .unwrap();
                panic!();
            }
        };

        // Set playback rate because we set it per-sound.
        self.set_playback_rate(self.playback_rate);
    }

    pub fn stop(&mut self) {
        // Quit if no sound.
        if self.current_sound.is_none() {
            return;
        }

        let sound = self.current_sound.as_mut().unwrap();

        sound.stop(Tween::default());
        self.current_sound = None;
    }

    pub fn pause_resume(&mut self) {
        // Quit if no sound.
        if self.current_sound.is_none() {
            return;
        }

        let sound = self.current_sound.as_mut().unwrap();

        if sound.state() == PlaybackState::Paused {
            sound.resume(Tween::default());
        } else {
            sound.pause(Tween::default());
        }
    }

    pub fn set_volume(&mut self, volume: f64) {
        self.volume = volume;

        self.audio_manager
            .main_track()
            .set_volume(Volume::Amplitude(volume), Tween::default())
    }

    pub fn get_volume(&self) -> f64 {
        self.volume
    }

    pub fn set_playback_rate(&mut self, rate: f64) {
        self.playback_rate = rate;

        // Quit if no sound.
        if self.current_sound.is_none() {
            return;
        }

        self.current_sound
            .as_mut()
            .unwrap()
            .set_playback_rate(rate, Tween::default());
    }

    pub fn get_playback_rate(&self) -> f64 {
        self.playback_rate
    }
}

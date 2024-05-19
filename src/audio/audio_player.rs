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
use std::rc::Rc;

pub struct CurrentSoundData {
    pub handle: StreamingSoundHandle<kira::sound::FromFileError>,
    pub wave: Rc<Vec<i8>>,
    pub duration: f64,
}

impl CurrentSoundData {
    pub fn new(handle: StreamingSoundHandle<kira::sound::FromFileError>, duration: f64) -> Self {
        // Generate some fake wave data.
        let mut wave_data: Vec<i8> = Vec::new();
        for i in 0..100 {
            wave_data.push((((i as f32 / 5.0).sin() / 2.0 + 0.5) * i8::MAX as f32) as i8);
        }

        Self {
            handle,
            wave: Rc::new(wave_data),
            duration,
        }
    }
}

pub struct AudioPlayer {
    audio_manager: AudioManager,
    current_sound: Option<CurrentSoundData>,
    playback_rate: f64,
    volume: f64,
}

impl AudioPlayer {
    pub fn is_format_supported(extension: &str) -> bool {
        extension == "mp3" || extension == "wav" || extension == "ogg" || extension == "flac"
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
        if let Some(data) = self.current_sound.as_mut() {
            data.handle.stop(Tween::default());
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

        let duration = sound_data.duration();

        // Play sound.
        self.current_sound = match self.audio_manager.play(sound_data) {
            Ok(handle) => Some(CurrentSoundData::new(handle, duration.as_secs() as f64)),
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

    pub fn get_current_sound_wave(&self) -> Rc<Vec<i8>> {
        if let Some(data) = &self.current_sound {
            return data.wave.clone();
        }

        Rc::new(Vec::new())
    }

    /// Returns the number of seconds passed since the start of the sound.
    pub fn get_current_sound_position(&self) -> f64 {
        // Quit if no sound.
        if self.current_sound.is_none() {
            return 0.0;
        }

        let sound_data = self.current_sound.as_ref().unwrap();

        sound_data.handle.position()
    }

    /// Returns length of the sound in seconds.
    pub fn get_current_sound_duration(&self) -> f64 {
        // Quit if no sound.
        if self.current_sound.is_none() {
            return 0.0;
        }

        let sound_data = self.current_sound.as_ref().unwrap();

        sound_data.duration
    }

    /// Sets position of the sound in seconds.
    pub fn set_current_sound_pos(&mut self, pos: f64) {
        // Quit if no sound.
        if self.current_sound.is_none() {
            return;
        }

        let sound_data = self.current_sound.as_mut().unwrap();

        sound_data.handle.seek_to(pos);
    }

    /// Stops the sound (if playing).
    pub fn stop(&mut self) {
        // Quit if no sound.
        if self.current_sound.is_none() {
            return;
        }

        let sound_data = self.current_sound.as_mut().unwrap();

        sound_data.handle.stop(Tween::default());
        self.current_sound = None;
    }

    /// Pauses or resumes the sound depending on its state.
    /// Does nothing if no sound is playing.
    pub fn pause_resume(&mut self) {
        // Quit if no sound.
        if self.current_sound.is_none() {
            return;
        }

        let sound_data = self.current_sound.as_mut().unwrap();

        if sound_data.handle.state() == PlaybackState::Paused {
            sound_data.handle.resume(Tween::default());
        } else {
            sound_data.handle.pause(Tween::default());
        }
    }

    /// Sets volume of the sound as a multiplier where 1.0 is "no modification to the volume".
    pub fn set_volume(&mut self, volume: f64) {
        self.volume = volume;

        self.audio_manager
            .main_track()
            .set_volume(Volume::Amplitude(volume), Tween::default())
    }

    /// Returns volume multiplier.
    pub fn get_volume(&self) -> f64 {
        self.volume
    }

    /// Sets playback speed multiplier where 1.0 is "original speed".
    pub fn set_playback_rate(&mut self, rate: f64) {
        self.playback_rate = rate;

        // Quit if no sound.
        if self.current_sound.is_none() {
            return;
        }

        self.current_sound
            .as_mut()
            .unwrap()
            .handle
            .set_playback_rate(rate, Tween::default());
    }

    /// Returns playback speed multiplier.
    pub fn get_playback_rate(&self) -> f64 {
        self.playback_rate
    }
}

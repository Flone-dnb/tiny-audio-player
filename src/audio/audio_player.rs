use kira::{
    manager::{backend::DefaultBackend, AudioManager, AudioManagerSettings},
    sound::{streaming::StreamingSoundData, PlaybackState},
    tween::Tween,
    Volume,
};
use native_dialog::MessageDialog;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use super::sound_data::CurrentSoundData;

#[derive(Clone)]
pub struct TrackInfo {
    pub name: String,
    pub path: String,
}

pub struct AudioPlayer {
    audio_manager: AudioManager,
    current_sound: Option<CurrentSoundData>,
    playback_rate: f64,
    volume: f64,
    current_track_index: Option<usize>,
    tracklist: Vec<TrackInfo>,
    track_switch_thread: Option<JoinHandle<()>>,
    stop_track_switch_thread: Arc<AtomicBool>,
}

impl Drop for AudioPlayer {
    fn drop(&mut self) {
        // Stop thread.
        self.stop_track_switch_thread.store(true, Ordering::SeqCst);
        self.track_switch_thread.take().map(JoinHandle::join);
    }
}

impl AudioPlayer {
    pub fn new() -> Arc<Mutex<Self>> {
        // Create audio manager.
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

        let stop_signal = Arc::new(AtomicBool::new(false));
        let stop_signal_clone = stop_signal.clone();

        let this = Arc::new(Mutex::new(Self {
            audio_manager,
            current_sound: None,
            playback_rate: 1.0,
            volume: 1.0,
            current_track_index: None,
            tracklist: Vec::new(),
            track_switch_thread: None,
            stop_track_switch_thread: stop_signal,
        }));

        // Spawn a thread that checks if the track is finished (since I can't find a callback in audio manager).
        let this_clone = this.clone();
        let track_switch_thread = Some(std::thread::spawn(move || {
            while !stop_signal_clone.load(Ordering::SeqCst) {
                {
                    let mut this = this_clone.lock().unwrap();

                    if let Some(sound) = this.current_sound.as_ref() {
                        if let Some(mut curren_track_index) = this.current_track_index {
                            if sound.handle.position() + 0.01 >= sound.duration {
                                // Switch to the next track.
                                if curren_track_index + 1 == this.tracklist.len() {
                                    curren_track_index = 0;
                                } else {
                                    curren_track_index += 1;
                                }

                                this.current_track_index = Some(curren_track_index);

                                // Play it.
                                this.play_track(curren_track_index);
                            }
                        }
                    }
                }

                std::thread::sleep(Duration::from_secs(1));
            }
        }));

        {
            let mut this_data = this.lock().unwrap();
            this_data.track_switch_thread = track_switch_thread;
        }

        this
    }

    pub fn is_format_supported(extension: &str) -> bool {
        extension == "mp3" || extension == "wav" || extension == "ogg" || extension == "flac"
    }

    pub fn get_current_track_index(&self) -> Option<usize> {
        self.current_track_index
    }

    pub fn get_tracklist(&self) -> &Vec<TrackInfo> {
        &self.tracklist
    }

    pub fn add_track(&mut self, track: TrackInfo) {
        self.tracklist.push(track);
    }

    pub fn clear_tracklist(&mut self) {
        self.stop();
        self.current_track_index = None;
        self.tracklist.clear();
    }

    pub fn move_track_up(&mut self, track_index: usize) {
        // Quit if only 1 track.
        if self.tracklist.len() == 1 {
            return;
        }

        let mut _target_track_index = 0;
        if track_index == 0 {
            // Swap first and last.
            _target_track_index = self.tracklist.len() - 1;
        } else {
            // Swap with upper track.
            _target_track_index = track_index - 1;
        }

        // Swap tracks.
        let temp = self.tracklist[_target_track_index].clone();
        self.tracklist[_target_track_index] = self.tracklist[track_index].clone();
        self.tracklist[track_index] = temp;

        // Update current if moved current played track.
        if let Some(current_index) = self.current_track_index {
            if current_index == track_index {
                // Moved current.
                self.current_track_index = Some(_target_track_index);
            } else if current_index == _target_track_index {
                // Moved some track to current.
                if _target_track_index == self.tracklist.len() - 1 {
                    self.current_track_index = Some(0);
                } else {
                    self.current_track_index = Some(current_index + 1);
                }
            }
        }
    }

    pub fn move_track_down(&mut self, track_index: usize) {
        // Quit if only 1 track.
        if self.tracklist.len() == 1 {
            return;
        }

        let mut _target_track_index = 0;
        if track_index == self.tracklist.len() - 1 {
            // Swap last and first.
            _target_track_index = 0;
        } else {
            // Swap with lower track.
            _target_track_index = track_index + 1;
        }

        // Swap tracks.
        let temp = self.tracklist[_target_track_index].clone();
        self.tracklist[_target_track_index] = self.tracklist[track_index].clone();
        self.tracklist[track_index] = temp;

        // Update current if moved current played track.
        if let Some(current_index) = self.current_track_index {
            if current_index == track_index {
                // Moved current.
                self.current_track_index = Some(_target_track_index);
            } else if current_index == _target_track_index {
                // Moved some track to current.
                if _target_track_index == 0 {
                    self.current_track_index = Some(self.tracklist.len() - 1);
                } else {
                    self.current_track_index = Some(current_index - 1);
                }
            }
        }
    }

    pub fn remove_track(&mut self, track_index: usize) {
        // Clear current index if this is the track being played.
        if let Some(current_index) = self.current_track_index {
            if current_index == track_index {
                self.current_track_index = None;
                self.stop();
            }
        }

        // Remove from list.
        self.tracklist.remove(track_index);

        // Update current index (if deleted not the current track).
        if let Some(index) = self.current_track_index {
            if index >= track_index {
                self.current_track_index = Some(index - 1);
            }
        }
    }

    pub fn play_track(&mut self, track_index: usize) {
        // Make sure the index is not out of bounds.
        if track_index >= self.tracklist.len() {
            return;
        }

        self.current_track_index = Some(track_index);
        self.play(&self.tracklist[track_index].path.clone());
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
            Ok(handle) => Some(CurrentSoundData::new(
                path,
                handle,
                duration.as_secs() as f64,
            )),
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

    pub fn get_current_sound_wave(&self) -> Arc<Mutex<Vec<u8>>> {
        if let Some(data) = self.current_sound.as_ref() {
            return data.wave.clone();
        }

        Arc::new(Mutex::new(Vec::new()))
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
        // Save rate.
        self.playback_rate = rate;

        // Quit if no sound.
        if self.current_sound.is_none() {
            return;
        }

        // Set playback rate.
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

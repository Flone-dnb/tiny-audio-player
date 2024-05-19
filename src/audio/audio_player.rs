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
use std::io::ErrorKind;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use symphonia::core::{audio::SampleBuffer, errors::*};

pub struct CurrentSoundData {
    pub handle: StreamingSoundHandle<kira::sound::FromFileError>,
    pub wave: Arc<Mutex<Vec<u8>>>,
    pub duration: f64,
    wave_calc_thread_handle: Option<JoinHandle<()>>,
    stop_wave_calc_signal: Arc<AtomicBool>,
}

impl Drop for CurrentSoundData {
    fn drop(&mut self) {
        self.stop_wave_calc_signal.store(true, Ordering::SeqCst);
        self.wave_calc_thread_handle.take().map(JoinHandle::join);
    }
}

impl CurrentSoundData {
    pub fn new(
        path: &str,
        handle: StreamingSoundHandle<kira::sound::FromFileError>,
        duration: f64,
    ) -> Self {
        let wave_data = Arc::new(Mutex::new(Vec::new()));
        let stop_signal = Arc::new(AtomicBool::new(false));

        // Spawn a thread that will calculate the wave.
        let wave_data_clone = wave_data.clone();
        let stop_signal_clone = stop_signal.clone();
        let path_clone = path.to_string();
        let wave_calc_thread_handle = Some(std::thread::spawn(move || {
            AudioPlayer::try_generating_wave_for_sound(
                &path_clone,
                wave_data_clone,
                stop_signal_clone,
            );
        }));

        Self {
            handle,
            wave: wave_data,
            duration,
            wave_calc_thread_handle,
            stop_wave_calc_signal: stop_signal,
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
    fn try_generating_wave_for_sound(
        path: &str,
        wave: Arc<Mutex<Vec<u8>>>,
        should_stop: Arc<AtomicBool>,
    ) {
        // Open the media source.
        let src = match std::fs::File::open(path) {
            Ok(s) => s,
            Err(msg) => {
                println!("error: {}", msg);
                return;
            }
        };

        // Create the media source stream.
        let mss = MediaSourceStream::new(Box::new(src), Default::default());

        // Create a probe hint using the file's extension. [Optional]
        let hint = Hint::new();

        // Use the default options for metadata and format readers.
        let meta_opts: MetadataOptions = Default::default();
        let fmt_opts: FormatOptions = Default::default();

        // Probe the media source.
        let probed = symphonia::default::get_probe()
            .format(&hint, mss, &fmt_opts, &meta_opts)
            .expect("unsupported format");

        // Get the instantiated format reader.
        let mut format = probed.format;

        // Find the first audio track with a known (decodeable) codec.
        let track = match format
            .tracks()
            .iter()
            .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        {
            Some(t) => t,
            None => {
                println!("unable to find a codec");
                return;
            }
        };

        // Use the default options for the decoder.
        let dec_opts: DecoderOptions = Default::default();

        // Create a decoder for the track.
        let mut decoder =
            match symphonia::default::get_codecs().make(&track.codec_params, &dec_opts) {
                Ok(d) => d,
                Err(msg) => {
                    println!("error: {}", msg);
                    return;
                }
            };

        // Store the track identifier, it will be used to filter packets.
        let track_id = track.id;

        let sample_count_to_average: usize = 50;
        let mut samples_to_average: Vec<f32> = Vec::with_capacity(sample_count_to_average);

        // The decode loop.
        loop {
            // Check if we should stop.
            if should_stop.load(Ordering::SeqCst) {
                break;
            }

            // Get the next packet from the media format.
            let packet = match format.next_packet() {
                Ok(packet) => packet,
                Err(Error::ResetRequired) => {
                    println!("unexpected reset");
                    return;
                }
                Err(Error::IoError(err)) => {
                    if err.kind() == ErrorKind::UnexpectedEof {
                        // Finished reading.
                        break;
                    }
                    println!("error: {:?}", err);
                    break;
                }
                Err(msg) => {
                    println!("error: {:?}", msg);
                    break;
                }
            };

            // Consume any new metadata that has been read since the last packet.
            while !format.metadata().is_latest() {
                // Pop the old head of the metadata queue.
                format.metadata().pop();

                // Consume the new metadata at the head of the metadata queue.
            }

            // If the packet does not belong to the selected track, skip over it.
            if packet.track_id() != track_id {
                continue;
            }

            // Decode the packet into audio samples.
            match decoder.decode(&packet) {
                Ok(decoded_packet) => {
                    let spec = *decoded_packet.spec();
                    let duration = decoded_packet.capacity() as u64;
                    let mut read_buffer = SampleBuffer::<f32>::new(duration, spec);
                    read_buffer.copy_planar_ref(decoded_packet);
                    let read_samples = read_buffer.samples();

                    // Find mean value.
                    let mut mean_value: f32 = 0.0;
                    for sample in read_samples {
                        mean_value += sample.abs();
                    }
                    mean_value = mean_value / read_samples.len() as f32;

                    // Update "processed" count.
                    samples_to_average.push(mean_value);

                    if samples_to_average.len() >= sample_count_to_average {
                        // Average all samples.
                        let mut average_value = 0.0;
                        for value in &samples_to_average {
                            average_value += value;
                        }
                        average_value = average_value / samples_to_average.len() as f32;
                        samples_to_average.clear();

                        // Add as a final sample.
                        {
                            let mut wave_data = wave.lock().unwrap();
                            wave_data.push((average_value * 2.0 * u8::MAX as f32) as u8);
                        }
                    }
                }
                Err(Error::IoError(_)) => {
                    continue;
                }
                Err(Error::DecodeError(_)) => {
                    continue;
                }
                Err(msg) => {
                    println!("error: {}", msg);
                    return;
                }
            }
        }
    }

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
        if let Some(data) = &self.current_sound {
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

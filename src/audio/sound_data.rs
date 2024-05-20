use kira::sound::streaming::StreamingSoundHandle;
use std::io::ErrorKind;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::codecs::CODEC_TYPE_NULL;
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
            Self::try_generating_wave_for_sound(&path_clone, wave_data_clone, stop_signal_clone);
        }));

        Self {
            handle,
            wave: wave_data,
            duration,
            wave_calc_thread_handle,
            stop_wave_calc_signal: stop_signal,
        }
    }

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

        let packet_count_to_average: usize = 20;
        let mut packets_to_average: Vec<f32> = Vec::with_capacity(packet_count_to_average);

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
                    mean_value /= read_samples.len() as f32;

                    // Update "processed" count.
                    packets_to_average.push(mean_value);

                    if packets_to_average.len() >= packet_count_to_average {
                        // Average all samples.
                        let mut average_value = 0.0;
                        for value in &packets_to_average {
                            average_value += value;
                        }
                        average_value /= packets_to_average.len() as f32;
                        packets_to_average.clear();

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
}

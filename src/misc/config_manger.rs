use native_dialog::{MessageDialog, MessageType};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Write;

pub const TRACKLIST_EXTENSION: &str = "tapt";

#[derive(Serialize, Deserialize, Default)]
pub struct TracklistConfig {
    pub paths: Vec<String>,
}

impl TracklistConfig {
    pub fn new() -> Self {
        Self { paths: Vec::new() }
    }
}

pub struct ConfigManager {}

impl ConfigManager {
    pub fn save_tracklist(path: &str, tracklist: TracklistConfig) {
        // Serialize to TOML.
        let toml = match toml::to_string(&tracklist) {
            Err(msg) => {
                MessageDialog::new()
                    .set_type(MessageType::Warning)
                    .set_title("Error")
                    .set_text(&format!("failed to serialize data, error: {}", msg))
                    .show_alert()
                    .unwrap();
                return;
            }
            Ok(t) => t,
        };

        // Write to file.
        let mut file = match File::create(path) {
            Err(msg) => {
                MessageDialog::new()
                    .set_type(MessageType::Warning)
                    .set_title("Error")
                    .set_text(&format!("failed to create a file, error: {}", msg))
                    .show_alert()
                    .unwrap();
                return;
            }
            Ok(f) => f,
        };

        if let Err(msg) = write!(file, "{}", toml) {
            MessageDialog::new()
                .set_type(MessageType::Warning)
                .set_title("Error")
                .set_text(&format!("failed to write to a file, error: {}", msg))
                .show_alert()
                .unwrap()
        }
    }

    pub fn load_tracklist(path: &str) -> TracklistConfig {
        // Read file.
        let file_content = match std::fs::read_to_string(path) {
            Ok(v) => v,
            Err(msg) => {
                MessageDialog::new()
                    .set_type(MessageType::Warning)
                    .set_title("Error")
                    .set_text(&format!("failed to read from a file, error: {}", msg))
                    .show_alert()
                    .unwrap();
                return TracklistConfig::default();
            }
        };

        // Deserialize.
        let config: TracklistConfig = match toml::from_str(&file_content) {
            Ok(config) => config,
            Err(msg) => {
                MessageDialog::new()
                    .set_type(MessageType::Warning)
                    .set_title("Error")
                    .set_text(&format!(
                        "failed to deserialize from a file, error: {}",
                        msg
                    ))
                    .show_alert()
                    .unwrap();
                return TracklistConfig::default();
            }
        };

        config
    }
}

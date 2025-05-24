use nih_plug::{nih_log, nih_warn};

pub struct ConfigFile;

impl ConfigFile {
    pub fn restore_last_loaded() -> Option<String> {
        if let Some(config_dir) = dirs::config_dir() {
            let path = config_dir.join(".pyo3_plugin");
            match std::fs::read_to_string(&path) {
                Ok(data) => {
                    nih_log!("Loaded last path {:?}: {:?}", path, data);
                    Some(data)
                }
                Err(e) => {
                    nih_warn!("Failed to read last loaded path at {:?}: {:?}", path, e);
                    None
                }
            }
        } else {
            None
        }
    }
    pub fn store_last_loaded(source_path: &str) {
        if let Some(config_dir) = dirs::config_dir() {
            let path = config_dir.join(".pyo3_plugin");
            let r = std::fs::write(&path, source_path);
            if let Err(e) = &r {
                nih_warn!("Failed to write last loaded path at {:?}: {:?}", path, r);
            } else {
                nih_log!("Write last loaded path at {:?}: {:?}", path, source_path);
            }
        }
    }
}

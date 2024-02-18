use nih_plug::params::persist::PersistentField;
use std::sync::Arc;

// #[derive(serde::Serialize, serde::Deserialize)]
#[derive(Clone)]
pub struct SourcePath(pub Arc<parking_lot::Mutex<String>>);
impl Default for SourcePath {
    fn default() -> Self {
        Self(Arc::new(parking_lot::Mutex::new("".to_string())))
    }
}

impl<'a> PersistentField<'a, String> for SourcePath {
    fn set(&self, new_value: String) {
        *self.0.lock() = new_value;
    }

    fn map<F, R>(&self, f: F) -> R
    where
        F: Fn(&String) -> R,
    {
        f(&self.0.lock())
    }
}

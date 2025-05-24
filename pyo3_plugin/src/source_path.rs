use nih_plug::params::persist::PersistentField;
use std::sync::Arc;

#[derive(Clone)]
pub struct PersistedSourcePath(pub Arc<parking_lot::Mutex<String>>);
impl Default for PersistedSourcePath {
    fn default() -> Self {
        Self::from_str("")
    }
}

impl PersistedSourcePath {
    pub fn from_str(s: &str) -> PersistedSourcePath {
        Self(Arc::new(parking_lot::Mutex::new(s.to_string())))
    }
}

impl<'a> PersistentField<'a, String> for PersistedSourcePath {
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

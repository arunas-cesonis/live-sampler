use pyo3::pyclass;

#[derive(Debug, Clone, Copy, PartialEq)]
#[pyclass(get_all, set_all)]
pub struct Transport {
    pub sample_rate: f32,
    pub tempo: Option<f64>,
    pub pos_samples: Option<i64>,
    pub time_sig_numerator: Option<i32>,
    pub time_sig_denominator: Option<i32>,
}

impl Default for Transport {
    fn default() -> Self {
        Self {
            sample_rate: 44100.0,
            tempo: None,
            pos_samples: None,
            time_sig_numerator: None,
            time_sig_denominator: None,
        }
    }
}

use nih_plug::nih_export_standalone;
use pyo3_plugin::PyO3Plugin;

fn main() {
    nih_export_standalone::<PyO3Plugin>();
}

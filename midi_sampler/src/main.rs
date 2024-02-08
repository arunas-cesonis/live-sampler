use nih_plug::nih_export_standalone;

use midi_sampler::MIDISampler;

fn main() {
    nih_export_standalone::<MIDISampler>();
}

use nih_plug::nih_export_standalone;

use audio_sampler::LiveSampler;

fn main() {
    nih_export_standalone::<LiveSampler>();
}

use nih_plug::nih_export_standalone;

use audio_sampler::AudioSampler;

fn main() {
    nih_export_standalone::<AudioSampler>();
}

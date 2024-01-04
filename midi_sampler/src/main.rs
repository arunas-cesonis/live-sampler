use nih_plug::nih_export_standalone;

#[cfg(not(target_env = "msvc"))]
use tikv_jemallocator::Jemalloc;

#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

use midi_sampler::MIDISampler;

fn main() {
    nih_export_standalone::<MIDISampler>();
}

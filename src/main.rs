use crossbeam_queue::{ArrayQueue, SegQueue};
use dasp::interpolate::sinc::Sinc;
use dasp::signal::interpolate::Converter;
use dasp::{ring_buffer, Signal};
use live_sampler::LiveSampler;
use nih_plug::nih_export_standalone;
use std::sync::Arc;
use std::time::Instant;
use std::vec::IntoIter;

fn main() {
    nih_export_standalone::<LiveSampler>();
}

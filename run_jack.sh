#!/bin/sh
set -eu
cargo build --package audio_sampler
export DYLD_LIBRARY_PATH='/opt/homebrew/lib/'
./target/debug/audio-sampler --backend jack --midi-input 'VirtualMidi Port1'

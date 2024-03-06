#!/bin/sh
set -eu
cargo build --package audio_sampler --release
export DYLD_LIBRARY_PATH='/opt/homebrew/lib/'
./target/release/audio-sampler --backend jack

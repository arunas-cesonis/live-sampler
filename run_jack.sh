#!/bin/sh
set -eux
cargo build --package audio_sampler --release
export DYLD_LIBRARY_PATH='/opt/homebrew/lib/'
./target/release/pyo3_plugin --backend jack

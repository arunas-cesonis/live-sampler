#!/bin/sh
set -eu
cargo build --package audio_sampler
export DYLD_LIBRARY_PATH='/opt/homebrew/lib/'
export MallocStackLogging=1
export DYLD_INSERT_LIBRARIES=/usr/lib/libgmalloc.dylib
./target/debug/audio-sampler --backend jack --midi-input 'VirtualMidi Port1'

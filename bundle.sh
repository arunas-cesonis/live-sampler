#!/bin/sh
set -eux
echo "
# Ensure nih-plug is installed:
# cargo install --git https://github.com/robbert-vdh/nih-plug.git cargo-nih-plug
"
# cargo nih-plug bundle live-sampler --release
cargo nih-plug bundle -p midi_sampler -p audio_sampler $@
#cargo nih-plug bundle midi-sampler $@
#cargo nih-plug bundle lua-plugin $@

#!/bin/sh
set -eux
echo "
# Ensure nih-plug is installed:
# cargo install --git https://github.com/robbert-vdh/nih-plug.git cargo-nih-plug
"
# cargo nih-plug bundle live-sampler --release
cargo nih-plug bundle live-sampler
cargo nih-plug bundle midi-sampler

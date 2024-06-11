#!/bin/sh
set -eux
echo "
# Ensure nih-plug is installed:
"
cargo install --git https://github.com/robbert-vdh/nih-plug.git cargo-nih-plug
cargo nih-plug bundle audio_sampler $@

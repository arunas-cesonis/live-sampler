#!/bin/sh
set -eux
echo "
# Ensure nih-plug is installed:
"
export PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1
cargo install --git https://github.com/robbert-vdh/nih-plug.git cargo-nih-plug
# cargo nih-plug bundle pyo3_plugin $@
cargo nih-plug bundle audio_sampler $@

#!/bin/sh
set -eux
cargo install --git https://github.com/robbert-vdh/nih-plug.git cargo-nih-plug
cargo nih-plug bundle pyo3_plugin $@
cargo nih-plug bundle audio_sampler $@

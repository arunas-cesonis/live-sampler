#!/bin/sh
echo "
# Ensure nih-plug is installed:
# cargo install --git https://github.com/robbert-vdh/nih-plug.git cargo-nih-plug
"
# cargo nih-plug bundle live-sampler --release
cargo nih-plug bundle live-sampler --release
sudo cp -vr ./target/bundled/live-sampler.clap /Library/Audio/Plug-Ins/CLAP/
sudo cp -vr ./target/bundled/live-sampler.vst3 /Library/Audio/Plug-Ins/VST3/




#!/bin/sh
set -eux
sh ./release.sh
sudo cp -vr ./target/bundled/live-sampler.clap /Library/Audio/Plug-Ins/CLAP/
sudo cp -vr ./target/bundled/live-sampler.vst3 /Library/Audio/Plug-Ins/VST3/
exec /Applications/REAPER.app/Contents/MacOS/REAPER

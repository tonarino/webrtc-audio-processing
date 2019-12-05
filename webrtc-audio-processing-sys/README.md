# webrtc-audio-processing-sys
[![Crates.io](https://img.shields.io/crates/v/webrtc-audio-processing-sys.svg)](https://crates.io/crates/webrtc-audio-processing-sys)
[![Docs.rs](https://docs.rs/webrtc-audio-processing-sys/badge.svg)](https://docs.rs/webrtc-audio-processing-sys)
[![Build Status](https://travis-ci.org/tonarino/webrtc-audio-processing.svg?branch=master)](https://travis-ci.org/tonarino/webrtc-audio-processing)
[![dependency status](https://deps.rs/repo/github/tonarino/webrtc-audio-processing/status.svg)](https://deps.rs/repo/github/tonarino/webrtc-audio-processing)

A wrapper around [PulseAudio's repackaging of WebRTC's AudioProcessing module](https://www.freedesktop.org/software/pulseaudio/webrtc-audio-processing/).

## Dependencies

You'll need the headers and library for PulseAudio's package installed in your OS.

### Linux

#### Arch
```sh
sudo pacman -S webrtc-audio-processing
```

#### Ubuntu/Debian
```sh
sudo apt install libwebrtc-audio-processing-dev
```

### MacOS

Build from source?

### Windows

Build from source?

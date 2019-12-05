# webrtc-audio-processing
[![Crates.io](https://img.shields.io/crates/v/webrtc-audio-processing.svg)](https://crates.io/crates/webrtc-audio-processing)
[![Docs.rs](https://docs.rs/webrtc-audio-processing/badge.svg)](https://docs.rs/webrtc-audio-processing)
[![Build Status](https://travis-ci.org/tonarino/webrtc-audio-processing.svg?branch=master)](https://travis-ci.org/tonarino/webrtc-audio-processing)
[![dependency status](https://deps.rs/repo/github/tonarino/webrtc-audio-processing/status.svg)](https://deps.rs/repo/github/tonarino/webrtc-audio-processing)

A wrapper around [PulseAudio's repackaging of WebRTC's AudioProcessing module](https://www.freedesktop.org/software/pulseaudio/webrtc-audio-processing/).

## Example Usage

See `examples/simple.rs` for an example of how to use this crate.

## Dependencies

You'll need the headers and library for PulseAudio's package installed in your OS.

### Linux

#### Arch
```sh
sudo pacman -S webrtc-audio-processing
```

#### Ubuntu/Debian
```sh
# If using the system library
sudo apt install libwebrtc-audio-processing-dev

# If building webrtc-audio-processing from source
sudo apt install autotools-dev
sudo apt install libtool
```

### MacOS

Build from source?

### Windows

Build from source?

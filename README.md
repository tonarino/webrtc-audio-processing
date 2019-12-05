# webrtc-audio-processing
[![Crates.io](https://img.shields.io/crates/v/webrtc-audio-processing.svg)](https://crates.io/crates/webrtc-audio-processing)
[![Docs.rs](https://docs.rs/webrtc-audio-processing/badge.svg)](https://docs.rs/webrtc-audio-processing)
[![Build Status](https://travis-ci.org/tonarino/webrtc-audio-processing.svg?branch=master)](https://travis-ci.org/tonarino/webrtc-audio-processing)
[![dependency status](https://deps.rs/repo/github/tonarino/webrtc-audio-processing/status.svg)](https://deps.rs/repo/github/tonarino/webrtc-audio-processing)

A wrapper around [PulseAudio's repackaging of WebRTC's AudioProcessing module](https://www.freedesktop.org/software/pulseaudio/webrtc-audio-processing/).

## Example Usage

See `examples/simple.rs` for an example of how to use this crate.

## Building

### Dynamic linking

By default the build will attempt to dynamically link with the library installed via your OS's package manager.

You can specify an include path yourself by setting the environment variable `WEBRTC_AUDIO_PROCESSING_INCLUDE`.

### Packages

```sh
sudo apt install webrtc-audio-processing-dev # Ubuntu/Debian
sudo pacman -S webrtc-audio-processing # Arch
```

### Static linking

Static linking can be enabled with the `bundler` feature flag.

# webrtc-audio-processing-sys
[![Crates.io](https://img.shields.io/crates/v/webrtc-audio-processing-sys.svg)](https://crates.io/crates/webrtc-audio-processing-sys)
[![Docs.rs](https://docs.rs/webrtc-audio-processing-sys/badge.svg)](https://docs.rs/webrtc-audio-processing-sys)
[![Build Status](https://travis-ci.org/tonarino/webrtc-audio-processing.svg?branch=master)](https://travis-ci.org/tonarino/webrtc-audio-processing)
[![dependency status](https://deps.rs/repo/github/tonarino/webrtc-audio-processing/status.svg)](https://deps.rs/repo/github/tonarino/webrtc-audio-processing)

A wrapper around [PulseAudio's repackaging of WebRTC's AudioProcessing module](https://www.freedesktop.org/software/pulseaudio/webrtc-audio-processing/).

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

Static linking can be enabled with the `bundled` feature flag.

The following tools are needed in order to use the `bundled` feature flag:

* libtool (`$ sudo apt install libtool`)
* autotools (`$ sudo apt install autotools-dev`)

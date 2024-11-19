# webrtc-audio-processing
[![Crates.io](https://img.shields.io/crates/v/webrtc-audio-processing.svg)](https://crates.io/crates/webrtc-audio-processing)
[![Docs.rs](https://docs.rs/webrtc-audio-processing/badge.svg)](https://docs.rs/webrtc-audio-processing)
[![Build Status](https://travis-ci.org/tonarino/webrtc-audio-processing.svg?branch=master)](https://travis-ci.org/tonarino/webrtc-audio-processing)
[![dependency status](https://deps.rs/repo/github/tonarino/webrtc-audio-processing/status.svg)](https://deps.rs/repo/github/tonarino/webrtc-audio-processing)

A wrapper around [PulseAudio's repackaging of WebRTC's AudioProcessing module](https://www.freedesktop.org/software/pulseaudio/webrtc-audio-processing/).

`webrtc-audio-processing` can remove echo from an audio input stream in the situation where a speaker is feeding back into a microphone, as well as noise-removal, auto-gain-control, voice-activity-detection, and more!

## Example Usage

See `examples/simple.rs` for an example of how to use this crate.

## Building

### Feature Flags

* `bundled` - Build `webrtc-audio-procesing` from the included C++ code
* `derive_serde` - Derive `serialize` and `deserialize` traits for Serde use

### Dynamic linking

By default the build will attempt to dynamically link with the library installed via your OS's package manager.

You can specify an include path yourself by setting the environment variable `WEBRTC_AUDIO_PROCESSING_INCLUDE`.

### Packages

```sh
sudo apt install libwebrtc-audio-processing-dev # Ubuntu/Debian
sudo pacman -S webrtc-audio-processing # Arch
```

### Build from source

The webrtc source code is included as a git submodule. Be sure to clone this repo with the `--recursive` flag, or pull the submodule with `git submodule update --init`.

Building from source and static linking can be enabled with the `bundled` feature flag. You need the following tools to build from source:

* `clang` or `gcc`
* `autotools` (MacOS: `brew install automake`, `brew install autoconf`)
* `libtoolize` (typically `glibtoolize` on MacOS: `brew install libtool`)
* `pkg-config` (MacOS: `brew install pkg-config`)
* `automake` (MacOS: `brew install automake`)

## Publishing

```bash
cargo login
cd ./webrtc-audio-processing-sys
cargo publish --features derive_serde --features bundled
cd ../
cargo publish --features derive_serde --features bundled
```

## Contributing

### Version increment

We are using semantic versioning. When incrementing a version, please do so in a separate commit, and also mark it with a Github tag.

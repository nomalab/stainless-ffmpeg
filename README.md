# stainless-ffmpeg
Efficient Rust wrapper for FFmpeg.

[![Build Status](https://travis-ci.com/nomalab/stainless-ffmpeg.svg?branch=master)](https://travis-ci.com/nomalab/stainless-ffmpeg)
[![Coverage Status](https://coveralls.io/repos/github/nomalab/stainless-ffmpeg/badge.svg?branch=master)](https://coveralls.io/github/nomalab/stainless-ffmpeg?branch=master)

## Prerequisites
* [Rust](https://rustup.rs/)

## Build

```bash
cargo build
```

## Run examples

- Display file characteristics from container format and streams (video, audio, subtitles, data, ..) :
```bash
cargo run --example probe -- my_movie.mxf
```

- Use graph for encoding and decoding video and audio :
```bash
cargo run --example graph -- my_graph.json
```

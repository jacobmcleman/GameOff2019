# GameOff2019 
[![Build Status](https://travis-ci.org/jacobmcleman/GameOff2019.svg?branch=master)](https://travis-ci.org/jacobmcleman/GameOff2019)
Setup instructions:
1. Follow the links/instructions here https://www.rust-lang.org/learn/get-started to install the rust toolchain
2. If you want to build the web version, install cargo-web with `cargo install cargo-web` (full instructions here https://github.com/koute/cargo-web)
3. If you're running on linux - make sure the following packages are installed: `libudev`, `zlib`, and `alsa` (on debian/ubuntu based distributions: `sudo apt install libudev-dev zlib1g-dev alsa libasound2-dev`
4. To build and run
 - for desktop - `cargo run`
 - for web - `cargo web start`
 
 VS Code is recommended, with the rls, crates, and better TOML extensions.

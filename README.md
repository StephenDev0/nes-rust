# nes-rust (StephenDev0 Continuation)

[![Build Status](https://travis-ci.com/StephenDev0/nes-rust.svg?branch=master)](https://travis-ci.com/StephenDev0/nes-rust)
[![Crate](https://img.shields.io/crates/v/nes_rust.svg)](https://crates.io/crates/nes_rust)
[![npm version](https://badge.fury.io/js/nes_rust_wasm.svg)](https://badge.fury.io/js/nes_rust_wasm)

nes-rust is a NES emulator written in Rust. This repository is a continuation of the original [takahirox/nes-rust](https://github.com/takahirox/nes-rust) project, updated with Swift Package Manager support for iOS.

## Online Demos / Videos (Original Project)

- [Online Singleplay Demo](https://takahirox.github.io/nes-rust/wasm/web/index.html)
- [Online Multiplay Demo](https://takahirox.github.io/nes-rust/wasm/web/multiplay.html) / [Video](https://twitter.com/superhoge/status/1205427421010247680)
- [Online VR Multiplay Demo](https://takahirox.github.io/nes-rust/wasm/web/vr.html) / [Video](https://twitter.com/superhoge/status/1209685614074875906)

## Screenshots

[nestest](http://wiki.nesdev.com/w/index.php/Emulator_tests)

![nestest](./screenshots/nestest.png)

[Sgt. Helmet Training Day](http://www.mojontwins.com/juegos_mojonos/sgt-helmet-training-day-nes/)

![Sgt. Helmet Training Day](./screenshots/Sgt_Helmet.png)

## Features

- Audio support with SDL2 / WebAudio
- WebAssembly support
- Remote multiplay support with WebRTC
- **Swift Package Manager (SPM) support for iOS**

## How to import into your Rust project

The emulator module and document are released at [crates.io](https://crates.io/crates/nes_rust).

## How to build core library locally

```
$ git clone https://github.com/StephenDev0/nes-rust.git
$ cd nes-rust
$ cargo build --release
```

## How to run as desktop application

Prerequirements
- Install [Rust-SDL2](https://github.com/Rust-SDL2/rust-sdl2#rust)

```
$ cd nes-rust/cli
$ cargo run --release path_to_rom_file
```

## How to use as a Swift Package for iOS

This repository contains a Swift Package definition that allows you to easily import the NES Rust core into your iOS projects.

### Prerequisites

1.  **Rust**: Ensure you have Rust installed.
2.  **iOS Targets**: Add the iOS targets for Rust:
    ```bash
    rustup target add aarch64-apple-ios x86_64-apple-ios aarch64-apple-ios-sim
    ```

### Building the XCFramework

Before adding the package to your project, you must build the underlying Rust library and generate the XCFramework.

1.  Run the build script:
    ```bash
    cd ios
    ./build_rust.sh
    ```
    This will compile the Rust code and place the result in `ios/NesRust.xcframework`.

### Adding to your Xcode Project

1.  Open your project in Xcode.
2.  Select **File > Add Packages...**
3.  Enter the Git URL of this repository: `https://github.com/StephenDev0/nes-rust.git`
4.  Select the **NesRust** package.
5.  Add the **NesRust** library to your app target.

> **Note:** The `ios/NesRust.xcframework` must be built and committed to the repository for the package to work via Git URL.

### Usage in Swift

```swift
import NesRust

let nes = nes_create()

let romData: [UInt8] = ... // Load your ROM data
nes_load_rom(nes, romData, romData.count)

nes_step_frame(nes)

nes_destroy(nes)
```

## How to import and use WebAssembly NES emulator in a web browser

See [wasm/web](https://github.com/StephenDev0/nes-rust/tree/master/wasm/web)

## How to install and use WebAssembly NES emulator npm package

See [wasm/npm](https://github.com/StephenDev0/nes-rust/tree/master/wasm/npm)
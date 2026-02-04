#!/bin/bash
set -e

echo "Building Rust library for iOS..."

cargo build --release --lib --target aarch64-apple-ios

echo "Building for Simulator (x86_64-apple-ios)..."
cargo build --release --lib --target x86_64-apple-ios
echo "Building for Simulator (aarch64-apple-ios-sim)..."
cargo build --release --lib --target aarch64-apple-ios-sim

mkdir -p ios/build
rm -rf ios/NesRust.xcframework

echo "Creating universal simulator library..."
lipo -create \
    target/x86_64-apple-ios/release/libnes_rust.a \
    target/aarch64-apple-ios-sim/release/libnes_rust.a \
    -output ios/build/libnes_rust_sim.a

cp target/aarch64-apple-ios/release/libnes_rust.a ios/build/libnes_rust_device.a

echo "Creating XCFramework..."
xcodebuild -create-xcframework \
    -library ios/build/libnes_rust_device.a -headers ios/include \
    -library ios/build/libnes_rust_sim.a -headers ios/include \
    -output ios/NesRust.xcframework

echo "Done! XCFramework is at ios/NesRust.xcframework"
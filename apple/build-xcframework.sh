#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "$0")" && pwd)"
repo_dir="$(cd "$script_dir/.." && pwd)"
artifact="$script_dir/Artifacts/CLastDraftFlow.xcframework"
headers="$script_dir/Sources/CLastDraftFlow"
simulator_dir="$repo_dir/target/apple-universal-sim/release"
simulator_library="$simulator_dir/liblastdraft_flow.a"

rustup target add aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios
cargo build --manifest-path "$repo_dir/Cargo.toml" --release --offline
cargo build --manifest-path "$repo_dir/Cargo.toml" --release --offline --target aarch64-apple-ios
cargo build --manifest-path "$repo_dir/Cargo.toml" --release --offline --target aarch64-apple-ios-sim
cargo build --manifest-path "$repo_dir/Cargo.toml" --release --offline --target x86_64-apple-ios

mkdir -p "$simulator_dir"
lipo -create \
  "$repo_dir/target/aarch64-apple-ios-sim/release/liblastdraft_flow.a" \
  "$repo_dir/target/x86_64-apple-ios/release/liblastdraft_flow.a" \
  -output "$simulator_library"

if [[ -e "$artifact" ]]; then
  rm -rf "$artifact"
fi

xcodebuild -create-xcframework \
  -library "$repo_dir/target/release/liblastdraft_flow.a" \
  -headers "$headers" \
  -library "$repo_dir/target/aarch64-apple-ios/release/liblastdraft_flow.a" \
  -headers "$headers" \
  -library "$simulator_library" \
  -headers "$headers" \
  -output "$artifact"

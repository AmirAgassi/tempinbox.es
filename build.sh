#!/bin/bash
set -e

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source $HOME/.cargo/env

# Add wasm32-unknown-unknown target
rustup target add wasm32-unknown-unknown

# Install trunk
cargo install trunk

# Create assets directory if it doesn't exist
mkdir -p assets
# Build the project
trunk build --release --public-url / --dist dist --no-sri

# Ensure assets are copied to the dist directory
if [ -d "assets" ]; then
  cp -r assets dist/
fi
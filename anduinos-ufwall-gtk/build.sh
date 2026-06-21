#!/bin/bash
set -e

echo "Building ufwall-gtk with Cargo..."
cargo build --release

#!/bin/bash
set -e

echo "Building ufwall-gtk with Cargo..."
cargo build --release

echo "Compiling locales..."
bash compile-locales.sh

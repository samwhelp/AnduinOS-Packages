#!/bin/bash
set -e

if ! command -v cargo &> /dev/null; then
    echo "Installing build dependencies..."
    sudo apt-get update
    sudo apt-get install -y cargo rustc libgtk-4-dev libadwaita-1-dev
fi

echo "Building ufwall-gtk with Cargo..."
cargo build --release

echo "Preparing deploy directory..."
rm -rf deploy
mkdir -p deploy/usr/bin
mkdir -p deploy/usr/share/applications
mkdir -p deploy/usr/share/polkit-1/actions

mkdir -p deploy/usr/share/icons/hicolor/scalable/apps

echo "Copying binary..."
cp target/release/ufwall-gtk deploy/usr/bin/

echo "Copying desktop file..."
cp data/com.anduinos.ufwall.desktop deploy/usr/share/applications/

echo "Copying polkit policy..."
cp data/com.anduinos.ufwall.policy deploy/usr/share/polkit-1/actions/

echo "Copying icon..."
cp data/com.anduinos.ufwall.svg deploy/usr/share/icons/hicolor/scalable/apps/

echo "Build and deploy preparation complete."

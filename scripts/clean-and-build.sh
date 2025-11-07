#!/bin/bash

# Clean and Build Script for MineKB
# This script cleans extended attributes and builds the Tauri app

set -e

echo "🧹 Cleaning extended attributes..."

# Remove extended attributes from the entire project
# This is necessary to avoid "failed to remove extra attributes from app bundle" error
# Using system xattr (not Python xattr) with -r for recursive and -c for clear
/usr/bin/xattr -cr . 2>/dev/null || true

# Clean the target directory if it exists
if [ -d "src-tauri/target" ]; then
    echo "🧹 Cleaning Rust target directory..."
    cargo clean --manifest-path src-tauri/Cargo.toml
fi

# Clean the dist directory if it exists
if [ -d "dist" ]; then
    echo "🧹 Cleaning dist directory..."
    rm -rf dist
fi

# Clean node_modules/.vite cache if it exists
if [ -d "node_modules/.vite" ]; then
    echo "🧹 Cleaning Vite cache..."
    rm -rf node_modules/.vite
fi

echo "✨ Cleaning complete!"
echo ""
echo "🔨 Starting Tauri build..."

# Source environment variables if .env.local exists
if [ -f ".env.local" ]; then
    echo "📝 Loading environment variables from .env.local"
    set -a
    source .env.local
    set +a
    # Unset signing identity to prevent automatic signing during build
    unset APPLE_SIGNING_IDENTITY
fi

echo "⚠️  Automatic signing disabled (signingIdentity: \"-\")"
echo ""

# Build the Tauri app
# Note: tauri build will automatically run 'npm run build' via beforeBuildCommand
# configured in tauri.conf.json, so we don't need to run it separately
npx @tauri-apps/cli build

echo ""
echo "✅ Build complete!"
echo "📦 Output location: src-tauri/target/release/bundle/macos/"


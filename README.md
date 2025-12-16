# MegaStrife - Sega Genesis/Mega Drive Emulator

A Sega Genesis/Mega Drive emulator written in Rust.

## Features

- Motorola 68000 CPU emulation (in progress)
- VDP (Video Display Processor) emulation
- Full memory system with ROM loading
- Window rendering using Minifb
- Input handling (keyboard)

## Building

### Requirements
- Rust 1.70+ (https://rustup.rs/)
- Windows (currently), but cross-platform support planned

### Build Commands

```powershell
# Debug build
cargo build

# Release build (optimized)
cargo build --release

# Run with test ROM
cargo run -- roms/test/Sonic\ The\ Hedgehog\ \(USA,\ Europe\).md

# Or use the build script
.\build.ps1 -Run
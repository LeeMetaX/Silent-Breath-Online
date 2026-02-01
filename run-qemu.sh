#!/bin/bash
set -euo pipefail

# Silent-Breath-Online QEMU Runner
# Builds the bare-metal kernel and launches it in QEMU (UEFI boot)
#
# Prerequisites:
#   apt install qemu-system-x86 ovmf mtools dosfstools
#   rustup component add rust-src llvm-tools-preview
#
# Usage:
#   ./run-qemu.sh              # Build + run (debug)
#   ./run-qemu.sh --release    # Build + run (release, optimized)
#   ./run-qemu.sh --build-only # Build disk image only, don't launch QEMU
#   ./run-qemu.sh --help       # Show help

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BAREMETAL_DIR="$SCRIPT_DIR/baremetal-abi"
RUNNER_DIR="$SCRIPT_DIR/qemu-runner"
BOOTLOADER_EFI="$BAREMETAL_DIR/boot/bootloader-x86_64-uefi.efi"

BUILD_MODE="debug"
BUILD_ONLY=""

for arg in "$@"; do
    case "$arg" in
        --release)    BUILD_MODE="release" ;;
        --build-only) BUILD_ONLY="--no-run" ;;
        --help|-h)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --release     Build in release mode (LTO, optimized)"
            echo "  --build-only  Create disk image without launching QEMU"
            echo "  -h, --help    Show this help"
            echo ""
            echo "Prerequisites:"
            echo "  apt install qemu-system-x86 ovmf mtools dosfstools"
            echo "  rustup component add rust-src llvm-tools-preview"
            exit 0
            ;;
        *)
            echo "Unknown option: $arg"
            exit 1
            ;;
    esac
done

echo "========================================"
echo "Silent-Breath-Online QEMU Runner"
echo "========================================"
echo "Build mode: $BUILD_MODE"
echo "Boot:       UEFI (OVMF)"
echo ""

# Step 1: Build the kernel ELF for the custom bare-metal target
echo "[1/3] Building bare-metal kernel..."

CARGO_BUILD_ARGS=(
    "build"
    "--bin" "minimal-kernel"
    "-Zbuild-std=core,compiler_builtins,alloc"
    "-Zbuild-std-features=compiler-builtins-mem"
    "-Zjson-target-spec"
    "--target" "$BAREMETAL_DIR/x86_64-i9-12900k.json"
    "--manifest-path" "$BAREMETAL_DIR/Cargo.toml"
)

if [ "$BUILD_MODE" = "release" ]; then
    CARGO_BUILD_ARGS+=("--release")
fi

cargo "${CARGO_BUILD_ARGS[@]}"

# Find the kernel ELF
TARGET_DIR="$BAREMETAL_DIR/target/x86_64-i9-12900k/$BUILD_MODE"
KERNEL_ELF="$TARGET_DIR/minimal-kernel"

if [ ! -f "$KERNEL_ELF" ]; then
    echo "Error: Kernel ELF not found at $KERNEL_ELF"
    exit 1
fi

echo "[+] Kernel ELF: $KERNEL_ELF"
echo ""

# Verify UEFI bootloader exists
if [ ! -f "$BOOTLOADER_EFI" ]; then
    echo "Error: UEFI bootloader not found at $BOOTLOADER_EFI"
    echo "The pre-built bootloader EFI should be in baremetal-abi/boot/"
    exit 1
fi

# Step 2: Build the disk image builder (host tool)
echo "[2/3] Building disk image builder..."
cargo build --manifest-path "$RUNNER_DIR/Cargo.toml" --release 2>&1
echo ""

# Step 3: Create UEFI disk image and optionally launch QEMU
echo "[3/3] Creating disk image and launching..."
RUNNER_BIN="$RUNNER_DIR/target/release/qemu-runner"
"$RUNNER_BIN" "$KERNEL_ELF" "$BOOTLOADER_EFI" $BUILD_ONLY

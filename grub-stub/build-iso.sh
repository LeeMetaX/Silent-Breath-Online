#!/usr/bin/env bash
# Build the GRUB-bootable ISO for the Silent-Breath GRUB stub.
#
# Requires: cargo (nightly), nasm, grub-mkrescue, xorriso, mtools.
# Optional: qemu-system-x86_64 (for the `run` subcommand below).

set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$HERE"

KERNEL_TARGET="x86_64-grub"
KERNEL_PROFILE="${KERNEL_PROFILE:-release}"
KERNEL_BIN="target/${KERNEL_TARGET}/${KERNEL_PROFILE}/grub-stub"
ISO_DIR="target/iso"
ISO_OUT="target/grub-stub.iso"

case "${KERNEL_PROFILE}" in
    release) CARGO_FLAGS="--release" ;;
    debug)   CARGO_FLAGS="" ;;
    *) echo "KERNEL_PROFILE must be 'release' or 'debug'" >&2; exit 1 ;;
esac

echo "==> Building kernel ($KERNEL_PROFILE)"
cargo +nightly build $CARGO_FLAGS

if [[ ! -f "$KERNEL_BIN" ]]; then
    echo "Kernel binary not found at $KERNEL_BIN" >&2
    exit 1
fi

echo "==> Verifying Multiboot2 header"
if ! grub-file --is-x86-multiboot2 "$KERNEL_BIN"; then
    echo "ERROR: $KERNEL_BIN does not contain a valid Multiboot2 header" >&2
    exit 1
fi

echo "==> Staging ISO tree at $ISO_DIR"
rm -rf "$ISO_DIR"
mkdir -p "$ISO_DIR/boot/grub"
cp "$KERNEL_BIN" "$ISO_DIR/boot/grub-stub"
cp grub.cfg "$ISO_DIR/boot/grub/grub.cfg"

echo "==> Producing ISO at $ISO_OUT"
grub-mkrescue -o "$ISO_OUT" "$ISO_DIR" >/dev/null 2>&1 \
    || grub-mkrescue -o "$ISO_OUT" "$ISO_DIR"

echo
echo "Done."
echo "  Kernel: $KERNEL_BIN"
echo "  ISO:    $ISO_OUT"
echo
echo "Boot in QEMU (serial mirrored to stdio):"
echo "  qemu-system-x86_64 -cdrom $ISO_OUT -serial stdio -display none -no-reboot"

# grub-stub

A minimal, GRUB-bootable Linux-style kernel stub for Silent-Breath-Online.

GRUB boots this image via the Multiboot2 protocol. The stub takes the CPU
from 32-bit protected mode all the way into 64-bit long mode and then hands
control to a Rust `kernel_main` function, which is where you start writing
the rest of your kernel.

```
[ GRUB ]                            (BIOS / UEFI loads grub.cfg, picks menu entry)
   |
   | Multiboot2 protocol: jumps to _start in 32-bit protected mode
   | eax = 0x36d76289, ebx = phys ptr to multiboot info
   v
[ src/boot.S : 32-bit ]             (verify magic, zero BSS, build page tables,
   |                                 enable PAE/LME/PG, load 64-bit GDT)
   |
   | far jmp 0x08:long_mode_start
   v
[ src/boot.S : 64-bit ]             (reload segments, enable OSFXSR, set rsp,
   |                                 pass multiboot info ptr in rdi)
   |
   | call kernel_main
   v
[ src/main.rs ]                     (Rust no_std no_main; VGA + COM1 output)
```

## Layout

| Path             | Role                                                       |
|------------------|------------------------------------------------------------|
| `src/boot.S`     | Multiboot2 header + 32→64-bit transition (NASM, ELF64)     |
| `src/main.rs`    | `#[no_mangle] kernel_main(multiboot_info_phys: u64)`       |
| `linker.ld`      | Places `.boot` first at 1 MiB, then text/rodata/data/bss   |
| `build.rs`       | Runs NASM, packs `boot.o` into a static lib, links it in   |
| `x86_64-grub.json` | Custom kernel target (static reloc, kernel code model)   |
| `grub.cfg`       | GRUB menu entry that boots the kernel via `multiboot2`     |
| `build-iso.sh`   | One-shot build: kernel → multiboot check → ISO via xorriso |

## Prerequisites

- `cargo` + `rustc` nightly (the `rust-toolchain.toml` at the repo root pins it)
- `rust-src` and `llvm-tools-preview` components (build-std)
- `nasm` (assembles `boot.S`)
- `grub-mkrescue` + `xorriso` + `mtools` (builds the ISO)
- `qemu-system-x86_64` (to test it)

On Debian/Ubuntu:

```
apt-get install nasm grub-pc-bin grub-common xorriso mtools qemu-system-x86
rustup component add rust-src llvm-tools-preview --toolchain nightly
```

## Build

```
cd grub-stub
./build-iso.sh           # produces target/grub-stub.iso
```

The script:

1. `cargo +nightly build --release` (uses `build-std`, custom target)
2. `grub-file --is-x86-multiboot2` (sanity-checks the ELF header)
3. Stages `target/iso/boot/{grub-stub, grub/grub.cfg}`
4. `grub-mkrescue` produces `target/grub-stub.iso`

## Run

```
qemu-system-x86_64 -cdrom target/grub-stub.iso -serial stdio -display none -no-reboot
```

You should see, on the serial console:

```
Silent-Breath GRUB stub: long mode reached
Multiboot2 info @ 0x00000000001XXXXX
CR3 = 0x0000000000104000
Identity map: 0..1 GiB (2 MiB pages)
Stub halting. Replace `kernel_main` to extend.
```

The same banner is also written to the VGA text buffer at `0xb8000`, which is
what you see if you boot on real hardware or omit `-display none`.

## State at `kernel_main`

When Rust takes over:

- CPU mode: 64-bit long mode, CPL 0, interrupts disabled
- Paging: 4-level, identity map of `[0, 1 GiB)` via 2 MiB huge pages
- `CR4`: PAE | OSFXSR | OSXMMEXCPT (so SSE2 is legal — the SysV ABI needs it)
- `CR0`: PE | MP | NE | WP | PG
- `EFER`: LME | LMA
- GDT: flat 64-bit (selector `0x08` code, `0x10` data); IDT not yet loaded
- `rsp`: top of a 16 KiB boot stack inside `.bss`
- `rdi`: physical address of the Multiboot2 information structure

What the stub does **not** do (left for you to add):

- Parse the Multiboot2 info tags (memory map, framebuffer, modules…)
- Install an IDT / interrupt handlers
- Set up a real frame allocator or kernel heap
- Bring up the APs / SMP

## Extending

Most kernel work goes in `src/main.rs`. The function signature is:

```rust
#[no_mangle]
pub extern "C" fn kernel_main(multiboot_info_phys: u64) -> !;
```

If you add more `.S` files, list them in `build.rs` and add their objects to
the static archive. If you change segment layout or the load address, update
`linker.ld` and the page-table setup in `boot.S` so they keep agreeing.

## Hardware notes

The stub itself runs on any x86_64 machine, but if you want to share state
with the rest of Silent-Breath-Online's i9-12900K-specific cache-coherency
code (`baremetal-abi/`), keep these points in mind:

- This stub uses `code-model=kernel` (kernel in the upper canonical half is
  not currently set up; we run identity-mapped at 1 MiB). Switch to a
  higher-half mapping before linking against `i9-12900k-baremetal-abi` if you
  want to share its address-space assumptions.
- The MESI/coherency runtime in `baremetal-abi/src` assumes a 16-core hybrid
  topology; the stub is single-CPU until you add AP bring-up.

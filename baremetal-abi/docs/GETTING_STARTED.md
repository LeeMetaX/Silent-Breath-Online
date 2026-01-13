# Getting Started with i9-12900K Bare-Metal ABI

This guide will help you build and run your custom ABI on the Intel Core i9-12900K.

---

## Table of Contents

1. [Prerequisites](#prerequisites)
2. [Project Structure](#project-structure)
3. [Building](#building)
4. [Testing in QEMU](#testing-in-qemu)
5. [Running on Real Hardware](#running-on-real-hardware)
6. [Writing Your First Program](#writing-your-first-program)
7. [Performance Tuning](#performance-tuning)
8. [Troubleshooting](#troubleshooting)

---

## Prerequisites

### Software Requirements

```bash
# 1. Install Rust nightly
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup default nightly

# 2. Add bare-metal target
rustup target add x86_64-unknown-none

# 3. Install bootimage tool
cargo install bootimage

# 4. Install QEMU (for testing)
# Ubuntu/Debian:
sudo apt install qemu-system-x86 ovmf

# Arch Linux:
sudo pacman -S qemu-system-x86 edk2-ovmf

# macOS:
brew install qemu
```

### Hardware Requirements

**For Virtual Testing:**
- QEMU with UEFI support
- 4+ GB RAM recommended

**For Real Hardware:**
- Intel Core i9-12900K processor
- UEFI-capable motherboard
- USB drive for booting
- **IMPORTANT**: Recovery mechanism in case of boot failure

---

## Project Structure

```
baremetal-abi/
├── Cargo.toml                    # Project manifest
├── x86_64-i9-12900k.json        # Custom target specification
├── README.md                     # Project documentation
├── docs/
│   ├── ABI_SPECIFICATION.md     # Complete ABI spec
│   └── GETTING_STARTED.md       # This file
└── src/
    ├── lib.rs                   # Library root
    ├── abi.rs                   # Calling convention & syscalls
    ├── boot.rs                  # UEFI entry point
    ├── cpu.rs                   # CPU initialization & MSR access
    ├── interrupts.rs            # Exception handlers
    ├── memory.rs                # Memory management
    └── performance.rs           # Performance monitoring
```

---

## Building

### Step 1: Navigate to Project Directory

```bash
cd /home/user/Silent-Breath-Online/baremetal-abi
```

### Step 2: Build Debug Version

```bash
cargo build --target x86_64-i9-12900k.json
```

### Step 3: Build Optimized Release Version

```bash
cargo build --release --target x86_64-i9-12900k.json
```

### Step 4: Create Bootable Image

```bash
# Install bootimage if you haven't already
cargo install bootimage

# Create bootable disk image
cargo bootimage --target x86_64-i9-12900k.json --release
```

**Output**: `target/x86_64-i9-12900k/release/bootimage-i9_12900k_baremetal_abi.bin`

---

## Testing in QEMU

### Basic QEMU Test

```bash
qemu-system-x86_64 \
    -bios /usr/share/ovmf/OVMF.fd \
    -drive format=raw,file=target/x86_64-i9-12900k/release/bootimage-i9_12900k_baremetal_abi.bin \
    -m 4G \
    -smp cores=16,threads=1 \
    -serial stdio \
    -no-reboot \
    -no-shutdown
```

### QEMU with Full CPU Features

```bash
qemu-system-x86_64 \
    -bios /usr/share/ovmf/OVMF.fd \
    -drive format=raw,file=target/x86_64-i9-12900k/release/bootimage-i9_12900k_baremetal_abi.bin \
    -m 8G \
    -smp cores=16,threads=1 \
    -cpu host \
    -enable-kvm \
    -serial stdio \
    -display none \
    -d int,cpu_reset
```

### QEMU Debug Mode

```bash
qemu-system-x86_64 \
    -bios /usr/share/ovmf/OVMF.fd \
    -drive format=raw,file=target/x86_64-i9-12900k/release/bootimage-i9_12900k_baremetal_abi.bin \
    -m 4G \
    -smp cores=16,threads=1 \
    -serial stdio \
    -s -S  # Wait for GDB on localhost:1234
```

Then in another terminal:
```bash
gdb target/x86_64-i9-12900k/release/i9_12900k_baremetal_abi
(gdb) target remote localhost:1234
(gdb) continue
```

---

## Running on Real Hardware

⚠️ **WARNING: Running bare-metal code on real hardware can brick your system if configured incorrectly. Always have a recovery mechanism!**

### Method 1: USB Boot (Recommended for Testing)

```bash
# 1. Write image to USB drive (replace /dev/sdX with your USB device)
sudo dd if=target/x86_64-i9-12900k/release/bootimage-i9_12900k_baremetal_abi.bin \
        of=/dev/sdX \
        bs=4M \
        status=progress
sudo sync

# 2. Boot from USB
# - Insert USB drive
# - Restart computer
# - Enter BIOS/UEFI (usually F2, F12, or Del)
# - Select USB drive as boot device
# - Boot
```

### Method 2: Network Boot (PXE)

1. Set up a PXE server
2. Copy bootimage to TFTP directory
3. Configure DHCP to point to your image
4. Boot via network

### Method 3: GRUB Chainload

```bash
# Copy image to /boot
sudo cp target/x86_64-i9-12900k/release/bootimage-i9_12900k_baremetal_abi.bin /boot/

# Add to GRUB config (/etc/grub.d/40_custom)
menuentry "i9-12900K Bare-Metal ABI" {
    set root='hd0,1'
    multiboot2 /boot/bootimage-i9_12900k_baremetal_abi.bin
    boot
}

# Update GRUB
sudo update-grub
```

---

## Writing Your First Program

### Example 1: Hello World (Serial Output)

Create `src/examples/hello.rs`:

```rust
#![no_std]
#![no_main]

use i9_12900k_baremetal_abi::boot::entry_point;

entry_point!(main);

fn main(_boot_info: &'static mut BootInfo) -> ! {
    // This will output to serial port
    serial_println!("Hello from i9-12900K!");

    // Print core information
    let core_type = cpu::get_core_type();
    let core_id = cpu::get_core_id();
    serial_println!("Core {}: {:?}", core_id, core_type);

    loop {
        unsafe {
            core::arch::asm!("hlt", options(nomem, nostack));
        }
    }
}
```

### Example 2: Performance Benchmark

```rust
use i9_12900k_baremetal_abi::performance;

fn benchmark_example() {
    // Benchmark matrix multiplication
    let (result, cycles) = performance::benchmark(|| {
        matrix_multiply_4x4()
    });

    serial_println!("Matrix multiplication: {} cycles", cycles);

    // Measure IPC
    let (_, ipc) = performance::benchmark_ipc(|| {
        fibonacci(20)
    });

    serial_println!("Fibonacci IPC: {:.2}", ipc);
}

fn matrix_multiply_4x4() -> [[i32; 4]; 4] {
    // Your implementation
    [[0; 4]; 4]
}

fn fibonacci(n: u32) -> u64 {
    match n {
        0 => 0,
        1 => 1,
        n => fibonacci(n - 1) + fibonacci(n - 2),
    }
}
```

### Example 3: Core Affinity Demo

```rust
use i9_12900k_baremetal_abi::{CoreAffinity, cpu, abi::syscalls};

fn core_affinity_demo() {
    // Check initial core
    let initial_core = cpu::get_core_id();
    serial_println!("Starting on core {}", initial_core);

    // Request P-core for performance-critical work
    syscalls::set_core_affinity(CoreAffinity::PerformanceRequired).unwrap();

    // Run fast computation
    let (result, p_cycles) = performance::benchmark(|| {
        intensive_computation()
    });
    serial_println!("P-core computation: {} cycles", p_cycles);

    // Switch to E-core for background work
    syscalls::set_core_affinity(CoreAffinity::EfficiencyPreferred).unwrap();

    // Run background task
    let (_, e_cycles) = performance::benchmark(|| {
        background_task()
    });
    serial_println!("E-core computation: {} cycles", e_cycles);
}
```

---

## Performance Tuning

### 1. Cache Optimization

```rust
// Align hot structures to cache line (64 bytes)
#[repr(align(64))]
struct HotPath {
    data: [u64; 8],  // Exactly one cache line
}

// Prefetch data
unsafe {
    use core::arch::x86_64::_mm_prefetch;
    _mm_prefetch(ptr as *const i8, 3);  // Prefetch to L1
}
```

### 2. Branch Prediction Hints

```rust
// Mark likely/unlikely branches
if likely(condition) {
    fast_path();
} else {
    slow_path();
}

#[inline(always)]
fn likely(condition: bool) -> bool {
    #[cold]
    #[inline(never)]
    fn cold() {}

    if !condition {
        cold();
    }
    condition
}
```

### 3. SIMD Optimization

```rust
use core::arch::x86_64::*;

unsafe fn avx2_sum(data: &[f32; 8]) -> f32 {
    let vec = _mm256_loadu_ps(data.as_ptr());
    let sum = _mm256_hadd_ps(vec, vec);
    let result = _mm256_extractf128_ps(sum, 0);
    // Extract and return sum
    0.0  // Simplified
}
```

### 4. Core-Specific Optimization

```rust
fn optimized_workload() {
    if cpu::is_performance_core() {
        // Use AVX2 and aggressive optimizations
        avx2_optimized_path();
    } else {
        // E-core: simpler, more power-efficient code
        scalar_optimized_path();
    }
}
```

---

## Troubleshooting

### Issue: QEMU doesn't boot

**Solution:**
```bash
# Ensure OVMF UEFI firmware is installed
sudo apt install ovmf  # Ubuntu/Debian
sudo pacman -S edk2-ovmf  # Arch

# Check OVMF location
ls /usr/share/ovmf/OVMF.fd
ls /usr/share/edk2-ovmf/x64/OVMF.fd

# Use correct path in QEMU -bios argument
```

### Issue: Build fails with "can't find crate"

**Solution:**
```bash
# Ensure nightly toolchain
rustup default nightly

# Add target
rustup target add x86_64-unknown-none

# Clean and rebuild
cargo clean
cargo build --target x86_64-i9-12900k.json
```

### Issue: Serial output not showing

**Solution:**
```bash
# Enable serial feature
cargo build --features serial --release

# In QEMU, ensure -serial stdio
qemu-system-x86_64 ... -serial stdio
```

### Issue: Triple fault on boot

**Cause:** Usually IDT or GDT misconfiguration

**Solution:**
- Enable QEMU debug mode: `-d int,cpu_reset`
- Check exception handlers are properly registered
- Verify stack alignment (16-byte)

### Issue: Performance counters return 0

**Cause:** PMU not enabled or running in VM without KVM

**Solution:**
```bash
# Enable KVM in QEMU
qemu-system-x86_64 ... -enable-kvm -cpu host

# Verify PMU is accessible
# Check MSR_PERF_GLOBAL_CTRL is set
```

---

## Next Steps

1. **Read the ABI Specification**: [docs/ABI_SPECIFICATION.md](ABI_SPECIFICATION.md)
2. **Explore Examples**: Check `src/examples/` for more samples
3. **Integrate Cache Coherency**: See Silent-Breath-Online integration
4. **Add Custom Syscalls**: Extend the syscall interface
5. **Implement Drivers**: Add hardware-specific drivers

---

## Additional Resources

- [Intel SDM Volume 4](https://www.intel.com/content/dam/develop/external/us/en/documents/335592-sdm-vol-4.pdf)
- [Phil Opp's OS Dev Blog](https://os.phil-opp.com/)
- [OSDev Wiki](https://wiki.osdev.org/)
- [rust-osdev GitHub](https://github.com/rust-osdev)

---

## Getting Help

If you encounter issues:

1. Check the [Troubleshooting](#troubleshooting) section
2. Review QEMU logs with `-d int,cpu_reset`
3. File an issue on the GitHub repository

---

**Happy bare-metal programming!** 🚀

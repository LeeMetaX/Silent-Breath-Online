# i9-12900K Bare-Metal Firmware ABI

A custom Application Binary Interface (ABI) and bare-metal firmware for the Intel Core i9-12900K (Alder Lake) processor.

## Features

### ✨ Custom ABI
- **Calling Convention**: System V AMD64-based with hybrid core extensions
- **Core Affinity**: P-core/E-core scheduling hints
- **Register Allocation**: Optimized for hybrid architecture
- **System Call Interface**: Custom syscall numbers for hardware control

### 🔧 Hardware Support
- **Hybrid Architecture**: 8 P-cores (Golden Cove) + 8 E-cores (Gracemont)
- **Thread Director Integration**: Hardware scheduling hints
- **MSR Access**: Read/write Model Specific Registers
- **CPUID Detection**: Core type detection and feature enumeration

### ⚡ Performance Optimization
- **Cache Coherency**: Integrated Silent-Breath-Online MESI protocol
- **Performance Counters**: Intel PMU integration (instructions, cycles, IPC)
- **Benchmarking**: Built-in cycle-accurate benchmarking
- **Cache-Line Alignment**: 64-byte aligned structures

### 🛠 Low-Level Control
- **UEFI Boot**: Modern UEFI entry point
- **CPU Initialization**: SSE/AVX enablement, CR0/CR4 setup
- **Exception Handling**: IDT with custom exception handlers
- **Memory Management**: Virtual memory, MMIO regions

---

## Architecture

### CPU Core Layout

```
i9-12900K (16 cores, 24 threads)
├─ P-Cores (0-7): Golden Cove, HT enabled
│  ├─ Logical 0-1:   Physical core 0
│  ├─ Logical 2-3:   Physical core 1
│  ├─ ...
│  └─ Logical 14-15: Physical core 7
│
└─ E-Cores (8-15): Gracemont, no HT
   ├─ Logical 16: Physical core 8
   ├─ Logical 17: Physical core 9
   ├─ ...
   └─ Logical 23: Physical core 15
```

### ISA Features

- **Base**: x86-64-v3
- **SIMD**: SSE4.2, AVX, AVX2 (up to 256-bit)
- **Extensions**: AES-NI, RDRAND, RDSEED, BMI1, BMI2, FMA
- **AVX-512**: Disabled (incompatible with E-cores active)

### Memory Layout

```
0xFFFF_8000_0000_0000  Direct Physical Mapping
0xFFFF_8800_0000_0000  Kernel Heap (1 GiB)
0xFFFF_9000_4000_0000  L3 Cache MMIO
0xFFFF_9000_4010_0000  Coherency Control
0xFFFF_9000_5000_0000  Shadow Registers
0xFFFF_9000_6000_0000  Hardware Fuses
0xFFFF_FFFF_8000_0000  Kernel Code & Data
```

---

## Building

### Prerequisites

```bash
# Install Rust nightly
rustup default nightly

# Add x86_64-unknown-none target
rustup target add x86_64-unknown-none

# Install bootimage tool
cargo install bootimage
```

### Build Commands

```bash
# Build debug version
cd baremetal-abi
cargo build

# Build release version (optimized)
cargo build --release

# Create bootable disk image
cargo bootimage --release

# Run in QEMU (requires QEMU with UEFI support)
qemu-system-x86_64 \
    -bios /usr/share/ovmf/OVMF.fd \
    -drive format=raw,file=target/x86_64-i9-12900k/release/bootimage-i9-12900k-baremetal-abi.bin \
    -m 4G \
    -smp cores=16,threads=1 \
    -serial stdio
```

---

## Usage Examples

### Example 1: Detect Core Type

```rust
use i9_12900k_baremetal_abi::cpu;

let core_type = cpu::get_core_type();
let core_id = cpu::get_core_id();

match core_type {
    CoreType::Performance => {
        println!("Running on P-core {}", core_id);
    }
    CoreType::Efficiency => {
        println!("Running on E-core {}", core_id);
    }
    CoreType::Unknown => {
        println!("Unknown core type");
    }
}
```

### Example 2: Performance Benchmarking

```rust
use i9_12900k_baremetal_abi::performance;

// Benchmark a function
let (result, cycles) = performance::benchmark(|| {
    // Your performance-critical code here
    expensive_computation()
});

println!("Executed in {} cycles", cycles);

// Measure IPC (Instructions Per Cycle)
let (result, ipc) = performance::benchmark_ipc(|| {
    some_function()
});

println!("IPC: {:.2}", ipc);
```

### Example 3: Core Affinity Hints

```rust
use i9_12900k_baremetal_abi::abi::syscalls;
use i9_12900k_baremetal_abi::CoreAffinity;

// Request to run on P-core
syscalls::set_core_affinity(CoreAffinity::PerformanceRequired).unwrap();

// Run performance-critical workload
perform_fast_computation();

// Switch to E-core for background work
syscalls::set_core_affinity(CoreAffinity::EfficiencyPreferred).unwrap();
background_task();
```

### Example 4: Cache Coherency

```rust
use silent_breath_mmio::{CoherencyRuntime, CacheState};

let mut runtime = CoherencyRuntime::new();

unsafe {
    // Initialize all cores
    for core_id in 0..16 {
        runtime.init_core(core_id);
    }

    // Execute cache coherency flow
    let address = 0x10000000;
    runtime.execute_coherency_flow(address).unwrap();
}
```

### Example 5: MSR Access

```rust
use i9_12900k_baremetal_abi::cpu::{read_msr, write_msr, msr};

unsafe {
    // Read platform info
    let platform_info = read_msr(msr::MSR_PLATFORM_INFO);

    // Read turbo ratio limit
    let turbo_limit = read_msr(msr::MSR_TURBO_RATIO_LIMIT);

    // Configure energy/performance bias
    write_msr(msr::MSR_ENERGY_PERF_BIAS, 0x6); // Balanced
}
```

### Example 6: Performance Counters

```rust
use i9_12900k_baremetal_abi::performance::{get_monitor, PerfEvent};

unsafe {
    let monitor = get_monitor();

    // Start counting L3 cache misses
    if let Some(counter) = monitor.counter_mut(0) {
        counter.start(PerfEvent::LLCMisses);
    }

    // Run your code...
    run_workload();

    // Read counter
    if let Some(counter) = monitor.counter(0) {
        let misses = counter.read();
        println!("L3 cache misses: {}", misses);
        counter.stop();
    }
}
```

---

## ABI Specification

See [docs/ABI_SPECIFICATION.md](docs/ABI_SPECIFICATION.md) for complete details.

### Calling Convention Summary

**Parameters**: `rdi`, `rsi`, `rdx`, `rcx`, `r8`, `r9`, then stack
**Return Value**: `rax` (primary), `rdx` (secondary)
**FP Parameters**: `xmm0`-`xmm7`
**Callee-Saved**: `rbx`, `rbp`, `r12`-`r15`
**Stack Alignment**: 16 bytes
**Red Zone**: Disabled

### System Call Numbers

| Number | Name | Description |
|--------|------|-------------|
| 0x00 | `exit` | Exit process |
| 0x01 | `read_msr` | Read MSR |
| 0x02 | `write_msr` | Write MSR |
| 0x03 | `get_core_type` | Get P-core/E-core type |
| 0x04 | `set_core_affinity` | Set core affinity |
| 0x05 | `cache_flush` | Flush cache line |
| 0x06 | `cache_invalidate` | Invalidate cache line |
| 0x10 | `perf_counter_read` | Read performance counter |
| 0x11 | `perf_counter_start` | Start performance counter |
| 0x12 | `perf_counter_stop` | Stop performance counter |

---

## Performance Characteristics

### Cache Line Size
- L1d: 64 bytes
- L2: 64 bytes
- L3: 64 bytes

### Core-Specific Latencies

| Operation | P-Core | E-Core |
|-----------|--------|--------|
| L1 hit | ~4 cycles | ~4 cycles |
| L2 hit | ~12 cycles | ~17 cycles |
| L3 hit | ~40 cycles | ~50 cycles |
| RAM | ~60-100 ns | ~60-100 ns |

### P-Core vs E-Core Performance

| Metric | P-Core (Golden Cove) | E-Core (Gracemont) |
|--------|---------------------|-------------------|
| IPC | ~5.5-6.0 | ~4.0-4.5 |
| Single-Thread Perf | 100% | ~70% |
| Power Efficiency | 1x | ~4x |
| L2 Cache | 1.25 MB/core | 2 MB/cluster (4 cores) |

---

## Integration with Silent-Breath-Online

This bare-metal ABI integrates the existing cache coherency system:

```rust
use silent_breath_mmio::{
    CacheState, CacheLine, CoherencyRuntime,
    ShadowRegisterRuntime, FuseMode
};

// Cache coherency for hybrid cores
let mut coherency = CoherencyRuntime::new();

// Shadow registers for hardware configuration
let mut shadow_runtime = ShadowRegisterRuntime::new();
shadow_runtime.register_fuse(0, 0x6000_0000, FuseMode::OTP)?;
```

---

## Testing

```bash
# Run unit tests
cargo test

# Run tests with serial output
cargo test --features serial

# Run specific test
cargo test test_core_type_values
```

---

## Safety Considerations

⚠️ **This is bare-metal firmware running at ring 0 with full hardware access.**

- All MSR access is `unsafe`
- No memory protection or isolation
- Direct hardware manipulation
- Incorrect configuration can brick hardware

**Use in:**
- Development/prototyping environments
- Virtualized testing (QEMU/KVM)
- Hardware with recovery mechanisms

**Do NOT use in production without thorough testing!**

---

## References

- [Intel® 64 and IA-32 Architectures Software Developer's Manual](https://www.intel.com/content/dam/develop/external/us/en/documents/335592-sdm-vol-4.pdf)
- [Intel Core i9-12900K Architecture - TechPowerUp](https://www.techpowerup.com/review/intel-core-i9-12900k-alder-lake-12th-gen/2.html)
- [System V AMD64 ABI - OSDev Wiki](https://wiki.osdev.org/System_V_ABI)
- [Writing an OS in Rust - Phil Opp](https://os.phil-opp.com/minimal-rust-kernel/)
- [rust-osdev/bootloader](https://github.com/rust-osdev/bootloader)
- [x86-64 Calling Conventions - Wikipedia](https://en.wikipedia.org/wiki/X86_calling_conventions)

---

## License

MIT License - Copyright 2026 @LeeMetaX

---

## Contributing

See the main [Silent-Breath-Online repository](../) for contribution guidelines.

---

## Version

**ABI Version**: 0.1.0
**Target CPU**: Intel Core i9-12900K (Alder Lake)
**Rust Version**: nightly (1.94.0+)
